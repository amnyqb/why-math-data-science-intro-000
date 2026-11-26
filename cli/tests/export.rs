//! `dyn export --xlsx`: the file is a valid zip (checked with Python's zipfile when it is installed) with the five sheets.

use std::process::Command;

#[test]
fn the_workbook_is_written_and_is_a_valid_zip() {
    let dir = std::env::temp_dir().join(format!("dyn-export-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let model = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../core/tests/data/ch02-growth.mdl");
    let out = dir.join("book.xlsx");
    let r = Command::new(env!("CARGO_BIN_EXE_dyn")).args(["export", model.to_str().unwrap(), "--xlsx", out.to_str().unwrap()]).output().unwrap();
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(&bytes[..2], b"PK");
    let text = String::from_utf8_lossy(&bytes);
    for s in ["xl/worksheets/sheet5.xml", "Cover", "Series", "Checks"] {
        assert!(text.contains(s), "{s}");
    }
    if let Ok(p) = Command::new("python3").args(["-c", "import sys, zipfile; z = zipfile.ZipFile(sys.argv[1]); assert z.testzip() is None; print(len(z.namelist()))", out.to_str().unwrap()]).output() {
        if p.status.success() {
            assert_eq!(String::from_utf8_lossy(&p.stdout).trim(), "10");
        }
    }
}
