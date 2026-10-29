# LP-0018 OSM Distribution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the LP-0018 deliverable — an on-chain OSM registry, an OSM registry SDK/module, a CLI, and an OSM distribution Basecamp app — implementing the full fetch → store → register → download pipeline for Geofabrik PBF snapshots on Logos testnet 0.4.

**Architecture:** Monorepo with a RISC Zero guest program directly using LEZ v0.3.1 plan/apply APIs. The registry is stored as Borsh `Registry` bytes in a program-owned public PDA shard; host SDK and CLI query the v0.3 sequencer and submit public wallet transactions.

**Tech Stack:** Rust, `lee_core` from LEZ tag `v0.3.1`, RISC Zero 3.0.5, rzup `embed_methods` guest build, v0.3.1 wallet/RPC client, Nix + `logos-module-builder`, `logoscore` storage CLI, Qt Remote Objects.

> Port update (2026-10-29): legacy SPEL and rev `27360cb` task details below are historical planning notes. The active v0.3.1 port design and implementation plan are in [`docs/superpowers/specs/2026-10-29-lez-v0.3-port-design.md`](superpowers/specs/2026-10-29-lez-v0.3-port-design.md) and [`docs/superpowers/plans/2026-10-29-lez-v0.3-port.md`](superpowers/plans/2026-10-29-lez-v0.3-port.md).

**Spec:** `../LP-0018.md` (copy of the prize spec, committed at repo root for reference; authoritative sections: Success Criteria, Scope, Adoption, Predefined Regions).

## Global Constraints

- Registry entry fields (spec, minimum): `region`, `parent`, `level`, `cid`, `source_url`, `checksum`, `version`, `hosted`, `timestamp`.
- `region` (Geofabrik path) is the unique key; `parent`/`level` make tree position explicit; country entries have `parent: null`, decomposed leaves have `parent` = `us` | `india` | `china` | `russia`.
- Predefined set is a **closed, non-overlapping set of exactly 72 regions** (48 country-level + 24 decomposed leaves) — see Task 1's `regions.json` contents. Leaves are frozen; do not add tree levels.
- Checksum = Geofabrik's published MD5 (32-char lowercase hex), verified at import time only; download integrity comes from the CID.
- Dual license: `MIT OR Apache-2.0` (SPDX everywhere, both license files committed).
- No mandatory centralized third-party services: beyond sequencer/LEZ node/Logos Storage/Geofabrik, nothing required; any extra service must be disableable and disclosed.
- Storage uploads retry on transient failures with exponential back-off, then surface a clear error.
- Registry account budget: single PDA ≤ 100 KiB (LEZ account limit). 72 entries × ~260 B ≈ 19 KB — keep entries bounded (region ≤ 128 B, cid ≤ 128 B, source_url ≤ 256 B, checksum 32 hex, version ≤ 16 B).
- Guest code must compile under `#![no_main]` with `risc0_zkvm::guest::entry!(main)`; only host crates may use std freely.
- Cycle counts must be logged from the guest (`risc0_zkvm::guest::env::cycle_count()`) for `register_region` and `register_batch`, and documented in `docs/cycles.md`.
- CI must be green on the default branch: e2e (host → query → download) runs against a real local sequencer; the Geofabrik-dependent live test is a separate dispatch-only job so CI never depends on network flakiness.
- Toolchain pins (deterministic program ID): `spel-framework` and `lee_core` pinned to git revs (start from reference: `lee_core` rev `27360cb7d6ccb2bfbcca7d171bab8a3938490264`; spel version vendored in `reference/logoz/vendor/spel-framework`). Bumping revs for testnet 0.4 compatibility is allowed but must be done deliberately in one commit and re-verified.
- Local dev machine is Windows (no Docker/Nix/Node): guest builds, Nix module builds, and localnet runs happen in CI or on Linux; host-side Rust (core, sdk, cli, ffi) must build and test on Windows with `cargo test`.

## Review Focus

1. **Geofabrik URL paths are parent-chain based, not id based** — e.g. `india/central-zone` downloads from `.../asia/india/central-zone-latest.osm.pbf`, `russia/*` from `.../russia/*` (russia is a root), `us/california` from `.../north-america/us/california-...`. A URL built from the region path alone 404s. Test: Task 3 `url_is_parent_chain` pins every bundled URL against the real index snapshot fixture.
2. **`ireland` and `malaysia` have no exact Geofabrik extracts** (`ireland-and-northern-ireland`, `malaysia-singapore-brunei`). The registry keys stay `ireland`/`malaysia`; the URL/checksum resolution must use the mapped extract. Test: Task 3 `mapped_regions_resolve`.
3. **`-latest.osm.pbf` is a 307 alias to a dated immutable file**; version = dated filename `YYMMDD`. Parsing must follow the redirect (or read the `.md5` embedded filename) and never treat `latest` as a version. Test: Task 4 `version_from_redirect` + `version_from_md5_filename`.
4. **md5 file format is `<32-hex>  <filename>`** (two spaces, filename may differ from URL). Parse must not reject the dated filename variant. Test: Task 4 `parse_md5_file_variants`.
5. **Batch register payload safety** — region paths contain `/`, URLs contain `:` and `/`; no comma/CSV packing. `register_batch` takes hex-encoded borsh `Vec<RegionEntry>`. Test: Task 2 roundtrip + Task 6 `batch_payload_roundtrip`.
6. **Overlapping-region guard** (spec: bulk selection within the closed set never overlaps; if non-partition regions are ever exposed, warn). Test: Task 7 `bulk_host_rejects_overlap`.

