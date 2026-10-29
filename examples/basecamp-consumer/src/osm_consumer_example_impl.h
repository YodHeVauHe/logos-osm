#pragma once

#include <string>

#include "logos_module_context.h"

// Minimal Basecamp consumer: the generated typed dependency client comes
// from metadata.json's required "osm" module dependency.
class OsmConsumerExampleImpl : public LogosModuleContext
{
public:
    /// Return the OSM registry entry JSON for a predefined region, or null.
    std::string resolveRegion(const std::string& region);

    /// Download a hosted region by CID, or fall back to Geofabrik.
    std::string downloadRegion(const std::string& region, const std::string& destDir);
};
