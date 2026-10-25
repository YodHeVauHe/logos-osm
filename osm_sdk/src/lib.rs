//! OSM registry SDK for Logos Basecamp (LP-0018).
//!
//! One crate, two faces:
//! - **Consumer face** (`regions`, `registry` queries, `download`): discover
//!   regions, resolve a region to its CID/metadata, and download hosted or
//!   fallback files — the subset other Basecamp apps need.
//! - **Distribution face** (`host`, `updates`): the full fetch → verify →
//!   store → register pipeline used by the distribution app and CLI.
//!
//! The on-chain registry is accessed through [`registry::RegistryClient`]
//! implementations (SPEL CLI in production, mock in tests); bytes are stored
//! through [`storage::StorageBackend`] implementations (logoscore binary in
//! production, in-memory in tests).

pub mod error;
pub mod geofabrik;
pub mod regions;
pub mod storage;

pub mod host;
pub mod registry;

pub use error::SdkError;
pub use host::{DownloadReport, DownloadSource, HostOpts, HostReport, OsmSdk, RegionUpdate};
