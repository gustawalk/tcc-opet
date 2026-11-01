//! Local file decoding. Never opens a database or evaluates spreadsheet formulas.
use crate::error::AppError;
use calamine::{DataRef, DataType, Reader, Xlsx};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;

pub(crate) const MAX_ROWS: usize = 5_000;
pub(crate) const MAX_COLUMNS: usize = 100;
pub(crate) const MAX_CELL_CHARS: usize = 4_096;
pub(crate) const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_EXPANDED_BYTES: u64 = 64 * 1024 * 1024;
const INVALID_CELL: &str = "\0Célula com fórmula ou erro: informe um valor.";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceRow {
    pub source_row: u32,
    pub cells: Vec<String>,
    pub numeric_columns: Vec<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParsedImport {
    pub headers: Vec<String>,
    pub rows: Vec<SourceRow>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportInspection {
    pub worksheets: Vec<String>,
    pub delimiter: String,
}

pub(crate) fn file_error(message: impl Into<String>) -> AppError {
    AppError::new("Invalid import file or data.", message)
}

fn read_file(path: &Path) -> Result<Vec<u8>, AppError> {
    let file = File::open(path)
        .map_err(|_| file_error("Não foi possível abrir o arquivo selecionado."))?;
    if file
        .metadata()
        .map_err(|_| file_error("Não foi possível ler o arquivo."))?
        .len()
        > MAX_FILE_BYTES
    {
        return Err(file_error("O arquivo excede o limite de 10 MiB."));
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| file_error("Não foi possível ler o arquivo."))?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(file_error("O arquivo excede o limite de 10 MiB."));
    }
    Ok(bytes)
}

fn extension(path: &Path) -> Result<String, AppError> {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("csv") => Ok("csv".into()),
        Some("xlsx") => Ok("xlsx".into()),
        _ => Err(file_error("Selecione um arquivo CSV ou XLSX.")),
    }
}

fn csv_text(bytes: &[u8]) -> Result<&str, AppError> {
    std::str::from_utf8(bytes)
        .map(|s| s.trim_start_matches('\u{feff}'))
        .map_err(|_| {
            file_error("O CSV deve estar em UTF-8. Exporte novamente com essa codificação.")
        })
}

fn csv_reader(text: &str, delimiter: u8) -> csv::Reader<&[u8]> {
    csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .delimiter(delimiter)
        .from_reader(text.as_bytes())
}

fn validate_csv_quotes(text: &str, delimiter: u8) -> Result<(), AppError> {
    #[derive(Clone, Copy)]
    enum State {
        Start,
        Text,
        Quoted,
        Closed,
    }
    let mut state = State::Start;
    for byte in text.bytes() {
        let separator = byte == delimiter || byte == b'\r' || byte == b'\n';
        state = match (state, byte, separator) {
            (State::Start, b'"', _) => State::Quoted,
            (State::Start | State::Text | State::Closed, _, true) => State::Start,
            (State::Text, b'"', false) => {
                return Err(file_error(
                    "O CSV contém aspas fora de um campo delimitado.",
                ))
            }
            (State::Start | State::Text, _, false) => State::Text,
            (State::Quoted, b'"', _) => State::Closed,
            (State::Quoted, _, _) => State::Quoted,
            (State::Closed, b'"', _) => State::Quoted,
            (State::Closed, _, false) => {
                return Err(file_error(
                    "O CSV contém caracteres após o fechamento de aspas.",
                ))
            }
        };
    }
    if matches!(state, State::Quoted) {
        return Err(file_error("O CSV contém um campo com aspas não fechadas."));
    }
    Ok(())
}

fn suggested_delimiter(text: &str) -> u8 {
    (*b";,\t")
        .into_iter()
        .max_by_key(|delimiter| {
            let widths: Vec<_> = csv_reader(text, *delimiter)
                .records()
                .take(20)
                .filter_map(Result::ok)
                .filter(|row| row.iter().any(|cell| !cell.trim().is_empty()))
                .map(|row| row.len())
                .collect();
            let width = widths.first().copied().unwrap_or(0);
            (width > 1 && widths.iter().all(|w| *w == width), width)
        })
        .unwrap_or(b',')
}

