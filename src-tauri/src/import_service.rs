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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportResult {
    pub created_count: usize,
    pub ids: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommitOutcome {
    pub result: Option<ImportResult>,
    pub validation: Option<ImportValidation>,
}

pub(crate) fn commit_with_conn(
    conn: &mut Connection,
    batch_id: &str,
    request: &ImportRequest,
) -> Result<CommitOutcome, AppError> {
    use crate::models::{customer::Customer, inventory_item::InventoryItem, user::User};
    use crate::repositories::{
        customer_repo::CustomerRepository, inventory_repo::InventoryRepository,
        user_repo::UserRepository,
    };
    use rusqlite::{params, OptionalExtension, TransactionBehavior};
    if uuid::Uuid::parse_str(batch_id).is_err() {
        return Err(file_error("O identificador da importação é inválido."));
    }
    check_request(request)?;
    let bytes = serde_json::to_vec(request)
        .map_err(|_| file_error("Não foi possível preparar a importação."))?;
    let hash = blake3::hash(&bytes).to_hex().to_string();
    let transaction = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let receipt: Option<(String, String)> = transaction
        .query_row(
            "SELECT payload_hash, result_json FROM import_batches WHERE batch_id = ?1",
            [batch_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    if let Some((old_hash, result)) = receipt {
        if old_hash != hash {
            return Err(file_error(
                "Este identificador já foi usado para outros dados. Inicie outra importação.",
            ));
        }
        let result = serde_json::from_str(&result)
            .map_err(|_| file_error("Não foi possível recuperar o resultado da importação."))?;
        return Ok(CommitOutcome {
            result: Some(result),
            validation: None,
        });
    }
    let validation = validate_with_conn(&transaction, request)?;
    if validation.invalid_count > 0 {
        return Ok(CommitOutcome {
            result: None,
            validation: Some(validation),
        });
    }
    let mut ids = Vec::with_capacity(validation.rows.len());
    for row in validation.rows {
        let values = row.values;
        let id = match request.entity {
            ImportEntity::Customers => {
                let customer = Customer::new(
                    values["name"].clone(),
                    values["phone"].clone(),
                    values["email"].clone(),
                    values["address"].clone(),
                );
                CustomerRepository::create_with_conn(&transaction, &customer)?;
                customer.id
            }
            ImportEntity::Employees => {
                let mut user = User::new(values["name"].clone(), values["email"].clone());
                user.phone = (!values["phone"].is_empty()).then(|| values["phone"].clone());
                user.cpf = (!values["cpf"].is_empty()).then(|| values["cpf"].clone());
                user.join_date =
                    (!values["joinDate"].is_empty()).then(|| values["joinDate"].clone());
                UserRepository::create_with_conn(&transaction, &user)?;
                user.id
            }
            ImportEntity::Inventory => {
                // Values are normalized and bounded by validate_with_conn above.
                let mut item = InventoryItem::new(
                    values["name"].clone(),
                    values["description"].clone(),
                    values["type"].clone(),
                    values["minQuantity"]
                        .parse()
                        .map_err(|_| file_error("Quantidade inválida."))?,
                    values["initialQuantity"]
                        .parse()
                        .map_err(|_| file_error("Quantidade inválida."))?,
                    values["costPrice"]
                        .parse()
                        .map_err(|_| file_error("Preço inválido."))?,
                    values["salePrice"]
                        .parse()
                        .map_err(|_| file_error("Preço inválido."))?,
                );
                item.tracks_stock = values["tracksStock"] == "true";
                item.supplier_name =
                    (!values["supplierName"].is_empty()).then(|| values["supplierName"].clone());
                InventoryRepository::create_with_conn(&transaction, &item)?;
                if item.tracks_stock && item.current_quantity > 0 {
                    transaction.execute(
                        "INSERT INTO inventory_movements (id, inventory_item_id, type, quantity, reference_os_id, reason, unit_cost_cents, created_at) VALUES (?1, ?2, 'entrada', ?3, NULL, 'bulk_import', ?4, ?5)",
                        params![uuid::Uuid::new_v4().to_string(), item.id, item.current_quantity, item.cost_price, item.created_at]
                    )?;
                }
                item.id
            }
        };
        ids.push(id);
    }
    let result = ImportResult {
        created_count: ids.len(),
        ids,
    };
    let result_json = serde_json::to_string(&result)
        .map_err(|_| file_error("Não foi possível registrar o resultado da importação."))?;
    transaction.execute(
        "INSERT INTO import_batches(batch_id, payload_hash, result_json) VALUES (?1, ?2, ?3)",
        params![batch_id, hash, result_json],
    )?;
    transaction.commit()?;
    Ok(CommitOutcome {
        result: Some(result),
        validation: None,
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
    #[test]
    fn import_service_commit_creates_duplicates_replays_and_rejects_changed_payload() {
        let mut conn = crate::test_helpers::setup_db();
        let mut req = request(
            ImportEntity::Customers,
            vec![
                &[("name", "Maria"), ("phone", "41999999999")],
                &[("name", "Maria"), ("phone", "41999999999")],
            ],
        );
        let batch = uuid::Uuid::new_v4().to_string();
        let result = commit_with_conn(&mut conn, &batch, &req)
            .unwrap()
            .result
            .unwrap();
        assert_eq!(result.created_count, 2);
        assert_ne!(result.ids[0], result.ids[1]);
        let replay = commit_with_conn(&mut conn, &batch, &req)
            .unwrap()
            .result
            .unwrap();
        assert_eq!(replay, result);
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM customers", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            2
        );
        req.rows[0].values.insert("name".into(), "Ana".into());
        assert!(commit_with_conn(&mut conn, &batch, &req)
            .unwrap_err()
            .pt
            .contains("outros dados"));
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM import_batches", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    #[test]
    fn import_service_commit_rolls_back_insert_failure_and_revalidates_conflicts() {
        let mut conn = crate::test_helpers::setup_db();
        conn.execute_batch("CREATE TRIGGER import_fault BEFORE INSERT ON customers WHEN NEW.name = 'Falha' BEGIN SELECT RAISE(ABORT, 'injected import fault'); END;").unwrap();
        let req = request(
            ImportEntity::Customers,
            vec![
                &[("name", "Maria"), ("phone", "41999999999")],
                &[("name", "Falha"), ("phone", "41999999999")],
            ],
        );
        assert!(commit_with_conn(&mut conn, &uuid::Uuid::new_v4().to_string(), &req).is_err());
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM customers", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM import_batches", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        let req = request(
            ImportEntity::Employees,
            vec![&[("name", "Maria"), ("email", "maria@example.com")]],
        );
        assert_eq!(validate_with_conn(&conn, &req).unwrap().valid_count, 1);
        conn.execute(
            "INSERT INTO users(id,name,email) VALUES ('other','Other','maria@example.com')",
            [],
        )
        .unwrap();
        let result = commit_with_conn(&mut conn, &uuid::Uuid::new_v4().to_string(), &req).unwrap();
        assert_eq!(result.validation.unwrap().invalid_count, 1);
        assert!(result.result.is_none());
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM import_batches", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn import_service_commit_stock_has_exact_costs_and_audit_movement() {
        let mut conn = crate::test_helpers::setup_db();
        let req = request(
            ImportEntity::Inventory,
            vec![
                &[
                    ("name", "Tela"),
                    ("description", "Tela OLED"),
                    ("type", "part"),
                    ("initialQuantity", "3"),
                    ("costPrice", "12,34"),
                    ("salePrice", "20,50"),
                    ("supplierName", "Fornecedor"),
                ],
                &[
                    ("name", "Limpeza"),
                    ("description", "Limpeza"),
                    ("type", "service"),
                ],
            ],
        );
        let batch = uuid::Uuid::new_v4().to_string();
        let result = commit_with_conn(&mut conn, &batch, &req)
            .unwrap()
            .result
            .unwrap();
        let item = crate::repositories::inventory_repo::InventoryRepository::get_by_id_with_conn(
            &conn,
            &result.ids[0],
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            (
                item.current_quantity,
                item.cost_price,
                item.average_cost,
                item.sale_price
            ),
            (3, 1234, 1234, 2050)
        );
        assert_eq!(item.supplier_name.as_deref(), Some("Fornecedor"));
        let movements =
            crate::repositories::inventory_repo::InventoryRepository::get_movements_with_conn(
                &conn, &item.id,
            )
            .unwrap();
        assert_eq!(movements.len(), 1);
        assert_eq!(movements[0].r#type, "entrada");
        assert_eq!(movements[0].quantity, 3);
        assert_eq!(movements[0].reason, "bulk_import");
        assert_eq!(movements[0].unit_cost, Some(1234));
        assert!(movements[0].reference_os_id.is_none());
        commit_with_conn(&mut conn, &batch, &req).unwrap();
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM inventory_movements", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    #[test]
    fn import_service_commit_employee_replay_precedes_own_uniqueness_conflict() {
        let mut conn = crate::test_helpers::setup_db();
        let req = request(
            ImportEntity::Employees,
            vec![&[
                ("name", "Maria"),
                ("email", "maria@example.com"),
                ("joinDate", "20/10/2026"),
                ("cpf", "00123456789"),
            ]],
        );
        let batch = uuid::Uuid::new_v4().to_string();
        let result = commit_with_conn(&mut conn, &batch, &req)
            .unwrap()
            .result
            .unwrap();
        assert_eq!(
            commit_with_conn(&mut conn, &batch, &req)
                .unwrap()
                .result
                .unwrap(),
            result
        );
        let user = crate::repositories::user_repo::UserRepository::get_by_id_with_conn(
            &conn,
            &result.ids[0],
        )
        .unwrap()
        .unwrap();
        assert_eq!(user.join_date.as_deref(), Some("2026-10-20"));
        assert_eq!(user.cpf.as_deref(), Some("00123456789"));
        assert!(user.phone.is_none());
    }
    #[test]
    fn import_service_commit_5000_rows_and_receipt_are_complete() {
        let mut conn = crate::test_helpers::setup_db();
        let mut req = request(
            ImportEntity::Customers,
            vec![&[("name", "Cliente"), ("phone", "41999999999")]],
        );
        req.rows = (0..5000)
            .map(|index| {
                let mut row = req.rows[0].clone();
                row.source_row = index + 2;
                row
            })
            .collect();
        let started = std::time::Instant::now();
        let result = commit_with_conn(&mut conn, &uuid::Uuid::new_v4().to_string(), &req)
            .unwrap()
            .result
            .unwrap();
        assert_eq!(result.created_count, 5000);
        assert_eq!(result.ids.iter().collect::<HashSet<_>>().len(), 5000);
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM customers", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            5000
        );
        eprintln!("5000-row import completed in {:?}", started.elapsed());
    }
    #[test]
    fn import_service_concurrent_identical_batch_creates_once() {
        let storage = crate::test_helpers::TestStorage::new(false);
        let batch = uuid::Uuid::new_v4().to_string();
        let req = request(
            ImportEntity::Customers,
            vec![&[("name", "Maria"), ("phone", "41999999999")]],
        );
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let threads: Vec<_> = (0..2)
            .map(|_| {
                let mut conn = storage.open();
                let batch = batch.clone();
                let req = req.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    commit_with_conn(&mut conn, &batch, &req)
                        .unwrap()
                        .result
                        .unwrap()
                })
            })
            .collect();
        let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        assert_eq!(results[0], results[1]);
        assert_eq!(
            storage
                .open()
                .query_row("SELECT COUNT(*) FROM customers", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
}
