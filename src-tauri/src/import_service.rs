//! Shared import validation and persistence for native IPC and authenticated LAN.
use crate::error::AppError;
use crate::import_file::{file_error, MAX_CELL_CHARS, MAX_ROWS};
use chrono::NaiveDate;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

pub(crate) const MAX_REQUEST_BYTES: usize = 8 * 1024 * 1024;
const MAX_SAFE_CENTS: i64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ImportEntity {
    Customers,
    Employees,
    Inventory,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum NumberFormat {
    Brazilian,
    International,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ImportRow {
    pub source_row: u32,
    pub values: BTreeMap<String, String>,
    #[serde(default)]
    pub numeric_fields: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ImportRequest {
    pub entity: ImportEntity,
    pub number_format: NumberFormat,
    pub rows: Vec<ImportRow>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ValidatedRow {
    pub source_row: u32,
    pub values: BTreeMap<String, String>,
    pub errors: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportValidation {
    pub rows: Vec<ValidatedRow>,
    pub valid_count: usize,
    pub invalid_count: usize,
}

fn fields(entity: ImportEntity) -> &'static [&'static str] {
    match entity {
        ImportEntity::Customers => &["name", "phone", "email", "address"],
        ImportEntity::Employees => &["name", "email", "phone", "cpf", "joinDate"],
        ImportEntity::Inventory => &[
            "name",
            "description",
            "type",
            "tracksStock",
            "minQuantity",
            "initialQuantity",
            "costPrice",
            "salePrice",
            "supplierName",
        ],
    }
}

fn check_request(request: &ImportRequest) -> Result<(), AppError> {
    if request.rows.is_empty() || request.rows.len() > MAX_ROWS {
        return Err(file_error("Inclua entre 1 e 5.000 linhas para importar."));
    }
    if serde_json::to_vec(request)
        .map_err(|_| file_error("Não foi possível ler os dados da importação."))?
        .len()
        > MAX_REQUEST_BYTES
    {
        return Err(file_error(
            "Os dados mapeados excedem 8 MiB. Divida a importação.",
        ));
    }
    let mut numbers = HashSet::new();
    for row in &request.rows {
        if row.source_row == 0 || !numbers.insert(row.source_row) {
            return Err(file_error(
                "As linhas de origem devem ter números únicos e maiores que zero.",
            ));
        }
        if row
            .values
            .keys()
            .any(|key| !fields(request.entity).contains(&key.as_str()))
            || row.numeric_fields.len() > fields(request.entity).len()
            || row
                .numeric_fields
                .iter()
                .any(|key| !row.values.contains_key(key))
        {
            return Err(file_error("O mapeamento contém um campo não permitido."));
        }
    }
    Ok(())
}

fn valid_email(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !local.starts_with('.')
        && !local.ends_with('.')
        && !local.contains("..")
        && local
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".!#$%&'*+/=?^_`{|}~-".contains(c))
        && domain.contains('.')
        && domain.split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
        && domain
            .rsplit('.')
            .next()
            .is_some_and(|tld| tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic()))
}

/// Decimal arithmetic with explicit locale; never strips signs or rounds money.
fn money_cents(raw: &str, format: NumberFormat, spreadsheet_number: bool) -> Option<i64> {
    if raw.is_empty() {
        return Some(0);
    }
    let raw = raw
        .strip_prefix("R$")
        .or_else(|| raw.strip_prefix('$'))
        .unwrap_or(raw)
        .trim();
    let (decimal, grouping) = if spreadsheet_number {
        ('.', None)
    } else {
        match format {
            NumberFormat::Brazilian => (',', Some('.')),
            NumberFormat::International => ('.', Some(',')),
        }
    };
    let mut parts = raw.split(decimal);
    let integer = parts.next()?;
    let fraction = parts.next();
    if parts.next().is_some() {
        return None;
    }
    let digits = if let Some(grouping) = grouping.filter(|g| integer.contains(*g)) {
        let groups: Vec<_> = integer.split(grouping).collect();
        if groups[0].is_empty()
            || groups[0].len() > 3
            || groups.iter().skip(1).any(|g| g.len() != 3)
        {
            return None;
        }
        groups.join("")
    } else {
        integer.to_string()
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let cents = match fraction {
        None => 0,
        Some(f) if !f.is_empty() && f.len() <= 2 && f.bytes().all(|b| b.is_ascii_digit()) => {
            f.parse::<i64>().ok()? * if f.len() == 1 { 10 } else { 1 }
        }
        _ => return None,
    };
    let result = digits
        .parse::<i64>()
        .ok()?
        .checked_mul(100)?
        .checked_add(cents)?;
    (result <= MAX_SAFE_CENTS).then_some(result)
}

fn quantity(raw: &str) -> Option<i32> {
    if raw.is_empty() {
        return Some(0);
    }
    if !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse().ok()
}

fn boolean(raw: &str) -> Option<bool> {
    match raw.to_lowercase().as_str() {
        "" | "false" | "não" | "nao" | "no" | "0" => Some(false),
        "true" | "sim" | "yes" | "1" => Some(true),
        _ => None,
    }
}

fn error(errors: &mut BTreeMap<String, String>, field: &str, message: &str) {
    errors
        .entry(field.to_string())
        .or_insert_with(|| message.to_string());
}

pub(crate) fn validate_with_conn(
    conn: &Connection,
    request: &ImportRequest,
) -> Result<ImportValidation, AppError> {
    check_request(request)?;
    let existing_emails = if request.entity == ImportEntity::Employees {
        conn.prepare("SELECT email FROM users")?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<HashSet<_>, _>>()?
    } else {
        HashSet::new()
    };
    let mut email_counts: HashMap<String, usize> = HashMap::new();
    if request.entity == ImportEntity::Employees {
        for row in &request.rows {
            *email_counts
                .entry(
                    row.values
                        .get("email")
                        .map(|s| s.trim())
                        .unwrap_or_default()
                        .to_string(),
                )
                .or_default() += 1;
        }
    }
    let mut rows = Vec::with_capacity(request.rows.len());
    for input in &request.rows {
        let mut values: BTreeMap<String, String> = fields(request.entity)
            .iter()
            .map(|field| {
                (
                    field.to_string(),
                    input
                        .values
                        .get(*field)
                        .map(|s| s.trim())
                        .unwrap_or_default()
                        .to_string(),
                )
            })
            .collect();
        let mut errors = BTreeMap::new();
        for (field, value) in &values {
            if value.contains('\0') {
                error(
                    &mut errors,
                    field,
                    "Célula com fórmula ou erro: informe um valor.",
                );
            }
            if value.chars().count() > MAX_CELL_CHARS {
                error(&mut errors, field, "O valor excede 4.096 caracteres.");
            }
        }
        if values["name"].chars().count() < 2 {
            error(&mut errors, "name", "Nome deve ter ao menos 2 caracteres.");
        }
        match request.entity {
            ImportEntity::Customers => {
                if values["phone"].chars().filter(char::is_ascii_digit).count() < 10 {
                    error(
                        &mut errors,
                        "phone",
                        "Telefone deve ter ao menos 10 dígitos.",
                    );
                }
                if !values["email"].is_empty() && !valid_email(&values["email"]) {
                    error(&mut errors, "email", "E-mail inválido.");
                }
                if !values["address"].is_empty() && values["address"].chars().count() < 5 {
                    error(
                        &mut errors,
                        "address",
                        "Endereço deve ter ao menos 5 caracteres.",
                    );
                }
            }
            ImportEntity::Employees => {
                if !valid_email(&values["email"]) {
                    error(&mut errors, "email", "E-mail inválido.");
                }
                if existing_emails.contains(&values["email"]) {
                    error(
                        &mut errors,
                        "email",
                        "Este e-mail já pertence a um funcionário, inclusive excluído.",
                    );
                }
                if email_counts.get(&values["email"]).copied().unwrap_or(0) > 1 {
                    error(
                        &mut errors,
                        "email",
                        "E-mail repetido nas linhas incluídas.",
                    );
                }
                if !values["joinDate"].is_empty() {
                    let raw = &values["joinDate"];
                    let date = ["%Y-%m-%d", "%d/%m/%Y"]
                        .iter()
                        .find_map(|format| NaiveDate::parse_from_str(raw, format).ok());
                    if let Some(date) = date {
                        values.insert("joinDate".into(), date.to_string());
                    } else {
                        error(
                            &mut errors,
                            "joinDate",
                            "Informe uma data válida em DD/MM/AAAA ou AAAA-MM-DD.",
                        );
                    }
                }
            }
            ImportEntity::Inventory => {
                if values["description"].chars().count() < 3 {
                    error(
                        &mut errors,
                        "description",
                        "Descrição deve ter ao menos 3 caracteres.",
                    );
                }
                let item_type = match values["type"].to_lowercase().as_str() {
                    "part" | "peça" | "peca" => "part",
                    "service" | "serviço" | "servico" => "service",
                    "item" => "item",
                    _ => "",
                };
                if item_type.is_empty() {
                    error(&mut errors, "type", "Selecione peça, serviço ou item.");
                }
                let tracks_stock = match item_type {
                    "part" => true,
                    "service" => false,
                    _ => boolean(&values["tracksStock"]).unwrap_or(false),
                };
                if !values["tracksStock"].is_empty() && boolean(&values["tracksStock"]).is_none() {
                    error(&mut errors, "tracksStock", "Informe sim/não ou true/false.");
                }
                values.insert("type".into(), item_type.into());
                values.insert("tracksStock".into(), tracks_stock.to_string());
                for field in ["minQuantity", "initialQuantity"] {
                    if let Some(value) = quantity(&values[field]) {
                        values.insert(field.into(), value.to_string());
                    } else {
                        error(
                            &mut errors,
                            field,
                            "Informe um inteiro entre 0 e 2.147.483.647.",
                        );
                    }
                }
                if !tracks_stock && quantity(&values["initialQuantity"]).is_some_and(|q| q > 0) {
                    error(
                        &mut errors,
                        "initialQuantity",
                        "Itens sem controle de estoque não podem ter quantidade inicial.",
                    );
                }
                if !tracks_stock && quantity(&values["minQuantity"]).is_some_and(|q| q > 0) {
                    error(
                        &mut errors,
                        "minQuantity",
                        "Itens sem controle de estoque não podem ter quantidade mínima.",
                    );
                }
                for field in ["costPrice", "salePrice"] {
                    if let Some(value) = money_cents(
                        &values[field],
                        request.number_format,
                        input.numeric_fields.iter().any(|f| f == field),
                    ) {
                        values.insert(field.into(), value.to_string());
                    } else {
                        error(&mut errors, field, "Informe um preço não negativo com até duas casas decimais, no formato escolhido.");
                    }
                }
            }
        }
        rows.push(ValidatedRow {
            source_row: input.source_row,
            values,
            errors,
        });
    }
    let invalid_count = rows.iter().filter(|row| !row.errors.is_empty()).count();
    Ok(ImportValidation {
        valid_count: rows.len() - invalid_count,
        invalid_count,
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn request(entity: ImportEntity, rows: Vec<&[(&str, &str)]>) -> ImportRequest {
        ImportRequest {
            entity,
            number_format: NumberFormat::Brazilian,
            rows: rows
                .into_iter()
                .enumerate()
                .map(|(i, fields)| ImportRow {
                    source_row: i as u32 + 2,
                    values: fields
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect(),
                    numeric_fields: vec![],
                })
                .collect(),
        }
    }
    #[test]
    fn import_service_money_is_exact_and_locale_specific() {
        for (raw, format, expected) in [
            ("R$ 1.234,56", NumberFormat::Brazilian, 123456),
            ("1,234.56", NumberFormat::International, 123456),
            ("12,3", NumberFormat::Brazilian, 1230),
            ("", NumberFormat::Brazilian, 0),
            ("90071992547409,91", NumberFormat::Brazilian, MAX_SAFE_CENTS),
        ] {
            assert_eq!(money_cents(raw, format, false), Some(expected));
        }
        for raw in [
            "-1",
            "1,234",
            "1.23",
            "1e2",
            "NaN",
            "1.0000,00",
            "90071992547409,92",
            "99999999999999999999999",
        ] {
            assert_eq!(
                money_cents(raw, NumberFormat::Brazilian, false),
                None,
                "{raw}"
            );
        }
        assert_eq!(
            money_cents("12.34", NumberFormat::Brazilian, true),
            Some(1234)
        );
    }
    #[test]
    fn import_service_clients_required_optional_and_formula_cells() {
        let conn = crate::test_helpers::setup_db();
        let req = request(
            ImportEntity::Customers,
            vec![
                &[("name", " Maria "), ("phone", "(41) 99999-9999")],
                &[
                    ("name", "A"),
                    ("phone", "123"),
                    ("email", "bad"),
                    ("address", "abc"),
                ],
                &[("name", "\0f"), ("phone", "41999999999")],
            ],
        );
        let result = validate_with_conn(&conn, &req).unwrap();
        assert_eq!((result.valid_count, result.invalid_count), (1, 2));
        assert_eq!(result.rows[0].values["name"], "Maria");
        assert_eq!(result.rows[0].values["email"], "");
        assert_eq!(
            result.rows[1]
                .errors
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["address", "email", "name", "phone"]
        );
        assert!(result.rows[2].errors["name"].contains("fórmula"));
    }
    #[test]
    fn import_service_employee_conflicts_include_deleted_and_all_duplicate_rows() {
        let conn = crate::test_helpers::setup_db();
        conn.execute("INSERT INTO users(id,name,email,deleted_at) VALUES ('old','Old','old@example.com','2020-01-01')", []).unwrap();
        let req = request(
            ImportEntity::Employees,
            vec![
                &[("name", "Maria"), ("email", "old@example.com")],
                &[
                    ("name", "João"),
                    ("email", "new@example.com"),
                    ("joinDate", "20/10/2026"),
                ],
                &[("name", "José"), ("email", "new@example.com")],
                &[
                    ("name", "Ana"),
                    ("email", "ana@example.com"),
                    ("joinDate", "31/02/2026"),
                ],
            ],
        );
        let result = validate_with_conn(&conn, &req).unwrap();
        assert_eq!(result.invalid_count, 4);
        assert!(result.rows[0].errors["email"].contains("excluído"));
        assert!(result.rows[1].errors["email"].contains("repetido"));
        assert!(result.rows[2].errors["email"].contains("repetido"));
        assert_eq!(result.rows[1].values["joinDate"], "2026-10-20");
        assert!(result.rows[3].errors.contains_key("joinDate"));
        let mut corrected = req.clone();
        corrected.rows.remove(2);
        corrected.rows.remove(0);
        corrected.rows.pop();
        assert_eq!(
            validate_with_conn(&conn, &corrected).unwrap().valid_count,
            1
        );
    }
    #[test]
    fn import_service_stock_rules_and_bounds() {
        let conn = crate::test_helpers::setup_db();
        let mut req = request(
            ImportEntity::Inventory,
            vec![
                &[
                    ("name", "Tela"),
                    ("description", "Tela OLED"),
                    ("type", "Peça"),
                    ("initialQuantity", "3"),
                    ("costPrice", "12,34"),
                ],
                &[
                    ("name", "Limpeza"),
                    ("description", "Limpeza"),
                    ("type", "serviço"),
                    ("initialQuantity", "2"),
                ],
                &[
                    ("name", "Item"),
                    ("description", "Item"),
                    ("type", "item"),
                    ("minQuantity", "-1"),
                    ("initialQuantity", "2147483648"),
                    ("salePrice", "1,234"),
                ],
            ],
        );
        let result = validate_with_conn(&conn, &req).unwrap();
        assert_eq!(result.valid_count, 1);
        assert_eq!(result.rows[0].values["type"], "part");
        assert_eq!(result.rows[0].values["tracksStock"], "true");
        assert_eq!(result.rows[0].values["costPrice"], "1234");
        assert!(result.rows[1].errors.contains_key("initialQuantity"));
        assert!(result.rows[2].errors.contains_key("minQuantity"));
        assert!(result.rows[2].errors.contains_key("initialQuantity"));
        assert!(result.rows[2].errors.contains_key("salePrice"));
        req.rows[0].source_row = 0;
        assert!(validate_with_conn(&conn, &req).is_err());
        req.rows.clear();
        assert!(validate_with_conn(&conn, &req).is_err());
    }
}
