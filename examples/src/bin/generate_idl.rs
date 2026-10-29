/// Generate IDL JSON for the osm-registry program.
///
/// Usage:
///   cargo run --bin generate_idl > osm-registry.idl.json
spel_framework::generate_idl!("../../../methods/guest/src/bin/osm_registry.rs");
