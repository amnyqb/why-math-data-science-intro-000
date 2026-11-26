//! A small Excel (.xlsx) writer with no dependencies: an OOXML workbook of plain cells in a zip file with stored (not
//! compressed) entries. Enough for numbers and text, one bold style, column widths. Kept in the command line crate so
//! that the core has no zip or XML library.

use dynamist_core::workbook::{Cell, Sheet};

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, t) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *t = c;
    }
    let mut crc = 0xFFFF_FFFFu32;
    for b in data {
        crc = table[((crc ^ *b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    !crc
}

/// A zip archive of stored entries.
pub fn zip(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut central: Vec<u8> = Vec::new();
    let le16 = |v: &mut Vec<u8>, x: u16| v.extend_from_slice(&x.to_le_bytes());
    let le32 = |v: &mut Vec<u8>, x: u32| v.extend_from_slice(&x.to_le_bytes());
    for (name, data) in entries {
        let offset = out.len() as u32;
        let crc = crc32(data);
        // local file header
        le32(&mut out, 0x0403_4B50);
        le16(&mut out, 20);
        le16(&mut out, 0x0800); // names are UTF-8
        le16(&mut out, 0); // stored
        le16(&mut out, 0);
        le16(&mut out, 0x21); // 1980-01-01
        le32(&mut out, crc);
        le32(&mut out, data.len() as u32);
        le32(&mut out, data.len() as u32);
        le16(&mut out, name.len() as u16);
        le16(&mut out, 0);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        // central directory entry
        le32(&mut central, 0x0201_4B50);
        le16(&mut central, 20);
        le16(&mut central, 20);
        le16(&mut central, 0x0800);
        le16(&mut central, 0);
        le16(&mut central, 0);
        le16(&mut central, 0x21);
        le32(&mut central, crc);
        le32(&mut central, data.len() as u32);
        le32(&mut central, data.len() as u32);
        le16(&mut central, name.len() as u16);
        le16(&mut central, 0);
        le16(&mut central, 0);
        le16(&mut central, 0);
        le16(&mut central, 0);
        le32(&mut central, 0);
        le32(&mut central, offset);
        central.extend_from_slice(name.as_bytes());
    }
    let start = out.len() as u32;
    out.extend_from_slice(&central);
    le32(&mut out, 0x0605_4B50);
    le16(&mut out, 0);
    le16(&mut out, 0);
    le16(&mut out, entries.len() as u16);
    le16(&mut out, entries.len() as u16);
    le32(&mut out, central.len() as u32);
    le32(&mut out, start);
    le16(&mut out, 0);
    out
}

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            c if (c as u32) < 0x20 && c != '\n' && c != '\t' => {}
            c => o.push(c),
        }
    }
    o
}

/// The column name of a zero-based index: 0 is A, 26 is AA.
fn col(mut i: usize) -> String {
    let mut s = String::new();
    loop {
        s.insert(0, (b'A' + (i % 26) as u8) as char);
        if i < 26 {
            return s;
        }
        i = i / 26 - 1;
    }
}

fn sheet_xml(s: &Sheet) -> String {
    let width = s.rows.iter().map(|r| r.len()).max().unwrap_or(1);
    let mut x = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">");
    if s.heading {
        x.push_str("<sheetViews><sheetView workbookViewId=\"0\"><pane ySplit=\"1\" topLeftCell=\"A2\" activePane=\"bottomLeft\" state=\"frozen\"/></sheetView></sheetViews>");
    }
    x.push_str("<cols>");
    for c in 0..width {
        let longest = s.rows.iter().filter_map(|r| r.get(c)).map(|cell| match cell { Cell::Text(t) => t.chars().count(), Cell::Num(_) => 12, Cell::Empty => 0 }).max().unwrap_or(8);
        let w = if !s.heading && c == 1 { 100.0 } else { (longest as f64 + 2.0).clamp(10.0, 60.0) };
        x.push_str(&format!("<col min=\"{0}\" max=\"{0}\" width=\"{1}\" customWidth=\"1\"/>", c + 1, w));
    }
    x.push_str("</cols><sheetData>");
    for (ri, row) in s.rows.iter().enumerate() {
        x.push_str(&format!("<row r=\"{}\">", ri + 1));
        for (ci, cell) in row.iter().enumerate() {
            let r = format!("{}{}", col(ci), ri + 1);
            let bold = if (s.heading && ri == 0) || (!s.heading && (ci == 0 || ri == 0)) { " s=\"1\"" } else { "" };
            match cell {
                Cell::Text(t) => x.push_str(&format!("<c r=\"{r}\" t=\"inlineStr\"{bold}><is><t xml:space=\"preserve\">{}</t></is></c>", esc(t))),
                Cell::Num(v) => x.push_str(&format!("<c r=\"{r}\"{bold}><v>{v}</v></c>")),
                Cell::Empty => {}
            }
        }
        x.push_str("</row>");
    }
    x.push_str("</sheetData></worksheet>");
    x
}

