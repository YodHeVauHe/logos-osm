//! The frozen predefined region set and Geofabrik index resolution.
//!
//! `regions.json` at the repository root is the closed, non-overlapping set
//! reconstructed verbatim from the LP-0018 prize document (72 regions: 48
//! countries, 24 decomposed leaves). It is embedded at compile time.

use crate::error::SdkError;
use serde::{Deserialize, Serialize};

pub const REGIONS_JSON: &str = include_str!("../../regions.json");

/// One predefined region. `level` uses the registry encoding
/// (`0` = country, `1` = subregion) so it round-trips into a
/// [`osm_registry_core::RegionEntry`] unchanged.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegionDef {
    /// Geofabrik path as frozen in the prize document (registry key).
    pub region: String,
    /// Decomposed parent or `null` for country-level entries.
    pub parent: Option<String>,
    /// `0` = country, `1` = subregion.
    pub level: u8,
    /// Human-readable display name.
    pub name: String,
    /// The id this region carries in the Geofabrik index. Differs from
    /// `region` for the decomposed countries (index ids have no country
    /// prefix) and for `ireland` / `malaysia` (combined extracts).
    pub index_id: String,
    /// Continent path segment used by Geofabrik URLs.
    pub continent: String,
    /// Canonical `*-latest.osm.pbf` URL (parent-chain based, not id based).
    pub pbf_url: String,
}

/// One entry of the live Geofabrik index (`index-v1-nogeom.json`).
#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub id: String,
    pub parent: Option<String>,
    pub name: String,
    pub pbf_url: Option<String>,
}

pub fn predefined() -> &'static [RegionDef] {
    use std::sync::OnceLock;
    static REGIONS: OnceLock<Vec<RegionDef>> = OnceLock::new();
    REGIONS.get_or_init(|| {
        serde_json::from_str(REGIONS_JSON).expect("regions.json is valid and committed")
    })
}

pub fn get(region: &str) -> Option<&'static RegionDef> {
    predefined().iter().find(|r| r.region == region)
}

/// The four decomposed countries. They are `parent` values only — they are
/// deliberately NOT regions of the set.
pub fn decomposed_parents() -> [&'static str; 4] {
    ["us", "india", "china", "russia"]
}

/// Parse the Geofabrik index (GeoJSON FeatureCollection, no geometry).
/// Entries without a `urls.pbf` are skipped — they cannot be hosted.
pub fn parse_index(json: &str) -> Result<Vec<IndexEntry>, SdkError> {
    let v: serde_json::Value = serde_json::from_str(json)?;
    let features = v
        .get("features")
        .and_then(|f| f.as_array())
        .ok_or_else(|| SdkError::IndexMismatch("index has no features array".into()))?;
    let mut out = Vec::with_capacity(features.len());
    for f in features {
        let p = match f.get("properties") {
            Some(p) => p,
            None => continue,
        };
        let id = match p.get("id").and_then(|s| s.as_str()) {
            Some(id) => id.to_string(),
            None => continue,
        };
        let parent = p
            .get("parent")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());
        let name = p
            .get("name")
            .and_then(|s| s.as_str())
            .unwrap_or(&id)
            .to_string();
        let pbf_url = p
            .get("urls")
            .and_then(|u| u.get("pbf"))
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());
        out.push(IndexEntry { id, parent, name, pbf_url });
    }
    Ok(out)
}

