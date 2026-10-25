//! Shared on-chain/off-chain types for the LP-0018 OSM registry.
//!
//! This crate is compiled both for the host (SDK, CLI) and inside the RISC0
//! guest (SPEL program), so it must stay `no_std`-friendly in spirit: no
//! IO, no threads, no allocations beyond what borsh needs. The guest uses
//! borsh only; serde types exist for the host side (JSON APIs, FFI).

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Maximum regions accepted in a single `register_batch` call. The frozen
/// predefined set has exactly 72 regions, so one batch can cover it all.
pub const MAX_BATCH: usize = 72;

/// Account data ceiling imposed by the LEZ runtime. 72 entries occupy well
/// under this, but the guest still guards writes with this constant.
pub const MAX_ACCOUNT_BYTES: usize = 100 * 1024;

pub const LEVEL_COUNTRY: u8 = 0;
pub const LEVEL_SUBREGION: u8 = 1;

/// One on-chain record per predefined region, keyed by `region` (the
/// Geofabrik path as frozen in the prize document, e.g. `us/california`,
/// `india/central-zone`, `germany`).
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub struct RegionEntry {
    /// Geofabrik path (unique key, also stored for SDK convenience).
    pub region: String,
    /// Decomposed parent (`us` | `india` | `china` | `russia`); `None` for
    /// country-level entries.
    pub parent: Option<String>,
    /// `LEVEL_COUNTRY` or `LEVEL_SUBREGION`.
    pub level: u8,
    /// Content identifier of the stored PBF snapshot in Logos Storage.
    pub cid: String,
    /// Canonical Geofabrik download URL (`*-latest.osm.pbf`).
    pub source_url: String,
    /// Geofabrik's published MD5 of the snapshot bytes (32 lowercase hex).
    pub checksum: String,
    /// Snapshot version: the dated Geofabrik filename suffix, `YYMMDD`.
    pub version: String,
    /// Whether the bytes are hosted in Logos Storage (vs Geofabrik fallback).
    pub hosted: bool,
    /// Unix seconds when this entry was registered (client-supplied).
    pub timestamp: i64,
    /// Account ID of the registrar (32-byte public key).
    pub registered_by: [u8; 32],
    /// Schema version of this record; increment when fields change.
    pub record_version: u8,
}

/// Global registry state held in the PDA at seed `[b"registry"]`.
///
/// `BTreeMap` serializes deterministically under borsh, so on-chain bytes
/// are identical across guest executions regardless of insertion order.
#[derive(Debug, Clone, Default, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct Registry {
    pub entries: BTreeMap<String, RegionEntry>,
}

// ── Error codes (stable, part of the IDL contract) ────────────────────────

pub const E_REGION_EMPTY: u32 = 1;
pub const E_BAD_LEVEL: u32 = 2;
pub const E_BAD_CHECKSUM: u32 = 3;
pub const E_BAD_TIMESTAMP: u32 = 4;
pub const E_BAD_VERSION: u32 = 5;
pub const E_REGISTRY_FULL: u32 = 6;
pub const E_DESERIALIZE: u32 = 7;
pub const E_BATCH_TOO_BIG: u32 = 8;
pub const E_BAD_PARENT: u32 = 9;
pub const E_BAD_REGION_ID: u32 = 10;
pub const E_BAD_CID: u32 = 11;
pub const E_BAD_SOURCE_URL: u32 = 12;

/// Structural validation. The guest enforces exactly these rules; the SDK
/// runs the same function before submitting so bad payloads fail locally.
pub fn validate_entry(e: &RegionEntry) -> Result<(), u32> {
    if e.region.is_empty() || e.region.len() > 128 {
        return Err(E_REGION_EMPTY);
    }
    if !e
        .region
        .bytes()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'/' || c == b'-')
    {
        return Err(E_BAD_REGION_ID);
    }
    if e.level > LEVEL_SUBREGION {
        return Err(E_BAD_LEVEL);
    }
    match (e.level, &e.parent) {
        (LEVEL_SUBREGION, Some(p)) if !p.is_empty() && p.len() <= 128 => {}
        (LEVEL_COUNTRY, None) => {}
        _ => return Err(E_BAD_PARENT),
    }
    if e.cid.is_empty() || e.cid.len() > 128 {
        return Err(E_BAD_CID);
    }
    if !is_md5_hex(&e.checksum) {
        return Err(E_BAD_CHECKSUM);
    }
    if e.version.is_empty() || e.version.len() > 16 {
        return Err(E_BAD_VERSION);
    }
    if e.timestamp <= 0 {
        return Err(E_BAD_TIMESTAMP);
    }
    if e.source_url.is_empty() || e.source_url.len() > 256 || !e.source_url.starts_with("http") {
        return Err(E_BAD_SOURCE_URL);
    }
    Ok(())
}

