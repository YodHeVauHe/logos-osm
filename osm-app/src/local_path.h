#ifndef OSM_APP_LOCAL_PATH_H
#define OSM_APP_LOCAL_PATH_H

#include <QString>
#include <QUrl>

namespace local_path {

// QML passes folders and files as URLs (`file:///home/me/Downloads`); the osm
// module and the FFI expect filesystem paths. Turn a local file URL into a
// path and leave plain paths (or anything else) unchanged.
inline QString fromUserInput(const QString& input) {
    const QString trimmed = input.trimmed();
    if (trimmed.startsWith(QStringLiteral("file:"), Qt::CaseInsensitive)) {
        const QUrl url(trimmed);
        if (url.isValid() && url.isLocalFile()) return url.toLocalFile();
    }
    return trimmed;
}

} // namespace local_path

#endif // OSM_APP_LOCAL_PATH_H