// Check both ZIP declarations and actual expanded bytes before calamine allocates
// worksheet ranges. This also rejects corrupt and encrypted archive entries.
fn workbook(bytes: &[u8]) -> Result<Xlsx<Cursor<&[u8]>>, AppError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| file_error("O arquivo XLSX está corrompido ou não é compatível."))?;
    let mut total = 0u64;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|_| file_error("Não foi possível ler o conteúdo do XLSX."))?;
        let remaining = MAX_EXPANDED_BYTES.saturating_sub(total);
        if entry.size() > remaining {
            return Err(file_error("O conteúdo expandido do XLSX excede 64 MiB."));
        }
        total += std::io::copy(&mut entry.take(remaining + 1), &mut std::io::sink())
            .map_err(|_| file_error("O conteúdo do XLSX está corrompido."))?;
        if total > MAX_EXPANDED_BYTES {
            return Err(file_error("O conteúdo expandido do XLSX excede 64 MiB."));
        }
    }
    Xlsx::new(Cursor::new(bytes))
        .map_err(|_| file_error("Não foi possível abrir o XLSX. Verifique se ele possui senha."))
}

pub(crate) fn inspect(path: &Path) -> Result<ImportInspection, AppError> {
    let ext = extension(path)?;
    let bytes = read_file(path)?;
    if ext == "csv" {
        return Ok(ImportInspection {
            worksheets: vec![],
            delimiter: (suggested_delimiter(csv_text(&bytes)?) as char).to_string(),
        });
    }
    Ok(ImportInspection {
        worksheets: workbook(&bytes)?.sheet_names().to_vec(),
        delimiter: ";".into(),
    })
}

fn append_row(parsed: &mut ParsedImport, row: SourceRow) -> Result<(), AppError> {
    if row.cells.len() > MAX_COLUMNS {
        return Err(file_error("O arquivo excede o limite de 100 colunas."));
    }
    if row
        .cells
        .iter()
        .any(|cell| cell.chars().count() > MAX_CELL_CHARS)
    {
        return Err(file_error(format!(
            "A linha {} contém uma célula com mais de 4.096 caracteres.",
            row.source_row
        )));
    }
    if row.cells.iter().all(|cell| cell.trim().is_empty()) {
        return Ok(());
    }
    if parsed.headers.is_empty() {
        parsed.headers = row.cells;
    } else {
        if parsed.rows.len() == MAX_ROWS {
            return Err(file_error(
                "O arquivo excede o limite de 5.000 linhas de dados.",
            ));
        }
        if row.cells.len() > parsed.headers.len() {
            return Err(file_error(format!(
                "A linha {} tem mais colunas que o cabeçalho.",
                row.source_row
            )));
        }
        let mut row = row;
        row.cells.resize(parsed.headers.len(), String::new());
        parsed.rows.push(row);
    }
    Ok(())
}