fn is_md5_hex(s: &str) -> bool {
    s.len() == 32
        && s.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good() -> RegionEntry {
        RegionEntry {
            region: "us/california".into(),
            parent: Some("us".into()),
            level: LEVEL_SUBREGION,
            cid: "bafktestcid0000000000000000000000000001".into(),
            source_url: "https://download.geofabrik.de/north-america/us/california-latest.osm.pbf"
                .into(),
            checksum: "35cff894271b8f56ddb81911dcffe089".into(),
            version: "261024".into(),
            hosted: true,
            timestamp: 1_798_000_000,
            registered_by: [7u8; 32],
            record_version: 1,
        }
    }

    fn country() -> RegionEntry {
        RegionEntry {
            region: "germany".into(),
            parent: None,
            level: LEVEL_COUNTRY,
            ..good()
        }
    }

    #[test]
    fn validate_accepts_minimal_entry() {
        assert_eq!(validate_entry(&good()), Ok(()));
        assert_eq!(validate_entry(&country()), Ok(()));
    }

    #[test]
    fn rejects_bad_checksum() {
        let mut e = good();
        e.checksum = "not-hex".into();
        assert_eq!(validate_entry(&e), Err(E_BAD_CHECKSUM));
        e.checksum = "35CFF894271B8F56DDB81911DCFFE089".into();
        assert_eq!(validate_entry(&e), Err(E_BAD_CHECKSUM));
    }

    #[test]
    fn rejects_zero_timestamp() {
        let mut e = good();
        e.timestamp = 0;
        assert_eq!(validate_entry(&e), Err(E_BAD_TIMESTAMP));
    }

    #[test]
    fn rejects_subregion_without_parent() {
        let mut e = good();
        e.parent = None;
        assert_eq!(validate_entry(&e), Err(E_BAD_PARENT));
    }

    #[test]
    fn rejects_country_with_parent() {
        let mut e = country();
        e.parent = Some("europe".into());
        assert_eq!(validate_entry(&e), Err(E_BAD_PARENT));
    }

    #[test]
    fn rejects_region_over_128_chars() {
        let mut e = good();
        e.region = "a/".repeat(70) + "a";
        assert_eq!(validate_entry(&e), Err(E_REGION_EMPTY));
    }

    #[test]
    fn rejects_bad_region_chars() {
        let mut e = good();
        e.region = "us/California".into();
        assert_eq!(validate_entry(&e), Err(E_BAD_REGION_ID));
        e.region = "us ca".into();
        assert_eq!(validate_entry(&e), Err(E_BAD_REGION_ID));
    }

    #[test]
    fn rejects_bad_cid() {
        let mut e = good();
        e.cid = String::new();
        assert_eq!(validate_entry(&e), Err(E_BAD_CID));
    }

    #[test]
    fn rejects_bad_source_url() {
        let mut e = good();
        e.source_url = "ftp://example.com/x.pbf".into();
        assert_eq!(validate_entry(&e), Err(E_BAD_SOURCE_URL));
    }

    #[test]
    fn borsh_roundtrip_preserves_entry() {
        let bytes = borsh::to_vec(&good()).unwrap();
        let back = RegionEntry::try_from_slice(&bytes).unwrap();
        assert_eq!(back, good());
    }

    #[test]
    fn borsh_bytes_deterministic() {
        let mut a = Registry::default();
        a.entries.insert("germany".into(), country());
        a.entries.insert("us/california".into(), good());
        let mut b = Registry::default();
        b.entries.insert("us/california".into(), good());
        b.entries.insert("germany".into(), country());
        assert_eq!(borsh::to_vec(&a).unwrap(), borsh::to_vec(&b).unwrap());
    }

    #[test]
    fn batch_size_capped() {
        assert_eq!(MAX_BATCH, 72);
    }

    #[test]
    fn registry_size_well_under_account_limit() {
        // 72 entries × 260 bytes is the spec-level worst case; assert the
        // full frozen-set-sized registry encodes far below 100 KiB.
        let mut reg = Registry::default();
        for i in 0..MAX_BATCH {
            let mut e = good();
            e.region = format!("r/{i:03}");
            e.parent = Some("r".into());
            reg.entries.insert(e.region.clone(), e);
        }
        let bytes = borsh::to_vec(&reg).unwrap();
        assert!(bytes.len() < MAX_ACCOUNT_BYTES, "{} bytes", bytes.len());
    }
}
