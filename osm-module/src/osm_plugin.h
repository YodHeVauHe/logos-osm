#ifndef OSM_PLUGIN_H
#define OSM_PLUGIN_H

#include <QObject>
#include <QString>

#include "osm_ffi_client.h"
#include "osm_interface.h"

class LogosAPIClient;
class LogosAPI;

// The OSM registry core module. Registry/host/import/updates logic runs in
// the Rust FFI cdylib; hosted-file downloads go straight to storage_module
// over Qt Remote Objects so the GUI-independent download path uses the same
// module the storage app hosts.
class OsmPlugin : public QObject, public OsmInterface
{
    Q_OBJECT
    Q_PLUGIN_METADATA(IID OsmInterface_iid FILE "metadata.json")
    Q_INTERFACES(OsmInterface PluginInterface)

public:
    OsmPlugin();
    ~OsmPlugin() override;

    QString name()    const override { return QStringLiteral("osm"); }
    QString version() const override { return QStringLiteral("1.0.0"); }

    // Consumer subset
    QString discoverJson() override;
    QString resolveJson(const QString& region) override;
    QString queryJson(const QString& kind, const QString& value) override;
    QString downloadRegionJson(const QString& region, const QString& destDir) override;

    // Distribution face
    QString hostRegionsJson(const QString& regionsCsv) override;
    QString importRegionJson(const QString& region, const QString& filePath,
                             const QString& md5Override) override;
    QString checkUpdatesJson() override;

    // Config
    QString getConfigJson() override;
    QString setConfigJson(const QString& cfgJson) override;

    // PluginInterface lifecycle (from interface.h)
    void initLogos(LogosAPI* api) override;

private:
    static QString okJson(const QString& valueJson);
    static QString errJson(const QString& error);

    FfiClient m_ffi;
    LogosAPI* m_api = nullptr;
    LogosAPIClient* m_storageClient = nullptr;
};

#endif // OSM_PLUGIN_H
