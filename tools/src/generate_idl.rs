use serde_json::{json, Value};

fn main() {
    let region_entry = json!({
        "region": "string", "parent": "option<string>", "level": "u8", "cid": "string",
        "source_url": "string", "checksum": "string", "version": "string", "hosted": "bool",
        "timestamp": "i64", "registered_by": "[u8; 32]", "record_version": "u8"
    });
    let account = json!({
        "name": "registry",
        "address": "AccountId::for_public_pda(program_account_id, PdaSeed([\"registry\", 0x00 x 24]))",
        "owner": "deployed registry program account",
        "kind": "public shard; writable",
        "signer": false
    });
    let result: Value = json!({
        "name": "osm_registry",
        "version": "0.3.1",
        "runtime": "LEZ plan/apply",
        "encoding": "borsh",
        "instruction_enum_discriminants": {"init_registry": 0, "register_region": 1, "register_batch": 2},
        "accounts": [account],
        "types": [{"name": "RegionEntry", "fields": region_entry}],
        "instructions": [
            {"name": "init_registry", "args": [], "accounts": ["registry"], "permissionless": true, "idempotent": true},
            {"name": "register_region", "args": ["RegionEntry"], "accounts": ["registry"]},
            {"name": "register_batch", "args": [{"entries": "Vec<RegionEntry>"}], "accounts": ["registry"], "max_entries": 72}
        ],
        "errors": [
            {"code": 1, "name": "E_REGION_EMPTY"}, {"code": 2, "name": "E_BAD_LEVEL"},
            {"code": 3, "name": "E_BAD_CHECKSUM"}, {"code": 4, "name": "E_BAD_TIMESTAMP"},
            {"code": 5, "name": "E_BAD_VERSION"}, {"code": 6, "name": "E_REGISTRY_FULL"},
            {"code": 7, "name": "E_DESERIALIZE"}, {"code": 8, "name": "E_BATCH_TOO_BIG"},
            {"code": 9, "name": "E_BAD_PARENT"}, {"code": 10, "name": "E_BAD_REGION_ID"},
            {"code": 11, "name": "E_BAD_CID"}, {"code": 12, "name": "E_BAD_SOURCE_URL"}
        ],
        "read": "fetch the registry account and decode account.data.shard(program_account_id) as osm_registry_core::Registry"
    });
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
