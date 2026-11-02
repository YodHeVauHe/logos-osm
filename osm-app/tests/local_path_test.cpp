#include "../src/local_path.h"

#include <cassert>

int main() {
    using local_path::fromUserInput;
    // What Main.qml passes: StandardPaths.writableLocation(DownloadLocation).
    assert(fromUserInput(QStringLiteral("file:///home/box/Downloads")) ==
           QStringLiteral("/home/box/Downloads"));
    assert(fromUserInput(QStringLiteral("file:///home/box/My%20Maps")) ==
           QStringLiteral("/home/box/My Maps"));
    assert(fromUserInput(QStringLiteral("FILE:///tmp")) == QStringLiteral("/tmp"));
    // Plain paths pass through.
    assert(fromUserInput(QStringLiteral("/home/box/Downloads")) ==
           QStringLiteral("/home/box/Downloads"));
    assert(fromUserInput(QStringLiteral("  /tmp/x  ")) == QStringLiteral("/tmp/x"));
    assert(fromUserInput(QStringLiteral("relative/dir")) == QStringLiteral("relative/dir"));
    assert(fromUserInput(QString()).isEmpty());
    // Not a local file URL: unchanged.
    assert(fromUserInput(QStringLiteral("https://example.com/x")) ==
           QStringLiteral("https://example.com/x"));
    return 0;
}
