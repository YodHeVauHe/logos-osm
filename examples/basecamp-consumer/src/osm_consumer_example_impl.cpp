#include "osm_consumer_example_impl.h"

#include <cstdio>

#include "logos_sdk.h"

namespace {

// A storage download of a large region can take minutes. The generated
// client's default RPC timeout is 20 s, which made downloadRegion fail with
// RPC_FAILED while osm was still downloading, so give long calls room.
constexpr int kResolveTimeoutMs = 60 * 1000;
constexpr int kDownloadTimeoutMs = 15 * 60 * 1000;

std::string escapeJson(const std::string& in)
{
    std::string out;
    out.reserve(in.size() + 8);
    for (unsigned char c : in) {
        switch (c) {
        case '"': out += "\\\""; break;
        case '\\': out += "\\\\"; break;
        case '\n': out += "\\n"; break;
        case '\r': out += "\\r"; break;
        case '\t': out += "\\t"; break;
        default:
            if (c < 0x20) {
                char buf[8];
                std::snprintf(buf, sizeof(buf), "\\u%04x", c);
                out += buf;
            } else {
                out += static_cast<char>(c);
            }
        }
    }
    return out;
}

// The generated client returns an empty string when the call itself fails
// (timeout, module not loaded). Report that as an osm-style error object.
std::string orError(const std::string& result, const logos::CallError& err,
                    const char* method)
{
    if (err.ok() && !result.empty())
        return result;
    std::string msg = std::string("osm.") + method + " failed";
    if (!err.code.empty())
        msg += " (" + err.code + ")";
    if (!err.message.empty())
        msg += ": " + err.message;
    return "{\"ok\":false,\"error\":\"" + escapeJson(msg) + "\"}";
}

} // namespace

std::string OsmConsumerExampleImpl::resolveRegion(const std::string& region)
{
    logos::CallError err;
    const std::string r = modules().osm.resolve(region, &err, kResolveTimeoutMs);
    return orError(r, err, "resolve");
}

std::string OsmConsumerExampleImpl::downloadRegion(const std::string& region,
                                                   const std::string& destDir)
{
    logos::CallError err;
    const std::string r =
        modules().osm.downloadRegion(region, destDir, &err, kDownloadTimeoutMs);
    return orError(r, err, "downloadRegion");
}
