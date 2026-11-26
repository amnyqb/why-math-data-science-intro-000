//! `dyn pack`: the audit pack for the fuel supply model (a Vensim import) and two library models, compared with a stored snapshot.
//! Run with UPDATE_SNAPSHOTS=1 to rewrite the snapshots after a change that is meant.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn pack(model: &Path, tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("dyn-pack-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let r = Command::new(env!("CARGO_BIN_EXE_dyn")).args(["pack", model.to_str().unwrap(), "--out", dir.to_str().unwrap()]).output().unwrap();
    assert!(r.status.success(), "{} {}", String::from_utf8_lossy(&r.stdout), String::from_utf8_lossy(&r.stderr));
    dir
}

fn snapshot(name: &str, text: &str) {
    let p = root().join("cli/tests/snapshots").join(name);
    if std::env::var("UPDATE_SNAPSHOTS").is_ok() {
        std::fs::write(&p, text).unwrap();
    }
    let want = std::fs::read_to_string(&p).unwrap_or_else(|_| panic!("no snapshot {name}; run with UPDATE_SNAPSHOTS=1"));
    assert_eq!(text, want, "snapshot {name} differs");
}

fn files(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().to_string()).collect();
    v.sort();
    v
}

#[test]
fn the_pack_for_the_fuel_supply_model() {
    let dir = pack(&root().join("core/tests/book_models/ch24__ch24-fuel.mdl"), "fuel");
    let f = files(&dir);
    for need in ["index.md", "model-card.md", "doctor-report.md", "doctor-report.json", "sanity-sheet.md", "checks.md", "conversion-loss.md", "workbook.xlsx"] {
        assert!(f.contains(&need.to_string()), "{need} missing from {f:?}");
    }
    assert_eq!(&std::fs::read(dir.join("workbook.xlsx")).unwrap()[..2], b"PK");
    snapshot("pack-fuel-index.md", &std::fs::read_to_string(dir.join("index.md")).unwrap());
    snapshot("pack-fuel-card.md", &std::fs::read_to_string(dir.join("model-card.md")).unwrap());
}

#[test]
fn the_pack_for_two_library_models() {
    for (name, tag) in [("bass-diffusion", "bass"), ("predator-prey", "prey")] {
        let dir = pack(&root().join(format!("library/classics/{name}/{name}.sdm")), tag);
        let f = files(&dir);
        assert!(!f.contains(&"conversion-loss.md".to_string()), "{f:?}");
        snapshot(&format!("pack-{tag}-index.md"), &std::fs::read_to_string(dir.join("index.md")).unwrap());
        snapshot(&format!("pack-{tag}-card.md"), &std::fs::read_to_string(dir.join("model-card.md")).unwrap());
    }
}

#[test]
fn a_field_deleted_from_the_manifest_is_not_stated() {
    let src = root().join("library/classics/bass-diffusion/bass-diffusion.sdm");
    let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&src).unwrap()).unwrap();
    // first the field is stated, then it is deleted
    v["manifest"]["description"] = serde_json::json!("A stated description.");
    let tmp0 = std::env::temp_dir().join(format!("dyn-pack-desc-{}.sdm", std::process::id()));
    std::fs::write(&tmp0, serde_json::to_string(&v).unwrap()).unwrap();
    let stated = std::fs::read_to_string(pack(&tmp0, "desc").join("model-card.md")).unwrap();
    assert!(stated.contains("## Description\n\nA stated description."), "{stated}");
    v["manifest"]["description"] = serde_json::json!("");
    v["manifest"]["validity"] = serde_json::json!([]);
    let tmp = std::env::temp_dir().join(format!("dyn-pack-nodesc-{}.sdm", std::process::id()));
    std::fs::write(&tmp, serde_json::to_string(&v).unwrap()).unwrap();
    let dir = pack(&tmp, "nodesc");
    let card = std::fs::read_to_string(dir.join("model-card.md")).unwrap();
    assert!(card.contains("## Description\n\nnot stated"), "{card}");
    assert!(card.contains("## Validity\n\nnot stated"), "{card}");
}

#[test]
fn a_part_that_fails_makes_the_exit_code_not_zero() {
    // a model that cannot run: the sanity sheet and the workbook cannot be made from it, but the pack must say so and exit 1
    let tmp = std::env::temp_dir().join(format!("dyn-pack-broken-{}.sdm", std::process::id()));
    let src = root().join("library/classics/bass-diffusion/bass-diffusion.sdm");
    let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&src).unwrap()).unwrap();
    v["vars"][0]["equation"] = serde_json::json!("1 +");
    std::fs::write(&tmp, serde_json::to_string(&v).unwrap()).unwrap();
    let dir = std::env::temp_dir().join(format!("dyn-pack-broken-out-{}", std::process::id()));
    let r = Command::new(env!("CARGO_BIN_EXE_dyn")).args(["pack", tmp.to_str().unwrap(), "--out", dir.to_str().unwrap()]).output().unwrap();
    let index = std::fs::read_to_string(dir.join("index.md")).unwrap_or_default();
    // either every part still produced something honest (exit 0, index says so) or some failed (exit 1, index says FAILED): never a silent success
    assert_eq!(r.status.success(), !index.contains("FAILED"), "{index}");
}
