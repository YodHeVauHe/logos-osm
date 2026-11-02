// Std-only tests for the osm module's reply escaping and path handling.
// Build: g++ -std=c++17 -I../src osm_module_util_test.cpp && ./a.out

#include "../src/local_path.h"
#include "../src/osm_json.h"

#include <cassert>
#include <string>

int main()
{
    // Plain text is unchanged.
    assert(osm_json::errorReply("storage offline") == R"({"ok":false,"error":"storage offline"})");

    // The 1.0.1 Download failure: a Storage error that quotes JSON broke the reply.
    const std::string storageError =
        R"(logosctl call failed: {"status":"error","message":"cannot open: file:///home/box/Downloads/x.osm.pbf"})";
    assert(osm_json::errorReply(storageError) ==
           R"({"ok":false,"error":"logosctl call failed: {\"status\":\"error\",\"message\":\"cannot open: file:///home/box/Downloads/x.osm.pbf\"}"})");

    // Backslashes, newlines, tabs and other control characters.
    assert(osm_json::escape("a\\b") == R"(a\\b)");
    assert(osm_json::escape("line1\nline2\r\tx") == R"(line1\nline2\r\tx)");
    assert(osm_json::escape(std::string("\x01\x1f", 2)) == R"(\u0001\u001f)");
    assert(osm_json::escape("\b\f") == R"(\b\f)");

    // UTF-8 passes through untouched.
    assert(osm_json::escape("Zürich – 東京") == "Zürich – 東京");

    // Local file URLs become paths.
    assert(local_path::fromUserInput("file:///home/box/Downloads") == "/home/box/Downloads");
    assert(local_path::fromUserInput("file:///home/box/My%20Maps") == "/home/box/My Maps");
    assert(local_path::fromUserInput("FILE:///tmp") == "/tmp");
    assert(local_path::fromUserInput("file://localhost/tmp/x") == "/tmp/x");
    assert(local_path::fromUserInput("file:/tmp/x") == "/tmp/x");
    assert(local_path::fromUserInput("file:///tmp/100%") == "/tmp/100%");

    // Paths and anything else pass through unchanged.
    assert(local_path::fromUserInput("/home/box/Downloads") == "/home/box/Downloads");
    assert(local_path::fromUserInput("relative/dir") == "relative/dir");
    assert(local_path::fromUserInput("") == "");
    assert(local_path::fromUserInput("file://server/share") == "file://server/share");
    assert(local_path::fromUserInput("https://example.com/x") == "https://example.com/x");
    return 0;
}
