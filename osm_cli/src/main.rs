//! OSM distribution CLI (LP-0018).
//!
//! Subcommands: init-registry, discover, host, import, lookup,
//! check-updates, download. All heavy lifting lives in the SDK.

use clap::{Parser, Subcommand};
use osm_sdk::registry::{MockRegistryClient, RegistryClient, SpelCliRegistryClient};
use osm_sdk::storage::{InMemoryStorage, LogoscoreStorage, StorageBackend};
use osm_sdk::{HostOpts, OsmSdk, SdkError};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "osm",
    version,
    about = "OpenStreetMap distribution for the Logos ecosystem (LP-0018)",
    after_help = "Region names use Geofabrik paths from the frozen predefined set, e.g. `germany`, `us/california`, `india/central-zone`."
)]
struct Cli {
    /// Config file (TOML). Defaults to ./osm.toml or ~/.config/osm/osm.toml.
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    /// Registry program ID (64 hex chars). Overrides the config file.
    #[arg(long, global = true, env = "OSM_PROGRAM_ID")]
    program_id: Option<String>,

    /// lgs signer account id (e.g. an account from `lgs wallet`).
    #[arg(long, global = true, env = "OSM_SIGNER")]
    signer: Option<String>,

    /// Use the in-memory storage double (dev/testing — no logoscore).
    #[arg(long, global = true)]
    storage_memory: bool,

