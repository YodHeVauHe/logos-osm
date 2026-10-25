#pragma once

#include <string>
#include "logos_module_context.h"

// OSM registry module — universal authoring model.
//
// The public methods of this class are the module's API (the LIDL contract
// and all glue are generated from this header). All methods are JSON-in /
// JSON-out strings; a response is a JSON object with at minimum
// {"ok": true|false}.
//
// Registry, host, import, and update logic run in the Rust SDK via the
// vendored libosm_ffi.so C ABI (see osm_ffi_client.h).
class OsmImpl : public LogosModuleContext
{
public:
    // ── Consumer subset (what other Basecamp apps need) ─────────────────

    /// The 72 predefined regions: JSON array of RegionDef.
    std::string discover();

    /// Resolve one region to its on-chain entry (CID, version, checksum…),
    /// or JSON `null` when it is not registered yet.
    std::string resolve(const std::string& region);

    /// Query the registry: kind is "region", "parent", or "cid".
    std::string query(const std::string& kind, const std::string& value);

    /// Download a region: by CID from Logos Storage when hosted, else
    /// straight from Geofabrik. Returns a DownloadReport JSON.
    std::string downloadRegion(const std::string& region, const std::string& destDir);

    // ── Distribution face (used by the OSM distribution app) ────────────

    /// Bulk host: regionsCsv is a comma-separated list of region paths.
    /// Fetch → verify MD5 → store → ONE batch-register transaction.
    std::string hostRegions(const std::string& regionsCsv);

    /// Import a local PBF: verify against Geofabrik's published MD5 (or
    /// md5Override), store, register by CID.
    std::string importRegion(const std::string& region, const std::string& filePath,
                             const std::string& md5Override);

    /// Compare Geofabrik's current snapshot versions against the registry.
    std::string checkUpdates();

    // ── Configuration ────────────────────────────────────────────────────

    /// Current FFI config JSON (program id, storage mode, paths…).
    std::string getConfig();

    /// Replace the FFI config JSON; returns the new config.
    std::string setConfig(const std::string& cfgJson);

private:
    class FfiClient;
    std::string callFfi(const char* symbol, int argc, const std::string* args);

    std::string ffiConfig_ = "{}";
};
