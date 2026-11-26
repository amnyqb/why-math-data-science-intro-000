//! The generated Model-Doctor page is the one on disk (set DYN_WRITE_WIKI=1 to rewrite it).
#[test]
fn the_wiki_page_is_the_generated_one() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../wiki/Model-Doctor.md");
    let want = dynamist_core::doctor::catalogue_markdown();
    if std::env::var("DYN_WRITE_WIKI").is_ok() {
        std::fs::write(&path, &want).unwrap();
        return;
    }
    let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(on_disk, want, "run the test with DYN_WRITE_WIKI=1 to refresh wiki/Model-Doctor.md");
}