pub(crate) fn parse(
    path: &Path,
    worksheet: Option<&str>,
    delimiter: Option<&str>,
) -> Result<ParsedImport, AppError> {
    let ext = extension(path)?;
    let bytes = read_file(path)?;
    let mut parsed = ParsedImport {
        headers: vec![],
        rows: vec![],
        warnings: vec![],
    };
    if ext == "csv" {
        let text = csv_text(&bytes)?;
        let delimiter = match delimiter {
            Some(",") => b',',
            Some(";") => b';',
            Some("\t") => b'\t',
            None => suggested_delimiter(text),
            _ => {
                return Err(file_error(
                    "Selecione vírgula, ponto e vírgula ou tabulação.",
                ))
            }
        };
        validate_csv_quotes(text, delimiter)?;
        for record in csv_reader(text, delimiter).records() {
            let record = record
                .map_err(|_| file_error("O CSV está malformado. Verifique separadores e aspas."))?;
            let source_row = record.position().map(|p| p.line() as u32).unwrap_or(1);
            append_row(
                &mut parsed,
                SourceRow {
                    source_row,
                    cells: record.iter().map(str::to_owned).collect(),
                    numeric_columns: vec![],
                },
            )?;
        }
    } else {
        let mut book = workbook(&bytes)?;
        let sheet = worksheet
            .map(str::to_owned)
            .or_else(|| book.sheet_names().first().cloned())
            .ok_or_else(|| file_error("O XLSX não contém planilhas."))?;
        let mut reader = book
            .worksheet_cells_reader(&sheet)
            .map_err(|_| file_error("Não foi possível ler a planilha selecionada."))?;
        let mut source_rows: BTreeMap<u32, Vec<String>> = BTreeMap::new();
        let mut numeric = false;
        let mut numeric_columns: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
        while let Some(cell) = reader
            .next_cell_with_formula_metadata()
            .map_err(|_| file_error("O conteúdo da planilha está corrompido."))?
        {
            let (row, column) = cell.pos;
            if column as usize >= MAX_COLUMNS {
                return Err(file_error("A planilha excede o limite de 100 colunas."));
            }
            let value = if cell.formula.is_some() || matches!(cell.value, DataRef::Error(_)) {
                INVALID_CELL.into()
            } else if matches!(cell.value, DataRef::DateTime(_) | DataRef::DateTimeIso(_)) {
                cell.value
                    .as_date()
                    .map(|date| date.to_string())
                    .unwrap_or_else(|| INVALID_CELL.into())
            } else {
                if matches!(cell.value, DataRef::Int(_) | DataRef::Float(_)) {
                    numeric = true;
                    numeric_columns
                        .entry(row + 1)
                        .or_default()
                        .push(column as usize);
                }
                calamine::Data::from(cell.value).to_string()
            };
            if value.chars().count() > MAX_CELL_CHARS {
                return Err(file_error(
                    "Uma célula excede o limite de 4.096 caracteres.",
                ));
            }
            if value.trim().is_empty() {
                continue;
            }
            let cells = source_rows.entry(row + 1).or_default();
            cells.resize(cells.len().max(column as usize + 1), String::new());
            cells[column as usize] = value;
            if source_rows.len() > MAX_ROWS + 1 {
                return Err(file_error(
                    "A planilha excede o limite de 5.000 linhas de dados.",
                ));
            }
        }
        // Headers may end with empty cells; retain the width needed by data rows.
        let width = source_rows.values().map(Vec::len).max().unwrap_or(0);
        for (source_row, mut cells) in source_rows {
            cells.resize(width, String::new());
            append_row(
                &mut parsed,
                SourceRow {
                    source_row,
                    cells,
                    numeric_columns: numeric_columns.remove(&source_row).unwrap_or_default(),
                },
            )?;
        }
        if numeric {
            parsed.warnings.push("A planilha contém células numéricas. Confira telefones e CPF: zeros iniciais perdidos no arquivo não podem ser recuperados.".into());
        }
    }
    if parsed.headers.is_empty() || parsed.rows.is_empty() {
        return Err(file_error(
            "O arquivo deve conter um cabeçalho e ao menos uma linha de dados.",
        ));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn csv_fixture(bytes: &[u8]) -> tempfile::NamedTempFile {
        let mut file = tempfile::Builder::new().suffix(".csv").tempfile().unwrap();
        std::io::Write::write_all(&mut file, bytes).unwrap();
        file
    }
    #[test]
    fn import_file_csv_bom_quotes_blanks_and_source_lines() {
        let file = csv_fixture("\u{feff}\nName;Phone;Name\n\"Maria; Silva\";041999999999;\"Rua\nA\"\n;;\nJoão;41988888888\n".as_bytes());
        assert_eq!(inspect(file.path()).unwrap().delimiter, ";");
        let parsed = parse(file.path(), None, Some(";")).unwrap();
        assert_eq!(parsed.headers, ["Name", "Phone", "Name"]);
        assert_eq!(parsed.rows.len(), 2);
        assert_eq!(
            parsed.rows[0].cells,
            ["Maria; Silva", "041999999999", "Rua\nA"]
        );
        assert_eq!(parsed.rows[0].source_row, 3);
        assert_eq!(parsed.rows[1].source_row, 6);
        assert_eq!(parsed.rows[1].cells[2], "");
    }
    #[test]
    fn import_file_csv_delimiters_and_encoding() {
        for (content, delimiter) in [
            ("name,phone\nMaria,41999999999", ","),
            ("name\tphone\nMaria\t41999999999", "\t"),
        ] {
            let file = csv_fixture(content.as_bytes());
            assert_eq!(inspect(file.path()).unwrap().delimiter, delimiter);
            assert_eq!(
                parse(file.path(), None, Some(delimiter)).unwrap().rows[0].cells[0],
                "Maria"
            );
        }
        assert!(parse(csv_fixture(&[0xff]).path(), None, None)
            .unwrap_err()
            .pt
            .contains("UTF-8"));
        assert!(parse(csv_fixture(b"name\n").path(), None, None)
            .unwrap_err()
            .pt
            .contains("linha de dados"));
        assert!(parse(csv_fixture(b"name\na,b").path(), None, Some(","))
            .unwrap_err()
            .pt
            .contains("mais colunas"));
    }
    #[test]
    fn import_file_limits_allow_5000_and_reject_overflow() {
        let content = format!("name\n{}", "Maria\n".repeat(MAX_ROWS));
        assert_eq!(
            parse(csv_fixture(content.as_bytes()).path(), None, None)
                .unwrap()
                .rows
                .len(),
            5000
        );
        assert!(parse(
            csv_fixture(format!("{content}extra\n").as_bytes()).path(),
            None,
            None
        )
        .is_err());
        assert!(parse(
            csv_fixture(format!("name\n{}", "x".repeat(MAX_CELL_CHARS + 1)).as_bytes()).path(),
            None,
            None
        )
        .is_err());
        assert!(parse(
            csv_fixture(format!("{}\na", vec!["h"; MAX_COLUMNS + 1].join(";")).as_bytes()).path(),
            None,
            Some(";")
        )
        .is_err());
        let file = csv_fixture(b"");
        file.as_file().set_len(MAX_FILE_BYTES + 1).unwrap();
        assert!(inspect(file.path()).unwrap_err().pt.contains("10 MiB"));
    }
    fn xlsx_fixture(xml: &str) -> tempfile::NamedTempFile {
        use std::io::Write;
        let file = tempfile::Builder::new().suffix(".xlsx").tempfile().unwrap();
        let mut zip = zip::ZipWriter::new(file.reopen().unwrap());
        let entries = [
            (
                "_rels/.rels",
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "[Content_Types].xml",
                r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/></Types>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Clientes" sheetId="1" r:id="rId1"/><sheet name="Outro" sheetId="2" r:id="rId2"/></sheets></workbook>"#,
            ),
            (
                "xl/_rels/workbook.xml.rels",
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet2.xml"/></Relationships>"#,
            ),
            (
                "xl/styles.xml",
                r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><cellXfs count="2"><xf numFmtId="0"/><xf numFmtId="14"/></cellXfs></styleSheet>"#,
            ),
            ("xl/worksheets/sheet1.xml", xml),
            (
                "xl/worksheets/sheet2.xml",
                r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Nome</t></is></c></row><row r="2"><c r="A2" t="inlineStr"><is><t>Outro cliente</t></is></c></row></sheetData></worksheet>"#,
            ),
        ];
        for (name, content) in entries {
            zip.start_file(name, zip::write::FileOptions::default())
                .unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        file
    }
    #[test]
    fn import_file_xlsx_sheets_dates_formulas_identifiers_and_sparse_coordinates() {
        let file = xlsx_fixture(
            r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><dimension ref="A1:XFD1048576"/><sheetData><row r="2"><c r="A2" t="inlineStr"><is><t>Nome</t></is></c><c r="B2" t="inlineStr"><is><t>Data</t></is></c><c r="C2" t="inlineStr"><is><t>CPF</t></is></c></row><row r="4"><c r="A4"><f>1+1</f><v>2</v></c><c r="B4" s="1"><v>45292</v></c><c r="C4" t="inlineStr"><is><t>00123456789</t></is></c></row></sheetData></worksheet>"#,
        );
        assert_eq!(
            inspect(file.path()).unwrap().worksheets,
            ["Clientes", "Outro"]
        );
        let parsed = parse(file.path(), Some("Clientes"), None).unwrap();
        assert_eq!(parsed.rows[0].source_row, 4);
        assert!(parsed.rows[0].cells[0].starts_with('\0'));
        assert_eq!(parsed.rows[0].cells[1], "2024-01-01");
        assert_eq!(parsed.rows[0].cells[2], "00123456789");
        assert_eq!(
            parse(file.path(), Some("Outro"), None).unwrap().rows[0].cells[0],
            "Outro cliente"
        );
        assert!(parse(file.path(), Some("Ausente"), None).is_err());
    }
    #[test]
    fn import_file_xlsx_rejects_corruption_and_expansion_and_column_overflow() {
        let file = xlsx_fixture(
            r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="CW1" t="inlineStr"><is><t>Nome</t></is></c></row></sheetData></worksheet>"#,
        );
        assert!(parse(file.path(), None, None)
            .unwrap_err()
            .pt
            .contains("100 colunas"));
        std::fs::write(file.path(), b"invalid zip").unwrap();
        assert!(inspect(file.path()).is_err());
        let mut zip = zip::ZipWriter::new(file.reopen().unwrap());
        zip.start_file(
            "large.xml",
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated),
        )
        .unwrap();
        let chunk = [b' '; 1024 * 1024];
        for _ in 0..65 {
            std::io::Write::write_all(&mut zip, &chunk).unwrap();
        }
        zip.finish().unwrap();
        assert!(inspect(file.path()).unwrap_err().pt.contains("64 MiB"));
    }
    #[test]
    fn import_file_rejects_unclosed_quotes_and_characters_after_quote() {
        for content in [
            "name,phone\n\"Maria,41999999999",
            "name,phone\n\"Maria\"wrong,41999999999",
            "name,phone\nMa\"ria,41999999999",
        ] {
            assert!(
                parse(csv_fixture(content.as_bytes()).path(), None, Some(","))
                    .unwrap_err()
                    .pt
                    .contains("aspas")
            );
        }
        let file = csv_fixture(b"name,phone\n\"Maria \"\"Silva\"\"\",41999999999");
        assert_eq!(
            parse(file.path(), None, Some(",")).unwrap().rows[0].cells[0],
            "Maria \"Silva\""
        );
    }
    #[test]
    fn import_file_xlsx_date_only_and_numeric_provenance() {
        let file = xlsx_fixture(
            r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Nome</t></is></c><c r="B1" t="inlineStr"><is><t>Data</t></is></c><c r="C1" t="inlineStr"><is><t>CPF</t></is></c><c r="D1" t="inlineStr"><is><t>Custo</t></is></c></row><row r="2"><c r="A2" t="inlineStr"><is><t>João</t></is></c><c r="B2" t="d"><v>2026-10-20</v></c><c r="C2"><v>1234567890</v></c><c r="D2"><v>12.34</v></c></row></sheetData></worksheet>"#,
        );
        let parsed = parse(file.path(), Some("Clientes"), None).unwrap();
        assert_eq!(
            parsed.rows[0].cells,
            ["João", "2026-10-20", "1234567890", "12.34"]
        );
        assert_eq!(parsed.rows[0].numeric_columns, [2, 3]);
        assert_eq!(parsed.warnings.len(), 1);
        assert!(parsed.warnings[0].contains("zeros iniciais"));
    }
}
