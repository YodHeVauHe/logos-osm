#include "osm_impl.h"

#include "osm_ffi_client.h"

// The FFI client is header-only (one instance per module context).
class OsmImpl::FfiClient : public OsmFfiClient
{
};

std::string OsmImpl::callFfi(const char* symbol, int argc, const std::string* args)
{
    static FfiClient client;
    client.setConfig(ffiConfig_);
    std::string err;
    if (!client.load(&err))
        return "{\"ok\":false,\"error\":\"" + err + "\"}";

    // Route through the named method wrappers (which re-check the load).
    std::string r;
    if (std::string(symbol) == "osm_discover")
        r = client.discover();
    else if (std::string(symbol) == "osm_resolve")
        r = client.resolve(args[0]);
    else if (std::string(symbol) == "osm_query")
        r = client.query(args[0], args[1]);
    else if (std::string(symbol) == "osm_check_updates")
        r = client.checkUpdates();
    else if (std::string(symbol) == "osm_host")
        r = client.host(args[0]);
    else if (std::string(symbol) == "osm_import")
        r = client.importRegion(args[0], args[1], args[2]);
    else if (std::string(symbol) == "osm_download")
        r = client.downloadRegion(args[0], args[1]);

    return r.empty() ? "{\"ok\":false,\"error\":\"" + client.lastError() + "\"}" : r;
}

std::string OsmImpl::discover()
{
    return callFfi("osm_discover", 0, nullptr);
}

std::string OsmImpl::resolve(const std::string& region)
{
    const std::string a[] = {region};
    return callFfi("osm_resolve", 1, a);
}

std::string OsmImpl::query(const std::string& kind, const std::string& value)
{
    const std::string a[] = {kind, value};
    return callFfi("osm_query", 2, a);
}

std::string OsmImpl::downloadRegion(const std::string& region, const std::string& destDir)
{
    const std::string a[] = {region, destDir};
    return callFfi("osm_download", 2, a);
}

std::string OsmImpl::hostRegions(const std::string& regionsCsv)
{
    const std::string a[] = {regionsCsv};
    return callFfi("osm_host", 1, a);
}

std::string OsmImpl::importRegion(const std::string& region, const std::string& filePath,
                                  const std::string& md5Override)
{
    const std::string a[] = {region, filePath, md5Override};
    return callFfi("osm_import", 3, a);
}

std::string OsmImpl::checkUpdates()
{
    return callFfi("osm_check_updates", 0, nullptr);
}

std::string OsmImpl::getConfig()
{
    return "{\"ok\":true,\"value\":" + ffiConfig_ + "}";
}

std::string OsmImpl::setConfig(const std::string& cfgJson)
{
    ffiConfig_ = cfgJson;
    return getConfig();
}