/// Rebuild the region set from a live index fetch, taking Geofabrik's own
/// `urls.pbf` as the source of truth for every download URL.
///
/// Resolution rules (matching how the set was frozen):
/// - US states: index id equals `region` (composite `us/<state>`).
/// - India zones / China provinces / Russia districts: the index id has no
///   country prefix; match on `(id == leaf, parent == decomposed country)`.
/// - Countries: match on `id == region`, falling back to `index_id` for the
///   two combined extracts (`ireland`, `malaysia`).
pub fn refresh_from_index(json: &str) -> Result<Vec<RegionDef>, SdkError> {
    let index = parse_index(json)?;
    let mut out = Vec::with_capacity(predefined().len());
    for def in predefined() {
        let parent = def.parent.as_deref();
        let found = index
            .iter()
            .find(|e| match (def.level, parent) {
                (1, Some(p)) => e.id == def.index_id && e.parent.as_deref() == Some(p),
                _ => e.id == def.region || e.id == def.index_id,
            })
            .ok_or_else(|| SdkError::IndexMismatch(def.region.clone()))?;
        let pbf_url = found
            .pbf_url
            .clone()
            .ok_or_else(|| SdkError::IndexMismatch(format!("{} (no pbf url)", def.region)))?;
        let mut def = def.clone();
        def.pbf_url = pbf_url;
        out.push(def);
    }
    Ok(out)
}

/// Geofabrik checksum files sit next to the archive: `<pbf>.md5`.
pub fn md5_url(pbf_url: &str) -> String {
    format!("{pbf_url}.md5")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predefined_has_72_regions() {
        assert_eq!(predefined().len(), 72);
        assert_eq!(predefined().iter().filter(|r| r.level == 0).count(), 48);
        assert_eq!(predefined().iter().filter(|r| r.level == 1).count(), 24);
    }

    #[test]
    fn set_is_non_overlapping() {
        // The decomposed countries are parents only, never members.
        for p in decomposed_parents() {
            assert!(get(p).is_none(), "{p} must not be a member");
        }
        // No member's path is a directory-prefix of another member's path.
        for a in predefined() {
            for b in predefined() {
                if a.region != b.region && b.region.starts_with(&format!("{}/", a.region)) {
                    panic!("{} contains {}", a.region, b.region);
                }
            }
        }
    }

    #[test]
    fn url_is_parent_chain() {
        for r in predefined() {
            assert!(
                r.pbf_url.starts_with(&format!("https://download.geofabrik.de/{}/", r.continent)),
                "{} -> {}",
                r.region,
                r.pbf_url
            );
            assert!(r.pbf_url.ends_with("-latest.osm.pbf"));
        }
        assert!(get("india/central-zone").unwrap().pbf_url.contains("/asia/india/central-zone-"));
        assert!(get("russia/central-fed-district").unwrap().pbf_url.contains("/russia/central-fed-district-"));
        assert!(get("us/california").unwrap().pbf_url.contains("/north-america/us/california-"));
    }

    #[test]
    fn mapped_regions_resolve() {
        assert!(get("ireland").is_some());
        assert!(get("ireland").unwrap().index_id == "ireland-and-northern-ireland");
        assert!(get("ireland").unwrap().pbf_url.contains("ireland-and-northern-ireland-latest.osm.pbf"));
        assert!(get("malaysia").unwrap().pbf_url.contains("malaysia-singapore-brunei-latest.osm.pbf"));
    }

    #[test]
    fn md5_url_appends_suffix() {
        assert_eq!(
            md5_url("https://download.geofabrik.de/europe/germany-latest.osm.pbf"),
            "https://download.geofabrik.de/europe/germany-latest.osm.pbf.md5"
        );
    }

    #[test]
    fn parse_index_reads_sample() {
        let idx = parse_index(crate::tests_fixture::INDEX_SAMPLE).unwrap();
        let cal = idx.iter().find(|e| e.id == "us/california").unwrap();
        assert_eq!(
            cal.pbf_url.as_deref(),
            Some("https://download.geofabrik.de/north-america/us/california-latest.osm.pbf")
        );
        let cz = idx.iter().find(|e| e.id == "central-zone").unwrap();
        assert_eq!(cz.parent.as_deref(), Some("india"));
        // Entries without a pbf url are skipped.
        assert!(idx.iter().all(|e| e.pbf_url.is_some()));
    }
}

/// Test fixtures shared by unit and integration tests.
pub(crate) mod tests_fixture {
    pub const INDEX_SAMPLE: &str = include_str!("../tests/fixtures/geofabrik-index-sample.json");
}