---

### Task 1: Repo scaffold, licenses, and the frozen region set

**Files:**
- Create: `Cargo.toml` (workspace), `.gitignore`, `LICENSE-MIT`, `LICENSE-APACHE`, `README.md` (stub), `scaffold.toml`, `regions.json`, `docs/LP-0018.md` (copy of spec), `docs/plan.md` (this plan, copied)
- Create: `.github/workflows/ci.yml` placeholder (filled in Task 12)

**Interfaces:**
- Produces: `regions.json` — array of `{ "region": string (spec path), "parent": string|null, "level": "country"|"subregion", "name": string, "index_id": string, "pbf_url": string, "continent": string }`. This file is the single source of truth for the closed set; every later task reads it.

- [ ] **Step 1: Write `regions.json`** with exactly these 72 entries (fields above; `pbf_url` = `https://download.geofabrik.de/<continent>/<index_id_path>-latest.osm.pbf` where `index_id_path` = index id for US (`north-america/us/<state>`), `<continent>/<index_id>` otherwise; the two mapped regions use their real extracts):

  Countries (48, `level: country`, `parent: null`):
  - Europe (`europe`): `germany`, `france`, `great-britain`, `italy`, `spain`, `poland`, `netherlands`, `belgium`, `switzerland`, `austria`, `czech-republic`, `sweden`, `norway`, `denmark`, `finland`, `portugal`, `greece`, `ireland`→`index_id: ireland-and-northern-ireland`, `hungary`, `romania`, `bulgaria`, `ukraine`, `belarus`, `turkey`
  - North America (`north-america`): `canada`, `mexico`
  - Asia (`asia`): `japan`, `south-korea`, `indonesia`, `thailand`, `vietnam`, `malaysia`→`index_id: malaysia-singapore-brunei`, `philippines`, `pakistan`, `bangladesh`, `iran`
  - Oceania (`australia-oceania`): `australia`
  - South America (`south-america`): `brazil`, `argentina`, `colombia`, `peru`, `chile`
  - Africa (`africa`): `south-africa`, `egypt`, `nigeria`, `kenya`, `morocco`, `ethiopia`

  Decomposed leaves (24, `level: subregion`, `parent` = decomposed country, `continent` per extract):
  - `us` (parent "us", continent `north-america`, index ids `us/california`, `us/texas`, `us/florida`, `us/new-york`, `us/washington`, `us/illinois`, `us/georgia`, `us/pennsylvania`)
  - `india` (continent `asia`, index ids WITHOUT prefix: `central-zone`, `eastern-zone`, `north-eastern-zone`, `northern-zone`, `southern-zone`, `western-zone`)
  - `china` (continent `asia`, index ids: `guangdong`, `jiangsu`, `shandong`, `zhejiang`, `sichuan`, `henan`)
  - `russia` (continent `russia` — it is a root, index ids: `central-fed-district`, `northwestern-fed-district`, `volga-fed-district`, `siberian-fed-district`)

  README documents the two index mismatches (`ireland`, `malaysia`) as a spec deviation note.

