// JSON helpers for the osm module's hand-built replies. Qt-free so the core
// module keeps its std-only sources.

#pragma once

#include <cstdio>
#include <string>

namespace osm_json {

/// Escape `text` for use inside a JSON string literal (RFC 8259): quotes,
/// backslashes and all control characters. Bytes >= 0x80 pass through, so
/// UTF-8 text stays UTF-8.
inline std::string escape(const std::string& text)
{
    std::string out;
    out.reserve(text.size() + 8);
    for (const char ch : text)
    {
        const unsigned char c = static_cast<unsigned char>(ch);
        switch (c)
        {
            case '"': out += "\\\""; break;
            case '\\': out += "\\\\"; break;
            case '\b': out += "\\b"; break;
            case '\f': out += "\\f"; break;
            case '\n': out += "\\n"; break;
            case '\r': out += "\\r"; break;
            case '\t': out += "\\t"; break;
            default:
                if (c < 0x20)
                {
                    char buf[7];
                    std::snprintf(buf, sizeof buf, "\\u%04x", c);
                    out += buf;
                }
                else
                {
                    out += ch;
                }
        }
    }
    return out;
}

/// `{"ok":false,"error":"<message>"}` with the message escaped.
inline std::string errorReply(const std::string& message)
{
    return "{\"ok\":false,\"error\":\"" + escape(message) + "\"}";
}

} // namespace osm_json
