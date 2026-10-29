//! C ABI bridge over the OSM SDK, consumed by the Basecamp core module
//! (`osm-module`) via dlopen — the same boundary the reference LP-0017
//! implementation uses.
//!
//! All string-JSON in/out. Every function returns a NUL-terminated UTF-8
//! JSON string (allocated, freed with `osm_free`) or NULL on error, with
//! the message retrievable from `osm_last_error()` (thread-local).
//!
//! The config JSON mirrors the CLI config keys:
//! `{"storage":"memory"|"logoscore", "program_id":"...", "rpc_url":"...",
//!   "cli_bin":"...", "wallet_home":"...",
//!   "signer_account_id":"...", "logoscore_bin":"...", "modules_dir":"...",
//!   "storage_config":"..." }`

use std::ffi::{c_char, CStr, CString};
use std::path::PathBuf;
use std::sync::OnceLock;

use osm_sdk::registry::{LezWalletRegistryClient, MockRegistryClient, RegistryClient};
use osm_sdk::storage::{InMemoryStorage, LogoscoreStorage, StorageBackend};
use osm_sdk::{HostOpts, OsmSdk};

#[derive(serde::Deserialize, Default, Clone)]
struct FfiConfig {
    #[serde(default)]
    storage: Option<String>,
    program_id: Option<String>,
    cli_bin: Option<PathBuf>,
    rpc_url: Option<String>,
    wallet_home: Option<PathBuf>,
    signer_account_id: Option<String>,
    logoscore_bin: Option<PathBuf>,
    modules_dir: Option<PathBuf>,
    storage_config: Option<PathBuf>,
}

const VERSION: &str = concat!("logos-osm-ffi/", env!("CARGO_PKG_VERSION"));

thread_local! {
    static LAST_ERROR: std::cell::RefCell<Option<CString>> = const { std::cell::RefCell::new(None) };
}

fn set_error(msg: String) {
    let c = CString::new(msg).unwrap_or_else(|_| CString::new("error").unwrap());
    LAST_ERROR.with(|l| *l.borrow_mut() = Some(c));
}

fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime")
    })
}

unsafe fn read_str<'a>(p: *const c_char) -> Result<&'a str, String> {
    if p.is_null() {
        return Err("null string argument".into());
    }
    CStr::from_ptr(p)
        .to_str()
        .map_err(|e| format!("invalid utf-8 argument: {e}"))
}

fn alloc_json<T: serde::Serialize>(v: &T) -> *mut c_char {
    match serde_json::to_string(v)
        .ok()
        .and_then(|s| CString::new(s).ok())
    {
        Some(c) => c.into_raw(),
        None => std::ptr::null_mut(),
    }
}

fn build_sdk(cfg: &FfiConfig) -> Result<OsmSdk, String> {
    let storage: Box<dyn StorageBackend> = match cfg.storage.as_deref() {
        Some("memory") => Box::new(InMemoryStorage::new()),
        _ => {
            let bin = cfg
                .logoscore_bin
                .clone()
                .unwrap_or_else(|| PathBuf::from("logoscore"));
            let modules = cfg
                .modules_dir
                .clone()
                .unwrap_or_else(|| PathBuf::from("./modules"));
            let scfg = cfg
                .storage_config
                .clone()
                .unwrap_or_else(|| PathBuf::from("storage.json"));
            Box::new(LogoscoreStorage::new(bin, modules, scfg))
        }
    };
    let registry: Box<dyn RegistryClient> = match cfg.storage.as_deref() {
        Some("memory") => Box::new(MockRegistryClient::new()),
        _ => {
            let program_id = cfg
                .program_id
                .clone()
                .ok_or_else(|| "config.program_id is required".to_string())?;
            Box::new(LezWalletRegistryClient {
                cli_bin: cfg
                    .cli_bin
                    .clone()
                    .unwrap_or_else(|| PathBuf::from("./target/debug/osm_registry_cli")),
                program_id,
                rpc_url: cfg.rpc_url.clone().unwrap_or_else(|| {
                    std::env::var("OSM_RPC_URL")
                        .unwrap_or_else(|_| "https://testnet.lez.logos.co".into())
                }),
                wallet_home: cfg.wallet_home.clone().unwrap_or_else(|| {
                    std::env::var("LEE_WALLET_HOME_DIR")
                        .map(PathBuf::from)
                        .unwrap_or_else(|_| PathBuf::from("./.lgs-wallet"))
                }),
                signer_account_id: cfg
                    .signer_account_id
                    .clone()
                    .unwrap_or_else(|| "default".into()),
            })
        }
    };
    Ok(OsmSdk::new(storage, registry))
}

