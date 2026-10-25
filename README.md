# logos-osm — OpenStreetMap Integration for Logos (LP-0018)

A decentralized OpenStreetMap data distribution system for the Logos ecosystem:
browse available map regions, host Geofabrik PBF snapshots in Logos Storage, and
download map files whose integrity is guaranteed by content-addressing (CID),
with import-time MD5 verification against Geofabrik.

Submission for [LP-0018](docs/LP-0018.md) (OpenStreetMap Integration —
Decentralized Map Data Distribution). Dual licensed under
[MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).

## Components

| Path | Purpose |
|---|---|
| `osm_registry_core/` | Shared on-chain/off-chain types (`RegionEntry`, `Registry`, error codes) |
| `methods/guest/` | On-chain OSM registry — SPEL/LEZ RISC0 guest program |
| `osm_sdk/` | Registry SDK: region discovery, Geofabrik index/checksums, storage backends, host/download workflows |
| `osm_cli/` | CLI: `discover`, `host`, `import`, `lookup`, `check-updates`, `download` |
| `ffi/` | C ABI bridge over the SDK for Basecamp modules |
| `osm-module/` | Basecamp core module (C++/Qt) |
| `osm-app/` | OSM distribution Basecamp app (QML) |
| `regions.json` | The frozen, non-overlapping 72-region predefined set |

## The predefined region set

LP-0018 measures coverage against a **closed, non-overlapping set of 72
regions**: 48 countries at country level plus 24 subregions for the four
decomposed countries (US states, India zones, China provinces, Russia federal
districts). No region in the set contains another, so no geography is ever
hosted or counted twice. The set is reconstructed verbatim from the prize
document in [`regions.json`](regions.json).

> **Spec deviation note.** Two country-level members of the frozen set have no
> standalone Geofabrik extract: `ireland` (Geofabrik publishes only
> `ireland-and-northern-ireland`) and `malaysia` (only
> `malaysia-singapore-brunei`). Registry keys stay `ireland` and `malaysia`
> exactly as the spec freezes them; checksum and download URLs resolve through
> the mapped combined extract. This is disclosed here and in the SDK docs.

## Status

Work in progress — see [docs/plan.md](docs/plan.md).

## License

`MIT OR Apache-2.0` (dual licensed, per LP-0018 eligibility).
