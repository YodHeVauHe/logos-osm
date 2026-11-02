//! Keeps the repository-level `module.json` index consistent with the
//! per-module `metadata.json` manifests that `logos-module-builder` consumes.

use std::path::Path;

use serde_json::Value;

fn read_json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("invalid JSON in {}: {e}", path.display()))
}

#[test]
fn module_json_matches_builder_metadata() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let index = read_json(&root.join("module.json"));

    assert_eq!(index["manifest_format"], "metadata.json");
    let modules = index["modules"].as_array().expect("modules array");
    assert_eq!(
        modules.len(),
        4,
        "osm, osm-app, the consumer example and its UI app"
    );

    for module in modules {
        let manifest_rel = module["manifest"].as_str().expect("manifest path");
        let dir = module["path"].as_str().expect("module path");
        assert_eq!(manifest_rel, format!("{dir}/metadata.json"));

        let metadata = read_json(&root.join(manifest_rel));
        for field in ["name", "type", "version"] {
            assert_eq!(
                module[field], metadata[field],
                "module.json `{field}` differs from {manifest_rel}"
            );
        }
    }
}