fn signer_of(cfg: &FfiConfig) -> [u8; 32] {
    let id = cfg
        .signer_account_id
        .clone()
        .unwrap_or_else(|| "default".into());
    if id.len() == 64 && hex_ok(&id) {
        let mut out = [0u8; 32];
        if let Some(b) = unhex(&id) {
            out.copy_from_slice(&b);
            return out;
        }
        return out;
    }
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(id.as_bytes());
    h.finalize().into()
}

fn hex_ok(s: &str) -> bool {
    s.bytes().all(|c| c.is_ascii_hexdigit())
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

macro_rules! ffi_fn {
    // Every generated function is `unsafe extern "C"`: it dereferences the
    // caller's C string pointers. The safety contract is identical for all
    // of them, so it is stated once here and in the module docs.
    ($name:ident, $body:expr, $($arg:ident),*) => {
        /// # Safety
        ///
        /// All string arguments must be valid NUL-terminated UTF-8 C strings
        /// (or NULL, which is rejected) and must outlive the call.
        #[no_mangle]
        pub unsafe extern "C" fn $name($($arg: *const c_char),*) -> *mut c_char {
            LAST_ERROR.with(|l| *l.borrow_mut() = None);
            let result = (|| -> Result<*mut c_char, String> {
                $(let $arg = read_str($arg)?;)*
                let f = $body;
                Ok(f)
            })();
            match result {
                Ok(ptr) => ptr,
                Err(e) => {
                    set_error(e);
                    std::ptr::null_mut()
                }
            }
        }
    };
}

/// SDK + FFI version string.
#[no_mangle]
pub extern "C" fn osm_sdk_version() -> *const c_char {
    static V: OnceLock<CString> = OnceLock::new();
    V.get_or_init(|| CString::new(VERSION).unwrap()).as_ptr()
}

/// Free a string returned by any osm_* function.
///
/// # Safety
///
/// `ptr` must be a pointer returned by an `osm_*` function in this crate,
/// and must not be used after this call. Passing any other pointer is
/// undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn osm_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

/// Last error message for this thread, or NULL.
#[no_mangle]
pub extern "C" fn osm_last_error() -> *const c_char {
    LAST_ERROR.with(|l| {
        l.borrow()
            .as_ref()
            .map(|c| c.as_ptr())
            .unwrap_or(std::ptr::null())
    })
}

// The 72 predefined regions: JSON array of RegionDef.
ffi_fn!(
    osm_discover,
    {
        let cfg: FfiConfig = serde_json::from_str(cfg_json).map_err(|e| e.to_string())?;
        let _ = cfg; // discovery needs no chain/storage access
        let defs = osm_sdk::regions::predefined();
        alloc_json(&defs.to_vec())
    },
    cfg_json
);

// Resolve one region to its registry entry (or JSON null).
ffi_fn!(
    osm_resolve,
    {
        let cfg: FfiConfig = serde_json::from_str(cfg_json).map_err(|e| e.to_string())?;
        let sdk = build_sdk(&cfg)?;
        let entry = runtime()
            .block_on(sdk.resolve(region))
            .map_err(|e| e.to_string())?;
        alloc_json(&entry)
    },
    cfg_json,
    region
);

// Query entries: kind ∈ {"region","parent","cid"}.
ffi_fn!(
    osm_query,
    {
        let cfg: FfiConfig = serde_json::from_str(cfg_json).map_err(|e| e.to_string())?;
        let sdk = build_sdk(&cfg)?;
        let entries = match kind {
            "region" => runtime()
                .block_on(sdk.resolve(value))
                .map_err(|e| e.to_string())?
                .map(|e| vec![e])
                .unwrap_or_default(),
            "parent" => runtime()
                .block_on(sdk.query_by_parent(value))
                .map_err(|e| e.to_string())?,
            "cid" => runtime()
                .block_on(sdk.query_by_cid(value))
                .map_err(|e| e.to_string())?,
            other => return Err(format!("unknown query kind: {other}")),
        };
        alloc_json(&entries)
    },
    cfg_json,
    kind,
    value
);

// Check for available updates (RegionUpdate list).
ffi_fn!(
    osm_check_updates,
    {
        let cfg: FfiConfig = serde_json::from_str(cfg_json).map_err(|e| e.to_string())?;
        let sdk = build_sdk(&cfg)?;
        let updates = runtime()
            .block_on(sdk.check_updates())
            .map_err(|e| e.to_string())?;
        alloc_json(&updates)
    },
    cfg_json
);

// Bulk host: regions_csv is a comma-separated list of region paths.
ffi_fn!(
    osm_host,
    {
        let cfg: FfiConfig = serde_json::from_str(cfg_json).map_err(|e| e.to_string())?;
        let sdk = build_sdk(&cfg)?;
        let regions: Vec<String> = regions_csv
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let report = runtime()
            .block_on(sdk.host_regions(&regions, signer_of(&cfg), HostOpts::default()))
            .map_err(|e| e.to_string())?;
        alloc_json(&report)
    },
    cfg_json,
    regions_csv
);

// Import a local PBF with optional md5 override.
ffi_fn!(
    osm_import,
    {
        let cfg: FfiConfig = serde_json::from_str(cfg_json).map_err(|e| e.to_string())?;
        let sdk = build_sdk(&cfg)?;
        let md5_opt: Option<String> = if md5_opt.is_empty() {
            None
        } else {
            Some(md5_opt.to_string())
        };
        let entry = runtime()
            .block_on(sdk.import_local(
                region,
                std::path::Path::new(file_path),
                signer_of(&cfg),
                HostOpts::default(),
                md5_opt,
            ))
            .map_err(|e| e.to_string())?;
        alloc_json(&entry)
    },
    cfg_json,
    region,
    file_path,
    md5_opt
);

// Download a region (storage by CID, else Geofabrik fallback).
ffi_fn!(
    osm_download,
    {
        let cfg: FfiConfig = serde_json::from_str(cfg_json).map_err(|e| e.to_string())?;
        let sdk = build_sdk(&cfg)?;
        let report = runtime()
            .block_on(sdk.download_region(region, std::path::Path::new(dest_dir)))
            .map_err(|e| e.to_string())?;
        alloc_json(&report)
    },
    cfg_json,
    region,
    dest_dir
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_json_roundtrip() {
        let cfg = CString::new("{}").unwrap();
        let ptr = unsafe { osm_discover(cfg.as_ptr()) };
        assert!(!ptr.is_null(), "discover failed: {:?}", unsafe {
            let e = osm_last_error();
            if e.is_null() {
                None
            } else {
                Some(CStr::from_ptr(e).to_string_lossy().into_owned())
            }
        });
        let json = unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() };
        unsafe { osm_free(ptr) };
        let defs: Vec<osm_sdk::regions::RegionDef> = serde_json::from_str(&json).unwrap();
        assert_eq!(defs.len(), 72);
    }

    #[test]
    fn version_string_ok() {
        let v = unsafe {
            CStr::from_ptr(osm_sdk_version())
                .to_string_lossy()
                .into_owned()
        };
        assert!(v.starts_with("logos-osm-ffi/"));
    }

    #[test]
    fn resolve_memory_mode_returns_null_for_unknown() {
        let cfg = CString::new("{\"storage\":\"memory\"}").unwrap();
        let region = CString::new("not-a-region").unwrap();
        let ptr = unsafe { osm_resolve(cfg.as_ptr(), region.as_ptr()) };
        assert!(ptr.is_null());
        let msg = unsafe {
            CStr::from_ptr(osm_last_error())
                .to_string_lossy()
                .into_owned()
        };
        assert!(msg.contains("not in predefined set"));
    }

    #[test]
    fn resolve_memory_mode_returns_null_when_unregistered() {
        let cfg = CString::new("{\"storage\":\"memory\"}").unwrap();
        let region = CString::new("germany").unwrap();
        let ptr = unsafe { osm_resolve(cfg.as_ptr(), region.as_ptr()) };
        // resolves to JSON `null` (registered=none) — not an error
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() };
        unsafe { osm_free(ptr) };
        assert_eq!(json.trim(), "null");
    }
}
