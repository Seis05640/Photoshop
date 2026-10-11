//! A saved `.pcraft` file reports the name it was saved under (#3312): the session adopts the new
//! name as soon as the save succeeds, so the manifest must not keep the previous one.

use photocraft_automation::Headless;
use photocraft_format::zip::ZipReader;
use serde_json::json;

fn tmp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "pc-manifest-name-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `document.name` as the file stores it.
fn manifest_name(path: &std::path::Path) -> String {
    let bytes = std::fs::read(path).expect("read");
    let archive = ZipReader::new(&bytes).expect("zip");
    let manifest: serde_json::Value = serde_json::from_slice(&archive.read_by_name("manifest.json", usize::MAX).expect("manifest")).expect("manifest json");
    manifest["document"]["name"].as_str().expect("document.name").to_string()
}

/// The issue's scenario: open a document named `FirstTest`, save it under a new name, reopen it.
#[test]
fn save_as_writes_the_new_name_into_the_manifest() {
    let dir = tmp("save-as");
    let first = dir.join("FirstTest.pcraft");
    let renamed = dir.join("_nametest.pcraft");
    let mut headless = Headless::trusted_local();
    headless.handle("doc.new", json!({"width": 8, "height": 8, "background": "white"})).unwrap();
    headless.handle("doc.save", json!({"path": first.to_str().unwrap()})).unwrap();
    headless.handle("doc.open", json!({"path": first.to_str().unwrap()})).unwrap();
    assert_eq!(headless.session.active().unwrap().doc.name, "FirstTest.pcraft");

    headless.handle("doc.save", json!({"path": renamed.to_str().unwrap()})).unwrap();
    // In-session, the save already adopted the new name…
    assert_eq!(headless.session.active().unwrap().doc.name, "_nametest.pcraft");

    // …and the file the session just wrote agrees with it.
    assert_eq!(manifest_name(&renamed), "_nametest.pcraft", "the manifest keeps the previous document name");
    headless.handle("doc.open", json!({"path": renamed.to_str().unwrap()})).unwrap();
    assert_eq!(headless.session.active().unwrap().doc.name, "_nametest.pcraft", "reopening reports the stale name");

    // A later save back in place keeps the name it adopted.
    headless.handle("engine.execute", json!({"command": "image.adjustments.invert", "params": {}})).unwrap();
    headless.handle("doc.save", json!({})).unwrap();
    assert_eq!(manifest_name(&renamed), "_nametest.pcraft", "saving in place drops back to the previous name");
    std::fs::remove_dir_all(dir).unwrap();
}

/// A capability-scoped write produces the same bytes as a local one.
#[test]
fn a_workspace_save_names_the_manifest_after_the_file() {
    use photocraft_automation::{AuthorizedWorkspace, files};

    let dir = tmp("workspace");
    let workspace = AuthorizedWorkspace::new(None, Some(dir.as_path())).expect("workspace");
    let mut headless = Headless::with_workspace(workspace);
    headless.handle("doc.new", json!({"width": 8, "height": 8, "background": "white"})).unwrap();
    let state = headless.session.active().expect("document");
    let doc = state.doc.clone();
    let source_name = doc.name.clone();
    let (bytes, _) = files::save_bytes(&doc, "copy.pcraft", None, &Default::default()).expect("save bytes");

    let archive = ZipReader::new(&bytes).expect("zip");
    let manifest: serde_json::Value = serde_json::from_slice(&archive.read_by_name("manifest.json", usize::MAX).expect("manifest")).expect("manifest json");
    assert_eq!(manifest["document"]["name"].as_str(), Some("copy.pcraft"), "the manifest keeps the previous document name");
    assert_eq!(doc.name, source_name, "a copy must not rename the document it was written from");
    std::fs::remove_dir_all(dir).unwrap();
}
