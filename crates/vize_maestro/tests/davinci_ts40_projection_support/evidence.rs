//! Authenticate the complete packets behind the reviewed #10933 transition.

use super::{capture_fixture, load_matrix, normalize::sha256};

const DIRECTORY: &str =
    "tests/davinci_ts40_projection_support/fixtures/public-instance-options-api-slots";
const MANIFEST: &str = include_str!("fixtures/public-instance-options-api-slots/manifest.json");

#[test]
fn public_instance_slot_packets_remain_authenticated() {
    let manifest: serde_json::Value =
        serde_json::from_str(MANIFEST).expect("fixed packet manifest");
    assert_eq!(manifest["version"], 1);
    let files = manifest["files"]
        .as_array()
        .expect("complete packet census");
    assert_eq!(files.len(), 194);
    let changed: Vec<_> = files
        .iter()
        .filter(|file| file["before"] != file["after"])
        .map(|file| file["path"].as_str().expect("packet relative path"))
        .collect();
    assert_eq!(
        changed,
        [
            "options-api-slots/maestro/authored-hits.txt",
            "options-api-slots/maestro/mappings.txt",
            "options-api-slots/maestro/text.txt",
            "options-api-slots/maestro-document-0/text.ts",
        ]
    );
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(DIRECTORY);
    let retained = manifest["retainedFiles"]
        .as_array()
        .expect("retained packets");
    assert_eq!(retained.len(), 14);
    for entry in retained {
        let path = entry["path"].as_str().expect("retained relative path");
        let text = std::fs::read_to_string(root.join(path)).expect("complete retained packet");
        assert_eq!(Some(text.len() as u64), entry["bytes"].as_u64(), "{path}");
        assert_eq!(
            Some(sha256(&text).as_str()),
            entry["sha256"].as_str(),
            "{path}"
        );
    }
    let matrix = load_matrix();
    let fixture = matrix
        .fixtures
        .iter()
        .find(|fixture| fixture.id == "options-api-slots")
        .expect("reviewed Options API slot fixture");
    assert_eq!(
        capture_fixture(fixture).render().as_str(),
        include_str!("fixtures/public-instance-options-api-slots/after/projection-record.txt")
    );
}
