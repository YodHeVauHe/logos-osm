#ifndef OSM_INTERFACE_H
#define OSM_INTERFACE_H

#include <QObject>
#include <QString>

#include "interface.h"

// Qt plugin interface for the OSM registry module.
// All methods are JSON-in / JSON-out — callers parse the returned QString as
// a JSON object containing at minimum {"ok": true|false}.
//
// Consumer subset (discover / resolve / query / download) is what other
// Basecamp apps need to use map data; host/import/updates are for the
// distribution app.
class OsmInterface : public PluginInterface
{
public:
    virtual ~OsmInterface() = default;

    // Consumer subset
    Q_INVOKABLE virtual QString discoverJson() = 0;
    Q_INVOKABLE virtual QString resolveJson(const QString& region) = 0;
    Q_INVOKABLE virtual QString queryJson(const QString& kind, const QString& value) = 0;
    Q_INVOKABLE virtual QString downloadRegionJson(const QString& region,
                                                   const QString& destDir) = 0;

    // Distribution face (used by the OSM app)
    Q_INVOKABLE virtual QString hostRegionsJson(const QString& regionsCsv) = 0;
    Q_INVOKABLE virtual QString importRegionJson(const QString& region,
                                                 const QString& filePath,
                                                 const QString& md5Override) = 0;
    Q_INVOKABLE virtual QString checkUpdatesJson() = 0;

    // Configuration (program id, storage mode, signer...)
    Q_INVOKABLE virtual QString getConfigJson() = 0;
    Q_INVOKABLE virtual QString setConfigJson(const QString& cfgJson) = 0;
};

#define OsmInterface_iid "org.logos.OsmInterface/1"
Q_DECLARE_INTERFACE(OsmInterface, OsmInterface_iid)

#endif // OSM_INTERFACE_H
