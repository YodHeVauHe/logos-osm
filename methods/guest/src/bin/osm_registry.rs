//! The on-chain OSM registry (LP-0018) — SPEL/LEZ RISC0 guest program.
//!
//! State lives in a single PDA at seed `[b"registry"]` holding a borsh
//! `Registry` (a BTreeMap keyed by region path). 72 predefined regions
//! occupy well under the 100 KiB account ceiling.
//!
//! Instructions:
//! - `init_registry`    — permissionless, idempotent, creates the PDA.
//! - `register_region`  — upsert one region entry.
//! - `register_batch`   — upsert many entries in one tx (bulk hosting);
//!                        payload is hex(borsh(Vec<RegionEntry>)) because
//!                        region paths and URLs make text packing unsafe.
//!
//! Structural validation (field shapes, level/parent consistency, MD5 hex,
//! timestamp sanity) is shared with the host via `osm_registry_core`.

#![no_main]

use spel_framework::prelude::*;
use lee_core::account::Data;

use osm_registry_core::{
    validate_entry, Registry, RegionEntry, MAX_ACCOUNT_BYTES, MAX_BATCH,
    E_BAD_TIMESTAMP, E_DESERIALIZE, E_REGISTRY_FULL,
};

risc0_zkvm::guest::entry!(main);

#[lez_program]
mod osm_registry {
    #[allow(unused_imports)]
    use super::*;

    /// Open the registry. Permissionless — anyone can call this once.
    #[instruction]
    pub fn init_registry(
        #[account(init, pda = [literal("registry")])]
        mut registry: AccountWithMetadata,
        #[account(signer)]
        authority: AccountWithMetadata,
    ) -> SpelResult {
        let cu_start = risc0_zkvm::guest::env::cycle_count();
        let empty = Registry::default();
        let bytes = borsh::to_vec(&empty)
            .map_err(|e| SpelError::SerializationError { message: e.to_string() })?;
        registry.account.data = Data::try_from(bytes)
            .map_err(|_| SpelError::custom(E_REGISTRY_FULL, "registry bytes overflow".to_string()))?;
        let cu_end = risc0_zkvm::guest::env::cycle_count();
        risc0_zkvm::guest::env::log(&format!("CU init_registry cycles={}", cu_end - cu_start));
        Ok(SpelOutput::execute(vec![registry, authority], vec![]))
    }

    /// Upsert one region entry. Idempotent: re-registering a region with a
    /// fresh CID/version (an update, or a second hoster mirroring it)
    /// replaces the previous entry.
    #[instruction]
    pub fn register_region(
        #[account(mut, pda = [literal("registry")])]
        mut registry: AccountWithMetadata,
        #[account(signer)]
        authority: AccountWithMetadata,
        region: String,
        parent: String,
        level: u8,
        cid: String,
        source_url: String,
        checksum: String,
        version: String,
        hosted: bool,
        timestamp: i64,
        registered_by: [u8; 32],
    ) -> SpelResult {
        let cu_start = risc0_zkvm::guest::env::cycle_count();
        if timestamp <= 0 {
            return Err(SpelError::custom(
                E_BAD_TIMESTAMP,
                "timestamp must be positive".to_string(),
            ));
        }
        let entry = RegionEntry {
            region,
            parent: if parent.is_empty() { None } else { Some(parent) },
            level,
            cid,
            source_url,
            checksum,
            version,
            hosted,
            timestamp,
            registered_by,
            record_version: 1,
        };
        validate_entry(&entry).map_err(|code| {
            SpelError::custom(code, format!("invalid region entry: {}", entry.region))
        })?;

        let mut state = decode_registry(&registry.account.data)?;
        state.entries.insert(entry.region.clone(), entry);
        write_registry(&mut registry.account.data, state)?;

        let cu_end = risc0_zkvm::guest::env::cycle_count();
        risc0_zkvm::guest::env::log(&format!("CU register_region cycles={}", cu_end - cu_start));
        Ok(SpelOutput::execute(vec![registry, authority], vec![]))
    }

    /// Upsert many region entries in one transaction (bulk hosting).
    /// Payload = hex(borsh(Vec<RegionEntry>)).
    #[instruction]
    pub fn register_batch(
        #[account(mut, pda = [literal("registry")])]
        mut registry: AccountWithMetadata,
        #[account(signer)]
        authority: AccountWithMetadata,
        payload: String,
        registered_by: [u8; 32],
    ) -> SpelResult {
        let cu_start = risc0_zkvm::guest::env::cycle_count();
        let raw = hex_decode(&payload)
            .map_err(|_| SpelError::custom(E_DESERIALIZE, "payload is not hex".to_string()))?;
        let entries: Vec<RegionEntry> = borsh::from_slice(&raw)
            .map_err(|e| SpelError::custom(E_DESERIALIZE, format!("payload borsh: {e}")))?;
        if entries.is_empty() || entries.len() > MAX_BATCH {
            return Err(SpelError::custom(
                E_DESERIALIZE,
                format!("batch size {} outside 1..={MAX_BATCH}", entries.len()),
            ));
        }
        let mut state = decode_registry(&registry.account.data)?;
        for entry in entries {
            if entry.timestamp <= 0 {
                return Err(SpelError::custom(
                    E_BAD_TIMESTAMP,
                    "timestamp must be positive".to_string(),
                ));
            }
            validate_entry(&entry).map_err(|code| {
                SpelError::custom(code, format!("invalid region entry: {}", entry.region))
            })?;
            state.entries.insert(entry.region.clone(), entry);
        }
        write_registry(&mut registry.account.data, state)?;

        let cu_end = risc0_zkvm::guest::env::cycle_count();
        risc0_zkvm::guest::env::log(&format!("CU register_batch cycles={}", cu_end - cu_start));
        Ok(SpelOutput::execute(vec![registry, authority], vec![]))
    }

    fn decode_registry(data: &Data) -> Result<Registry, SpelError> {
        if data.is_empty() {
            return Ok(Registry::default());
        }
        borsh::from_slice::<Registry>(&data)
            .map_err(|e| SpelError::custom(E_DESERIALIZE, format!("registry decode: {e}")))
    }

    fn write_registry(slot: &mut Data, state: Registry) -> Result<(), SpelError> {
        let bytes = borsh::to_vec(&state)
            .map_err(|e| SpelError::SerializationError { message: e.to_string() })?;
        if bytes.len() > MAX_ACCOUNT_BYTES {
            return Err(SpelError::custom(
                E_REGISTRY_FULL,
                format!("registry would exceed {MAX_ACCOUNT_BYTES} bytes"),
            ));
        }
        *slot = Data::try_from(bytes)
            .map_err(|_| SpelError::custom(E_REGISTRY_FULL, "registry bytes overflow".to_string()))?;
        Ok(())
    }

    fn hex_decode(s: &str) -> Result<Vec<u8>, ()> {
        fn val(c: u8) -> Result<u8, ()> {
            match c {
                b'0'..=b'9' => Ok(c - b'0'),
                b'a'..=b'f' => Ok(c - b'a' + 10),
                b'A'..=b'F' => Ok(c - b'A' + 10),
                _ => Err(()),
            }
        }
        if s.len() % 2 != 0 {
            return Err(());
        }
        let b = s.as_bytes();
        let mut out = Vec::with_capacity(b.len() / 2);
        let mut i = 0;
        while i < b.len() {
            out.push(val(b[i])? * 16 + val(b[i + 1])?);
            i += 2;
        }
        Ok(out)
    }
}