/// A workbook of these sheets, as the bytes of an .xlsx file.
pub fn workbook(sheets: &[Sheet]) -> Vec<u8> {
    let mut e: Vec<(String, Vec<u8>)> = Vec::new();
    let mut ct = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/><Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/>");
    for i in 0..sheets.len() {
        ct.push_str(&format!("<Override PartName=\"/xl/worksheets/sheet{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>", i + 1));
    }
    ct.push_str("</Types>");
    e.push(("[Content_Types].xml".into(), ct.into_bytes()));
    e.push(("_rels/.rels".into(), b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>".to_vec()));
    let mut wb = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets>");
    let mut rels = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">");
    for (i, s) in sheets.iter().enumerate() {
        wb.push_str(&format!("<sheet name=\"{}\" sheetId=\"{}\" r:id=\"rId{}\"/>", esc(&s.name), i + 1, i + 1));
        rels.push_str(&format!("<Relationship Id=\"rId{0}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet{0}.xml\"/>", i + 1));
        e.push((format!("xl/worksheets/sheet{}.xml", i + 1), sheet_xml(s).into_bytes()));
    }
    wb.push_str("</sheets></workbook>");
    rels.push_str(&format!("<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/></Relationships>", sheets.len() + 1));
    e.push(("xl/workbook.xml".into(), wb.into_bytes()));
    e.push(("xl/_rels/workbook.xml.rels".into(), rels.into_bytes()));
    e.push(("xl/styles.xml".into(), b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><fonts count=\"2\"><font><sz val=\"11\"/><name val=\"Calibri\"/></font><font><b/><sz val=\"11\"/><name val=\"Calibri\"/></font></fonts><fills count=\"2\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill></fills><borders count=\"1\"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs><cellXfs count=\"2\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/><xf numFmtId=\"0\" fontId=\"1\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyFont=\"1\"/></cellXfs></styleSheet>".to_vec()));
    zip(&e)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_zip_and_columns_are_right() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!((col(0), col(25), col(26), col(701), col(702)), ("A".to_string(), "Z".to_string(), "AA".to_string(), "ZZ".to_string(), "AAA".to_string()));
        let z = zip(&[("a.txt".into(), b"hello".to_vec()), ("b/c.txt".into(), b"world!".to_vec())]);
        assert_eq!(&z[..4], b"PK\x03\x04");
        // the end record says two entries
        let end = z.len() - 22;
        assert_eq!(&z[end..end + 4], b"PK\x05\x06");
        assert_eq!(u16::from_le_bytes([z[end + 10], z[end + 11]]), 2);
    }

    #[test]
    fn a_workbook_has_its_sheets_and_escapes_text() {
        let s = Sheet { name: "A & B".into(), rows: vec![vec![Cell::Text("x < y".into()), Cell::Num(1.5), Cell::Empty, Cell::Text("z".into())]], heading: true };
        let w = workbook(&[s]);
        let text = String::from_utf8_lossy(&w);
        assert!(text.contains("A &amp; B") && text.contains("x &lt; y") && text.contains("<v>1.5</v>") && text.contains("r=\"D1\""));
        assert!(text.contains("xl/worksheets/sheet1.xml") && text.contains("[Content_Types].xml"));
    }
}