- [ ] **Step 2: Write workspace `Cargo.toml`** (members: `osm_registry_core`, `osm_sdk`, `osm_cli`, `ffi`; exclude: `methods`, `examples`; workspace deps: borsh 1 derive, serde 1 derive, serde_json 1, thiserror 2, anyhow 1, tokio 1 full, clap 4 derive+env, reqwest 0.12 json+rustls-tls, md-5 0.10, hex 0.4, async-trait 0.1, tracing 0.1, wiremock 0.6 dev, tempfile 3 dev) + `.gitignore` (target/, result/, .direnv/, *.lgx) + both license files (standard MIT and Apache-2.0 texts) + `README.md` stub with the deviation note + `scaffold.toml` (`[build] guest = "docker"`).
- [ ] **Step 3: Verify** — `cargo metadata --no-deps` succeeds (empty workspace members that don't exist yet are created as stub crates in Task 2; for this task, verify JSON validity instead: `python -c "import json;d=json.load(open('regions.json'));assert len(d)==72"`).
- [ ] **Step 4: Commit** — `git add -A && git commit -m "chore: scaffold repo with frozen predefined region set"`

### Task 2: `osm_registry_core` — shared on-chain/off-chain types

**Files:**
- Create: `osm_registry_core/Cargo.toml`, `osm_registry_core/src/lib.rs`
- Test: `osm_registry_core/src/lib.rs` (`#[cfg(test)]`)

**Interfaces:**
- Produces:
  - `pub const MAX_BATCH: usize = 72;`
  - `#[derive(...BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)] pub struct RegionEntry { pub region: String, pub parent: Option<String>, pub level: u8, pub cid: String, pub source_url: String, pub checksum: String, pub version: String, pub hosted: bool, pub timestamp: i64, pub registered_by: [u8; 32], pub record_version: u8 }` (`record_version` starts at 1)
  - `pub const LEVEL_COUNTRY: u8 = 0; pub const LEVEL_SUBREGION: u8 = 1;`
  - `#[derive(Default...)] pub struct Registry { pub entries: std::collections::BTreeMap<String, RegionEntry> }` (BTreeMap for deterministic borsh bytes)
  - `pub fn validate_entry(e: &RegionEntry) -> Result<(), u32>` and error code consts: `E_REGION_EMPTY=1, E_BAD_LEVEL=2, E_BAD_CHECKSUM=3, E_BAD_TIMESTAMP=4, E_BAD_VERSION=5, E_REGISTRY_FULL=6, E_DESERIALIZE=7, E_BATCH_TOO_BIG=8, E_BAD_PARENT=9, E_BAD_REGION_ID=10`
- Guest (Task 9) and SDK (Tasks 5–7) both import this crate.

- [ ] **Step 1: Failing tests** — `validate_accepts_minimal_entry`, `rejects_bad_checksum` (not 32 hex), `rejects_zero_timestamp`, `rejects_subregion_without_parent`, `rejects_country_with_parent`, `rejects_region_over_128_chars`, `borsh_roundtrip_preserves_entry`, `borsh_bytes_deterministic` (serialize two Registries with same entries inserted in different order → identical bytes), `batch_size_capped` (MAX_BATCH == 72).
- [ ] **Step 2:** `cargo test -p osm_registry_core` → FAIL (not implemented).
- [ ] **Step 3: Implement** types + `validate_entry` (checks: region non-empty ≤128 and chars `[a-z0-9/-]`; level ∈ {0,1}; checksum is exactly 32 lowercase hex; version non-empty ≤16; timestamp > 0; level==1 ⇒ parent Some(non-empty ≤128); level==0 ⇒ parent None; cid non-empty ≤128; source_url non-empty ≤256 and starts with `http`; registered_by any).
- [ ] **Step 4:** `cargo test -p osm_registry_core` → PASS.
- [ ] **Step 5: Commit** — `feat(core): RegionEntry, Registry, validation, error codes`

### Task 3: `osm_sdk::regions` — closed-set loading and Geofabrik URL resolution

**Files:**
- Create: `osm_sdk/Cargo.toml`, `osm_sdk/src/lib.rs` (module decls + `error.rs`), `osm_sdk/src/regions.rs`, fixture `osm_sdk/tests/fixtures/geofabrik-index-sample.json` (10 real entries copied from the live index, incl. `us/california`, `central-zone`, `guangdong`, `central-fed-district`, `ireland-and-northern-ireland`, `malaysia-singapore-brunei`, `germany`, `australia`)
- Test: `osm_sdk/tests/regions_tests.rs`

**Interfaces:**
- Consumes: `regions.json` via `include_str!("../../regions.json")`.
- Produces:
  - `pub struct RegionDef { pub region: String, pub parent: Option<String>, pub level: u8, pub name: String, pub index_id: String, pub pbf_url: String, pub continent: String }`
  - `pub fn predefined() -> &'static [RegionDef]` (lazily parsed, embedded)
  - `pub fn get(region: &str) -> Option<&'static RegionDef>`
  - `pub fn decomposed_parents() -> [&'static str; 4]` → `["us", "india", "china", "russia"]`
  - `pub struct IndexEntry { pub id: String, pub parent: Option<String>, pub name: String, pub pbf_url: Option<String> }` and `pub fn parse_index(json: &str) -> Result<Vec<IndexEntry>, SdkError>` (GeoJSON FeatureCollection → properties `{id, parent, name, urls.pbf}`; missing `urls.pbf` ⇒ skip entry)
  - `pub fn refresh_from_index(json: &str) -> Result<Vec<RegionDef>, SdkError>` (for each predefined region, find index entry by `index_id` (US) or by `(id == leaf, parent == decomposed country)` (india/china/russia) or by `id == region` (countries); take the LIVE `urls.pbf`; error if any of the 72 cannot be resolved)
  - `pub fn md5_url(pbf_url: &str) -> String` → `format!("{pbf_url}.md5")`

- [ ] **Step 1: Failing tests** — `predefined_has_72_regions`; `no_region_contains_another` (no region path is a prefix-path of another, i.e. set is non-overlapping: for the 4 decomposed parents assert `us`, `india`, `china`, `russia` are NOT themselves entries); `url_is_parent_chain` (every `pbf_url` starts `https://download.geofabrik.de/<continent>/` and the India zones' URLs contain `/asia/india/`, Russia districts' `/russia/`, US states' `/north-america/us/`); `mapped_regions_resolve` (`ireland` → pbf_url contains `ireland-and-northern-ireland-latest.osm.pbf`, `malaysia` → `malaysia-singapore-brunei-latest.osm.pbf`); `parse_index_reads_sample` (fixture → contains id `us/california` with the correct pbf_url; skips entries without pbf url); `refresh_resolves_all_72` (fixture extended inline to cover all 72 via generated entries — test builds a synthetic index JSON containing all 72 index ids with URL patterns and asserts refresh returns 72 defs); `md5_url_appends_suffix`.
- [ ] **Step 2:** `cargo test -p osm_sdk regions` → FAIL.
- [ ] **Step 3: Implement** `regions.rs` (+ minimal `error.rs`: `#[derive(thiserror::Error)] pub enum SdkError { #[error(...)] Json(#[from] serde_json::Error), #[error("region not in predefined set: {0}")] UnknownRegion(String), #[error("geofabrik index does not resolve region {0}")] IndexMismatch(String), #[error(transparent)] Other(#[from] anyhow::Error) }`).
- [ ] **Step 4:** tests PASS.
- [ ] **Step 5: Commit** — `feat(sdk): frozen region set, index parsing, URL resolution`

### Task 4: `osm_sdk::geofabrik` — checksums, versions, downloads

**Files:**
- Create: `osm_sdk/src/geofabrik.rs`
- Test: `osm_sdk/tests/geofabrik_tests.rs` (uses `wiremock` + `tempfile`)

**Interfaces:**
- Produces:
  - `pub fn parse_md5_file(text: &str) -> Result<String, SdkError>` (accept `\n`-terminated or not, ` *`/two-space separators, returns lowercase hex; rejects non-hex/short)
  - `pub fn version_from_dated_filename(location: &str) -> Option<String>` (regex-free: find `-<6 digits>.osm.pbf` in a URL/path, return the 6 digits)
  - `pub struct Geofabrik { http: reqwest::Client }`; `impl Geofabrik { pub fn new() -> Self; pub async fn fetch_checksum(&self, pbf_url: &str) -> Result<String, SdkError>; pub async fn current_version(&self, pbf_url: &str) -> Result<String, SdkError> (GET, no redirect-follow for version: disable redirect policy, read `Location` header → `version_from_dated_filename`; fallback: `Last-Modified` header → `yyyymmdd` from UTC date); pub async fn download(&self, url: &str, dest: &Path, retries: u32) -> Result<u64, SdkError> }` (download follows redirects, streams to file, retries up to `retries` times with exponential back-off starting 1s + jitter, returns byte count; verifies non-empty 200)
  - `pub async fn sha256_file(path: &Path) -> Result<String, SdkError>`; `pub async fn md5_file(path: &Path) -> Result<String, SdkError>` (md-5 crate)
- Note: `reqwest::redirect::Policy::none()` client for `current_version`; default-policy client for `download`/`fetch_checksum`.

- [ ] **Step 1: Failing tests** — `parse_md5_file_variants` (exact fixture strings: `d72ae8dde254945f12adc92386fde236  berlin-latest.osm.pbf\n`, dated variant `...  california-261024.osm.pbf`, ` *` binary-marker variant, no-trailing-newline variant, uppercase-hex rejected → Err); `version_from_dated_filename` (`.../california-261024.osm.pbf` → `261024`, `latest` URL → None); `download_writes_file` (wiremock serves 64 KiB body → file exists, md5 matches body hash, returns 65536); `download_retries_then_succeeds` (wiremock: first 2 responses 503, then 200 → succeeds; assert 3 calls); `download_fails_after_retries` (always 500 → Err after configured retries); `current_version_reads_location_header` (wiremock 307 with `Location: .../california-261024.osm.pbf` → `261024`).
- [ ] **Step 2:** FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4:** PASS.
- [ ] **Step 5: Commit** — `feat(sdk): geofabrik checksum/version/download with retries`

### Task 5: `osm_sdk::storage` — pluggable storage backend

**Files:**
- Create: `osm_sdk/src/storage.rs`
- Test: `osm_sdk/tests/storage_tests.rs`

**Interfaces:**
- Produces:
  - `#[async_trait] pub trait StorageBackend: Send + Sync { async fn put(&self, file: &Path, name: &str) -> Result<PutReceipt, SdkError>; async fn get(&self, cid: &str, dest: &Path) -> Result<(), SdkError>; fn name(&self) -> &'static str; }`
  - `pub struct PutReceipt { pub cid: String, pub size: u64 }`
  - `pub struct InMemoryStorage { ... }` (deterministic test double: `cid = format!("bafk-test-{:016x}", sha256(bytes)[..8])`; get returns stored bytes)
  - `pub struct LogoscoreStorage { bin: PathBuf, modules_dir: PathBuf, config: PathBuf, max_retries: u32 }` — shells out to the `logoscore` headless binary: one invocation per op, `logoscore -m <modules_dir> --load-modules storage_module -c "storage_module.init(@<config>)" -c "storage_module.start()" -c "storage_module.uploadFile(<file>)"` / `-c "storage_module.downloadFile(<cid>, <dest>)"`, parses the CID from stdout JSON; retries with the same exponential back-off helper as Task 4 (shared `pub(crate) fn backoff_delays(retries: u32) -> Vec<Duration>`); clear error with full stderr after retries exhausted. Exact command strings live in ONE const so they can be adjusted against real logoscore behavior without re-deriving logic.
  - `pub fn backoff_delays(retries: u32) -> Vec<std::time::Duration>` (1s, 2s, 4s, ... capped 30s)

- [ ] **Step 1: Failing tests** — `inmemory_roundtrip` (put → receipt.cid non-empty; get to temp file → bytes equal); `backoff_delays_shape` ([1s,2s,4s,8s] for retries=4, capped at 30s for retries=8); `logoscore_builds_expected_command` (construct LogoscoreStorage, call `#[cfg(test)] pub(crate) fn command_for_put(&self, file)` → assert arg vector contains `--load-modules`, `storage_module`, `uploadFile(...)`; no binary spawned); `logoscore_parses_cid_from_output` (parse helper fed fixture stdout JSON `{"success":true,"value":"bafk...abc"}` → Ok(cid)); `logoscore_error_includes_stderr` (parse helper fed failure JSON + stderr text → Err mentioning "storage upload failed after N retries" — text pinned).
- [ ] **Step 2:** FAIL.
- [ ] **Step 3: Implement** (logoscore spawn path is thin over `tokio::process::Command`; keep output parsing pure/testable).
- [ ] **Step 4:** PASS.
- [ ] **Step 5: Commit** — `feat(sdk): storage backend trait, in-memory test double, logoscore backend`

### Task 6: `osm_sdk::registry` — on-chain registry client (SPEL CLI + lgs account reads)

**Files:**
- Create: `osm_sdk/src/registry.rs`
- Test: `osm_sdk/tests/registry_tests.rs`

**Interfaces:**
- Produces:
  - `#[async_trait] pub trait RegistryClient: Send + Sync { async fn pda(&self) -> Result<String, SdkError>; async fn init_registry(&self) -> Result<(), SdkError>; async fn register(&self, entries: &[RegionEntry], signer: [u8; 32]) -> Result<(), SdkError>; async fn fetch_all(&self) -> Result<Option<std::collections::BTreeMap<String, RegionEntry>>, SdkError>; }` (`register` with 1 entry → `register_region` instruction; >1 → `register_batch`; entries pre-validated by caller)
  - `pub fn batch_payload(entries: &[RegionEntry]) -> Result<String, SdkError>` (hex(borsh(Vec<RegionEntry>))) and `pub fn parse_batch_payload(hex: &str) -> Result<Vec<RegionEntry>, SdkError>`
  - `pub struct SpelCliRegistryClient { cli_bin: PathBuf, idl_path: PathBuf, program_id: String, lgs_bin: PathBuf, wallet_home: PathBuf }` — `pda()`: run `<cli> --dry-run=text init-registry --anchorer <program-default>` and parse the `PDA registry -> <addr>` line (same parser contract as the reference implementation); `fetch_all()`: `lgs wallet -- account get --account-id Public/<pda> --raw` → parse first `{`..last `}` JSON → `data` hex → borsh `Registry`; `register()`: `<cli> -i <idl> -p <pid> register-region --region ... --cid ...` or `register-batch --payload <hex>`. All via `tokio::process::Command` with `LEE_WALLET_HOME_DIR` env.
  - `pub struct MockRegistryClient` (in-crate, `#[cfg(any(test, feature = "test-util"))]`): stores registered entries in a BTreeMap, `fetch_all` returns them; counts `register` calls and asserts batch ≤ MAX_BATCH.
  - Query helpers (free functions, off-chain): `pub fn query_by_region(map: &BTreeMap<String, RegionEntry>, region: &str) -> Option<&RegionEntry>`; `query_by_parent(map, parent) -> Vec<&RegionEntry>`; `query_by_cid(map, cid) -> Vec<&RegionEntry>`.

- [ ] **Step 1: Failing tests** — `batch_payload_roundtrip` (2 entries → hex → parse → equal); `batch_payload_rejects_empty`; `query_by_parent_filters_and_sorts_by_timestamp` (insert india zones with shuffled timestamps → returned vec ascending by timestamp — this pins the spec's "entries are ordered by timestamp for version sorting"); `query_by_cid_matches_multiple`; `mock_client_registers_and_fetches` (init → register 1 → register 3 → fetch_all has 4 entries, register call count 2).
- [ ] **Step 2:** FAIL.
- [ ] **Step 3: Implement.** (SpelCliRegistryClient's process-running methods get pure parsing helpers `parse_pda_line(&str)`, `parse_account_get_json(&str)` tested with fixture strings copied from the reference README/impl.)
- [ ] **Step 4:** PASS.
- [ ] **Step 5: Commit** — `feat(sdk): registry client trait, spel-cli backend, off-chain queries`

### Task 7: `osm_sdk` workflows — host, bulk host, import, updates, download

**Files:**
- Create: `osm_sdk/src/host.rs`, `osm_sdk/src/updates.rs`, `osm_sdk/src/download.rs`
- Test: `osm_sdk/tests/workflow_tests.rs` (wiremock Geofabrik + InMemoryStorage + MockRegistryClient)

**Interfaces:**
- Consumes: Tasks 3–6 APIs.
- Produces (on `pub struct OsmSdk` built by `OsmSdk::new(geofabrik, storage: Box<dyn StorageBackend>, registry: Box<dyn RegistryClient>)`):
  - `pub struct HostOpts { pub hosted: bool /* default true */, pub max_retries: u32 /* default 3 */ }`
  - `pub async fn host_region(&self, region: &str, signer: [u8;32], opts: HostOpts) -> Result<RegionEntry, SdkError>` — pipeline: `predefined().get(region)` (else UnknownRegion) → `geofabrik.download(pbf_url, tmp)` → `md5_file(tmp)` vs `geofabrik.fetch_checksum(pbf_url)` (mismatch ⇒ `SdkError::ChecksumMismatch { expected, actual }` and temp file deleted) → `storage.put` → `geofabrik.current_version` → `registry.register(&[entry])` → returns entry. Timestamp = now unix seconds.
  - `pub async fn host_regions(&self, regions: &[String], signer, opts) -> Result<HostReport, SdkError>` — rejects overlaps up front (`bulk_host_rejects_overlap`: any two regions where one's path is a prefix-dir of the other's — with the frozen set this never fires, but the guard is structural); downloads/verifies/stores all first, then ONE `registry.register(&batch)` transaction; per-region opt-out is the caller filtering `regions`; returns `HostReport { registered: Vec<RegionEntry>, skipped: Vec<(String, SdkError)> }` (a region failing download/verify is skipped with error, does not abort the batch; empty success set ⇒ no tx).
  - `pub async fn import_local(&self, region: &str, file: &Path, signer, opts, checksum_override: Option<String>) -> Result<RegionEntry, SdkError>` — md5 from `checksum_override` or fetched from Geofabrik (import-time verification), then store → register.
  - `pub struct RegionUpdate { pub region: String, pub registry_version: String, pub central_version: String }`; `pub async fn check_updates(&self, signer) -> Result<Vec<RegionUpdate>, SdkError>` — fetch_all + per registered region `current_version` (matched on region path); includes `hosted == false` entries too.
  - `pub struct DownloadReport { pub region: String, pub source: DownloadSource, pub path: PathBuf, pub bytes: u64, pub verified: bool }`, `pub enum DownloadSource { LogosStorage, Geofabrik }`; `pub async fn download_region(&self, region: &str, dest_dir: &Path) -> Result<DownloadReport, SdkError>` — resolve entry: hosted ⇒ `storage.get(cid)` else `geofabrik.download(source_url)` (spec: direct fallback); after download `md5_file` vs entry.checksum → `verified` flag (report includes it; mismatch is still a successful download with `verified: false` — CID-guaranteed integrity is the trust anchor, checksum is evidence).

- [ ] **Step 1: Failing tests** (wiremock serves: pbf bytes, `.md5`, 307 redirect):
  - `host_region_full_pipeline` → entry fields populated, cid from InMemoryStorage, checksum = served md5, version from redirect, `hosted=true`, registry contains it.
  - `host_region_checksum_mismatch_aborts` (served md5 ≠ body hash) → Err ChecksumMismatch, storage did NOT receive bytes, registry empty.
  - `host_regions_batches_into_one_tx` (2 regions → MockRegistryClient.register call count == 1 with 2 entries).
  - `bulk_host_rejects_overlap` (`["us", "us/california"]` — synthetic defs registered into a test-local region table via `#[cfg(test)] pub(crate) fn add_test_region(def)`; assert Err overlap) — plus `bulk_host_skips_failed_region` (one region's pbf 404s → report.skipped has it, other registered).
  - `import_local_uses_override_checksum` (temp file + override → registered without any HTTP hit for md5).
  - `check_updates_detects_newer_central` (registry version `261024`, redirect now points `261025` → one RegionUpdate).
  - `download_prefers_storage_then_falls_back` (hosted=false → source Geofabrik; hosted=true → InMemoryStorage; verified=true in both).
- [ ] **Step 2:** FAIL.
- [ ] **Step 3: Implement** the three modules + `OsmSdk` facade in `lib.rs`.
- [ ] **Step 4:** PASS. (`cargo test -p osm_sdk` green, including Tasks 3–6 tests.)
- [ ] **Step 5: Commit** — `feat(sdk): host/bulk/import/updates/download workflows`

### Task 8: `osm_cli` — command-line interface

**Files:**
- Create: `osm_cli/Cargo.toml`, `osm_cli/src/main.rs`, `osm_cli/src/config.rs`
- Test: `osm_cli/tests/cli_tests.rs` (assert_help/list subcommands; config parse; JSON shapes)

**Interfaces:**
- Consumes: `OsmSdk` full API.
- Produces: binary `osm` with global flags `--config <path>` (TOML: `program_id`, `idl_path`, `cli_bin`, `lgs_bin`, `wallet_home`, `logoscore_bin`, `modules_dir`, `storage_config`, `rpc_url`), env overrides; subcommands:
  - `init-registry`
  - `discover [--json] [--refresh]`
  - `host <REGION>... [--exclude <REGION>]... [--json]`
  - `import <REGION> <FILE> [--md5 <hex>]`
  - `lookup (--region <R> | --parent <P> | --cid <C>) [--json]`
  - `check-updates [--json]`
  - `download <REGION> [--dest <DIR>]`
- Exit codes: 0 ok; 1 usage; 2 checksum mismatch; 3 storage failure after retries; 4 registry error. `--json` output shapes are serde-serialized `RegionEntry`/`HostReport`/`RegionUpdate`/`DownloadReport` (documented in README).

- [ ] **Step 1: Failing test** — `cli_help_lists_all_seven_subcommands` (run binary via `assert_cmd`-style `std::process::Command` on the built bin? Use `clap::Command::debug_assert()` + a unit test calling the clap parser: `parse_from(["osm","discover"])` etc. — plus `config_parses_and_env_overrides`).
- [ ] **Step 2:** FAIL.
- [ ] **Step 3: Implement** main.rs (thin: build config → construct SpelCliRegistryClient + LogoscoreStorage/InMemoryStorage (`--storage memory` flag for dev/test) → OsmSdk → dispatch; human output with hosted status, versions, verification results per spec's UX criterion) + config.rs.
- [ ] **Step 4:** PASS (`cargo test -p osm_cli`; also `cargo run -p osm_cli -- discover --json` locally prints all 72 regions — no network needed).
- [ ] **Step 5: Commit** — `feat(cli): full CLI over the SDK`

### Task 9: Guest program — `methods/guest` SPEL `osm_registry` + IDL

**Files:**
- Create: `methods/guest/Cargo.toml`, `methods/guest/src/bin/osm_registry.rs`, `examples/Cargo.toml`, `examples/src/bin/generate_idl.rs`, `examples/src/bin/osm_registry_cli.rs`, `idl/osm-registry.idl.json` (generated), `Makefile` (build/idl/cli/deploy/inspect targets modeled on reference), `tools/Cargo.toml` + `tools/src/bin/mk-program-binary.rs` (copied from reference — same ELF wrapper)
- Test: guest `#[cfg(test)]` tests (run under `RISC0_DEV_MODE=1` in CI; skip locally when env absent — `if std::env::var("RISC0_DEV_MODE").is_err() { return; }` pattern from reference)

**Interfaces:**
- Consumes: `osm_registry_core::{RegionEntry, Registry, MAX_BATCH, E_*}`.
- Produces (instructions, all taking `#[account(signer)] authority: AccountWithMetadata`):
  - `init_registry(#[account(init, pda = [literal("registry")])] mut registry: AccountWithMetadata, authority)` — writes empty Registry.
  - `register_region(mut registry, authority, region: String, parent: String, level: u8, cid: String, source_url: String, checksum: String, version: String, hosted: bool, timestamp: i64, registered_by: [u8; 32])` — upsert via `validate_entry` (parent empty ⇒ None), cycle-logged.
  - `register_batch(mut registry, authority, payload: String, registered_by: [u8; 32])` — payload = hex borsh `Vec<RegionEntry>`, `1..=MAX_BATCH` entries, upsert each, cycle-logged.
  - `pub const PROGRAM_NAME: &str = "osm_registry";`
- IDL generated by `spel generate-idl methods/guest/src/bin/osm_registry.rs > idl/osm-registry.idl.json` and committed.

- [ ] **Step 1: Implement** the guest (logic lives in core::validate_entry so guest stays thin; guest does borsh decode/encode of Registry, upsert, and the `Data::try_from` ≤100 KiB guard exactly like the reference).
- [ ] **Step 2: Local check** — `cargo check -p osm_registry_guest --target x86_64-unknown-linux-gnu 2>/dev/null || cargo check --manifest-path methods/guest/Cargo.toml` may not link risc0 deps on Windows; the binding local check is `cargo check -p osm_registry_core -p osm_sdk -p osm_cli` plus review. Full guest build happens in CI (`make build`).
- [ ] **Step 3: CI verifies** (Task 12 job): `make build` succeeds, `make idl` output committed matches regenerated (`git diff --exit-code idl/`), guest tests pass under `RISC0_DEV_MODE=1`.
- [ ] **Step 4: Commit** — `feat(guest): osm_registry SPEL program (init/register_region/register_batch) + IDL`

### Task 10: `ffi` — C ABI bridge for the Basecamp module

**Files:**
- Create: `ffi/Cargo.toml` (crate-type cdylib + rlib for tests), `ffi/src/lib.rs`
- Test: `ffi/tests/ffi_tests.rs`

**Interfaces:**
- Consumes: `OsmSdk` (full).
- Produces (all string-JSON in/out, `#[no_mangle] pub extern "C"`, caller frees with `osm_free`):
  - `osm_sdk_version() -> *const c_char`
  - `osm_discover(cfg_json: *const c_char) -> *mut c_char` (cfg JSON = CLI config TOML fields as JSON; returns JSON array of RegionDef)
  - `osm_resolve(cfg_json, region) -> *mut c_char` (registry entry JSON or `null`)
  - `osm_query(cfg_json, kind: *const c_char /*"region"|"parent"|"cid"*/, value) -> *mut c_char`
  - `osm_check_updates(cfg_json) -> *mut c_char`
  - `osm_host(cfg_json, regions_csv) -> *mut c_char` (HostReport JSON)
  - `osm_import(cfg_json, region, file_path, md5_opt) -> *mut c_char`
  - `osm_download(cfg_json, region, dest_dir) -> *mut c_char` (DownloadReport JSON)
  - `osm_free(ptr)`, `osm_last_error() -> *const c_char`
- Each fn returns `null` on error with `osm_last_error()` populated (thread-local message).

- [ ] **Step 1: Failing tests** — `discover_json_roundtrip` (call `osm_discover` with in-memory config → parse JSON → 72 entries; `version_string_ok`).
- [ ] **Step 2:** FAIL.
- [ ] **Step 3: Implement** (cfg JSON → Config → OsmSdk with InMemoryStorage default in `--storage memory` mode or LogoscoreStorage).
- [ ] **Step 4:** PASS.
- [ ] **Step 5: Commit** — `feat(ffi): C ABI over the SDK for Basecamp modules`

### Task 11: Basecamp module (`osm-module`) and app (`osm-app`) + flake

**Files:**
- Create: `osm-module/metadata.json` (`{ "name": "osm", "version": "1.0.0", "type": "core", "main": "osm_plugin", "dependencies": ["storage_module"], ... }` modeled exactly on reference `logos-chronicle/metadata.json`), `osm-module/CMakeLists.txt`, `osm-module/src/osm_plugin.{h,cpp}` (Q_INVOKABLE: `discover()`, `resolve(region)`, `queryBy(kind, value)`, `checkUpdates()`, `hostRegions(QStringList)`, `importRegion(region, fileUrl)`, `downloadRegion(region, destUrl)`; storage via `LogosAPIClient("storage_module", ...)` for in-app downloads of hosted regions; registry/host/import via FFI `dlopen("libosm_ffi.so")` exactly like reference `chronicle_anchor_client.cpp`), `osm-app/metadata.json`, `osm-app/CMakeLists.txt`, `osm-app/src/qml/Main.qml`, `flake.nix` (modeled on reference: `mkLogosModule { src ./osm-module ... postInstall cp FFI .so }`, `mkLogosQmlModule { src ./osm-app ... }`), `Makefile` target `ffi`
- Test: none locally (no Qt/Nix on this machine); CI nix job (Task 12) builds both; QML verified by review against reference Main.qml patterns.

**Interfaces:**
- Consumes: FFI JSON API (Task 10); reference `logos-chronicle` C++ patterns.
- Produces: installable Basecamp module `osm` (core) and QML app `osm-app` exposing the **consumer subset** through the module API: discover / resolve / queryBy / downloadRegion (+ host/import/checkUpdates for the app's own use via FFI). The module does NOT expose host/register to other consumer apps (spec: consumer subset only needs discover+use).

- [ ] **Step 1: Write** `osm-module` (C++ plugin; the upload/download progress events pattern (`storageUploadDone`) copied from reference with retry/backoff constants; FFI client in `osm_ffi_client.{h,cpp}`).
- [ ] **Step 2: Write** `osm-app` QML: `Main.qml` with (a) region ListView grouped by level, columns: name, hosted status, version, verification badge; (b) bulk-host panel with per-region checkboxes + opt-out and a parent/child overlap warning bar; (c) import file dialog (local PBF → FFI import); (d) update banner listing `checkUpdates()` results; (e) download buttons with progress + verification result display.
- [ ] **Step 3: Write** `flake.nix` + Makefile `ffi` target (cp vendored .so like reference).
- [ ] **Step 4: Commit** — `feat(basecamp): osm core module + distribution QML app + nix flake`
- [ ] **Step 5: CI gate (Task 12)** — `nix build .#osm-install .#osm-app-install` must succeed.

### Task 12: CI, e2e test, cycle documentation, module catalog

**Files:**
- Create: `.github/workflows/ci.yml`, `tests/e2e.rs` (in workspace `tests/` crate `osm-e2e`, feature `e2e`), `docs/cycles.md`, `docs/catalog.md` (fork + `logos-repo.json` + release-action instructions), `docs/sdk.md` (worked example: resolve region → CID/metadata)
- Modify: `Makefile` (add `test-e2e`, `cycles` targets)

**Interfaces:**
- Consumes: everything above.
- Produces: green CI on main; `docs/cycles.md` with measured `register_region`/`register_batch` CU numbers from a localnet run (job greps `CU register_*` guest logs).

- [ ] **Step 1: Write e2e test** (`tests/e2e/main.rs`, gated `#[cfg(feature = "e2e")]`): deterministic synthetic 64 KiB "pbf" fixture (fixed seed bytes), local `.md5` fixture (import path — no Geofabrik in CI): `init_registry → import_local(region "e2e-test-region" via test-region registration in a test-local table) → query_by_region → download → md5 assert`. Against a real localnet when `LGS_LOCALNET=1` (SpelCliRegistryClient), in-memory otherwise.
- [ ] **Step 2: Write ci.yml** jobs: `rust` (fmt --check, clippy -D warnings, test workspace on ubuntu + windows), `chain` (docker: `make build`, `make idl` diff check, `lgs setup && lgs localnet start && make deploy`, run e2e with `LGS_LOCALNET=1`, extract CU logs into docs/cycles.md artifact), `nix` (build both module installs), `live-geofabrik` (`workflow_dispatch` only: host smallest real predefined region end-to-end).
- [ ] **Step 3: Write docs** (cycles template, catalog publishing steps incl. exact `logos-repo.json` URL requirement, SDK worked example).
- [ ] **Step 4: Verify locally what's verifiable**: `cargo test --workspace` green on Windows; `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`.
- [ ] **Step 5: Commit** — `ci: guest build + localnet e2e + nix modules + cycle docs`

### Task 13: README, FURPS self-assessment, final polish

**Files:**
- Modify: `README.md` (full: setup, prerequisites, build/deploy steps incl. testnet 0.4 notes, CLI usage walkthrough, app install, SDK worked example, region list table, program ID section, deviation note for ireland/malaysia, license)
- Create: `docs/furps.md` (self-assessment mapping every Success Criteria checkbox to where it's satisfied)

- [ ] **Step 1:** Write docs.
- [ ] **Step 2:** Checklist sweep — walk every LP-0018 success-criteria checkbox and confirm a repo artifact satisfies it; fix gaps found (each fix its own commit).
- [ ] **Step 3:** `cargo test --workspace && cargo clippy --all-targets -- -D warnings && cargo fmt --check` green; commit — `docs: README, FURPS self-assessment`

### Task 14: Save project memory + handoff notes

- [ ] Save memory files (project state, toolchain findings, deviations) so a future session can resume without re-research.
- [ ] Commit any remaining changes.

## Self-Review

- Spec coverage: functionality (Tasks 3–9), usability/CLI/GUI/SDK docs (Tasks 8, 10–13), reliability retries/graceful failures/timestamp ordering/no central services (Tasks 4–7), performance cycle docs (Tasks 9, 12), supportability testnet 0.4 deploy (Task 12 chain job + README), module-builder/catalog/IDL (Tasks 11–12), e2e in CI (Task 12), adoption criteria are operational (hosting real regions, 5 external modules) — documented as post-build actions in README/furps, not code tasks.
- Type consistency: `RegionEntry`/`RegistryClient`/`StorageBackend`/`OsmSdk` signatures used consistently across Tasks 5–10.
- Review Focus items 1–6 each have pinning tests in Tasks 3, 4, 2, 6, 7.
