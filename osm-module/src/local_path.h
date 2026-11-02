// Turn a destination that may be a local file URL into a filesystem path.
//
// QML hands folders around as URLs (`file:///home/me/Downloads`), and other
// Basecamp apps may do the same. The FFI expects a path; given a URL it would
// create `./file:/home/...` under the working directory instead.

#pragma once

#include <string>

namespace local_path {

inline int hexValue(char c)
{
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

inline std::string percentDecode(const std::string& in)
{
    std::string out;
    out.reserve(in.size());
    for (std::size_t i = 0; i < in.size(); ++i)
    {
        if (in[i] == '%' && i + 2 < in.size())
        {
            const int hi = hexValue(in[i + 1]);
            const int lo = hexValue(in[i + 2]);
            if (hi >= 0 && lo >= 0)
            {
                out += static_cast<char>(hi * 16 + lo);
                i += 2;
                continue;
            }
        }
        out += in[i];
    }
    return out;
}

/// `file:///a/b%20c` and `file://localhost/a/b` become `/a/b c` and `/a/b`.
/// Anything that is not a local `file:` URL (a plain path, a relative path,
/// an empty string) is returned unchanged.
inline std::string fromUserInput(const std::string& input)
{
    static const std::string scheme = "file:";
    if (input.size() < scheme.size()) return input;
    for (std::size_t i = 0; i < scheme.size(); ++i)
    {
        const char c = input[i];
        const char lower = (c >= 'A' && c <= 'Z') ? static_cast<char>(c - 'A' + 'a') : c;
        if (lower != scheme[i]) return input;
    }
    std::string rest = input.substr(scheme.size());
    if (rest.rfind("//", 0) == 0)
    {
        rest = rest.substr(2);
        const std::size_t slash = rest.find('/');
        const std::string host = slash == std::string::npos ? rest : rest.substr(0, slash);
        if (!host.empty() && host != "localhost") return input; // remote host: not a local path
        rest = slash == std::string::npos ? std::string("/") : rest.substr(slash);
    }
    if (rest.empty() || rest[0] != '/') return input;
    return percentDecode(rest);
}

} // namespace local_path