    /// Rewrite `https://download.geofabrik.de` with this base (mirror).
    #[arg(long, global = true, env = "OSM_GEOFABRIK_BASE")]
    geofabrik_base: Option<String>,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create the on-chain registry PDA (permissionless, idempotent).
    InitRegistry,
    /// List the frozen predefined region set (optionally refresh URLs from
    /// the live Geofabrik index).
    Discover {
        #[arg(long)]
        json: bool,
        /// Fetch the live index (network) for fresh canonical URLs.
        #[arg(long)]
        refresh: bool,
    },
    /// Download PBF(s) from Geofabrik, verify MD5, store in Logos Storage,
    /// register on-chain. Multiple regions batch into ONE transaction.
    Host {
        regions: Vec<String>,
        /// Opt out individual regions from the selection.
        #[arg(long)]
        exclude: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Import a local PBF: verify against Geofabrik's MD5, store, register.
    Import {
        region: String,
        file: PathBuf,
        /// Checksum override (offline mode; else fetched from Geofabrik).
        #[arg(long)]
        md5: Option<String>,
    },
    /// Query the on-chain registry.
    Lookup {
        #[arg(long, group = "by")]
        region: Option<String>,
        #[arg(long, group = "by")]
        parent: Option<String>,
        #[arg(long, group = "by")]
        cid: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Compare Geofabrik's current snapshot versions against the registry.
    CheckUpdates {
        #[arg(long)]
        json: bool,
    },
    /// Download a region: by CID from Logos Storage when hosted, else
    /// straight from Geofabrik.
    Download {
        region: String,
        #[arg(long, default_value = ".")]
        dest: PathBuf,
    },
}

#[derive(Debug, serde::Deserialize, Default)]
struct Config {
    program_id: Option<String>,
    idl_path: Option<PathBuf>,
    cli_bin: Option<PathBuf>,
    lgs_bin: Option<PathBuf>,
    wallet_home: Option<PathBuf>,
    signer_account_id: Option<String>,
    logoscore_bin: Option<PathBuf>,
    modules_dir: Option<PathBuf>,
    storage_config: Option<PathBuf>,
}

fn load_config(path: Option<&PathBuf>) -> Config {
    let candidates: Vec<PathBuf> = match path {
        Some(p) => vec![p.clone()],
        None => vec![PathBuf::from("osm.toml"), dirs_config_path()],
    };
    for c in candidates {
        if c.is_file() {
            let raw = std::fs::read_to_string(&c).unwrap_or_default();
            let cfg: Config = toml::from_str(&raw).unwrap_or_default();
            eprintln!("using config {}", c.display());
            return cfg;
        }
    }
    Config::default()
}

fn dirs_config_path() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default();
    PathBuf::from(home).join(".config/osm/osm.toml")
}

fn exit_code(err: &SdkError) -> i32 {
    match err {
        SdkError::ChecksumMismatch { .. } => 2,
        SdkError::RetriesExhausted { .. } => 3,
        SdkError::Registry { .. } | SdkError::InvalidEntry { .. } | SdkError::BatchTooBig(_) => 4,
        _ => 1,
    }
}

fn signer_from(cli: &Cli, cfg: &Config) -> Result<[u8; 32], SdkError> {
    let id = cli
        .signer
        .clone()
        .or_else(|| cfg.signer_account_id.clone())
        .unwrap_or_else(|| "default".into());
    // The CLI passes the account id to the registry client; the on-chain
    // instruction takes the 32-byte account key. Accept hex here, else hash
    // the textual id deterministically (localnet dev accounts).
    Ok(if id.len() == 64 && hex::decode(&id).is_ok() {
        let mut out = [0u8; 32];
        out.copy_from_slice(&hex::decode(&id).unwrap());
        out
    } else {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(id.as_bytes());
        h.finalize().into()
    })
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let code = run(cli).await;
    std::process::exit(code);
}

async fn run(cli: Cli) -> i32 {
    let cfg = load_config(cli.config.as_ref());
    let needs_chain = !matches!(cli.cmd, Cmd::Discover { .. });
    let program_id = match cli.program_id.clone().or_else(|| cfg.program_id.clone()) {
        Some(p) => p,
        None if needs_chain => {
            eprintln!(
                "no --program-id (or OSM_PROGRAM_ID, or config program_id); run `osm init-registry` after `lgs deploy` and set it"
            );
            return 1;
        }
        None => String::new(),
    };

    let storage: Box<dyn StorageBackend> = if cli.storage_memory {
        Box::new(InMemoryStorage::new())
    } else {
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
    };

    let registry: Box<dyn RegistryClient> = if cli.storage_memory {
        Box::new(MockRegistryClient::new())
    } else {
        Box::new(SpelCliRegistryClient {
            cli_bin: cfg
                .cli_bin
                .clone()
                .unwrap_or_else(|| PathBuf::from("./target/debug/osm_registry_cli")),
            idl_path: cfg
                .idl_path
                .clone()
                .unwrap_or_else(|| PathBuf::from("./idl/osm-registry.idl.json")),
            program_id: program_id.clone(),
            lgs_bin: cfg.lgs_bin.clone().unwrap_or_else(|| PathBuf::from("lgs")),
            wallet_home: cfg.wallet_home.clone().unwrap_or_else(|| {
                std::env::var("LEE_WALLET_HOME_DIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::from("./.lgs-wallet"))
            }),
            signer_account_id: cli
                .signer
                .clone()
                .or_else(|| cfg.signer_account_id.clone())
                .unwrap_or_else(|| "default".into()),
        })
    };

    let mut sdk = OsmSdk::new(storage, registry);
    if let Some(base) = &cli.geofabrik_base {
        sdk.geofabrik
            .set_url_rewrite("https://download.geofabrik.de", base.clone());
    }

    match cli.cmd {
        Cmd::InitRegistry => match sdk.registry.init_registry().await {
            Ok(()) => {
                println!(
                    "registry initialized (or already present): PDA {}",
                    sdk.registry.pda().await.unwrap_or_default()
                );
                0
            }
            Err(e) => fail(&e),
        },
        Cmd::Discover { json, refresh } => {
            let defs: Vec<osm_sdk::regions::RegionDef> = if refresh {
                match fetch_index() {
                    Some(idx) => match sdk.discover_refreshed(&idx) {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("live index refresh failed ({e}); using bundled set");
                            sdk.discover().to_vec()
                        }
                    },
                    None => {
                        eprintln!("live index fetch failed; using bundled set");
                        sdk.discover().to_vec()
                    }
                }
            } else {
                sdk.discover().to_vec()
            };
            if json {
                print_json(&defs);
            } else {
                println!("{:<28} {:<10} {:<9} URL", "REGION", "PARENT", "LEVEL");
                for d in &defs {
                    println!(
                        "{:<28} {:<10} {:<9} {}",
                        d.region,
                        d.parent.as_deref().unwrap_or("-"),
                        if d.level == 0 { "country" } else { "subregion" },
                        d.pbf_url
                    );
                }
                println!("\n{} predefined regions (frozen, non-overlapping)", defs.len());
            }
            0
        }
        Cmd::Host { ref regions, ref exclude, json } => {
            let selected: Vec<String> =
                regions.iter().filter(|r| !exclude.contains(r)).cloned().collect();
            if selected.is_empty() {
                eprintln!("no regions selected");
                return 1;
            }
            let signer = match signer_from(&cli, &cfg) {
                Ok(s) => s,
                Err(e) => return fail(&e),
            };
            match sdk
                .host_regions(&selected, signer, HostOpts::default())
                .await
            {
                Ok(report) => {
                    if json {
                        print_json(&report);
                    } else {
                        for e in &report.registered {
                            println!(
                                "hosted  {:<28} cid={} version={} checksum={}",
                                e.region, e.cid, e.version, e.checksum
                            );
                        }
                        for s in &report.skipped {
                            eprintln!("skipped {:<28} {}", s.region, s.error);
                        }
                        println!(
                            "\n{} registered in one batch tx, {} skipped",
                            report.registered.len(),
                            report.skipped.len()
                        );
                    }
                    0
                }
                Err(e) => fail(&e),
            }
        }
        Cmd::Import { ref region, ref file, ref md5 } => {
            let signer = match signer_from(&cli, &cfg) {
                Ok(s) => s,
                Err(e) => return fail(&e),
            };
            match sdk
                .import_local(region, file, signer, HostOpts::default(), md5.clone())
                .await
            {
                Ok(e) => {
                    println!(
                        "imported {} cid={} version={} checksum={} hosted={}",
                        e.region, e.cid, e.version, e.checksum, e.hosted
                    );
                    0
                }
                Err(e) => fail(&e),
            }
        }
        Cmd::Lookup { region, parent, cid, json } => {
            let result = if let Some(r) = region {
                sdk.resolve(&r)
                    .await
                    .map(|o| o.map(|e| vec![e]).unwrap_or_default())
            } else if let Some(p) = parent {
                sdk.query_by_parent(&p).await
            } else if let Some(c) = cid {
                sdk.query_by_cid(&c).await
            } else {
                eprintln!("one of --region/--parent/--cid is required");
                return 1;
            };
            match result {
                Ok(entries) => {
                    if json {
                        print_json(&entries);
                    } else if entries.is_empty() {
                        println!("no entries");
                    } else {
                        for e in &entries {
                            println!(
                                "{:<28} hosted={:<5} version={:<8} cid={} ts={}",
                                e.region, e.hosted, e.version, e.cid, e.timestamp
                            );
                        }
                    }
                    0
                }
                Err(e) => fail(&e),
            }
        }
        Cmd::CheckUpdates { json } => match sdk.check_updates().await {
            Ok(updates) => {
                if json {
                    print_json(&updates);
                } else if updates.is_empty() {
                    println!("all registered regions are up to date");
                } else {
                    for u in &updates {
                        println!(
                            "update available: {} registry={} geofabrik={}",
                            u.region, u.registry_version, u.central_version
                        );
                    }
                }
                0
            }
            Err(e) => fail(&e),
        },
        Cmd::Download { region, dest } => match sdk.download_region(&region, &dest).await {
            Ok(r) => {
                println!(
                    "downloaded {} from {:?} -> {} ({} bytes, checksum verified: {})",
                    r.region,
                    r.source,
                    r.path.display(),
                    r.bytes,
                    r.verified
                );
                0
            }
            Err(e) => fail(&e),
        },
    }
}

fn fail(e: &SdkError) -> i32 {
    eprintln!("error: {e}");
    exit_code(e)
}

fn print_json<T: serde::Serialize>(v: &T) {
    println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
}

fn fetch_index() -> Option<String> {
    // A blocking one-shot fetch in a cold thread keeps the async runtime
    // untouched; the SDK's client handles retries elsewhere.
    let url = "https://download.geofabrik.de/index-v1-nogeom.json";
    match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
    {
        Ok(c) => match c.get(url).send().and_then(|r| r.error_for_status()?.text()) {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("index fetch: {e}");
                None
            }
        },
        Err(e) => {
            eprintln!("http client: {e}");
            None
        }
    }
}
