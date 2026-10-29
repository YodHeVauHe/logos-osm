#ifndef OSM_APP_PLUGIN_H
#define OSM_APP_PLUGIN_H

#include <QObject>
#include <QJsonValue>
#include <QString>

#include "osm_app_interface.h"

class LogosAPI;
class LogosObject;

// UI-facing bridge for the OSM distribution app. State is exposed as Qt
// Remote Object properties; the QML view binds to them. All heavy lifting
// lives in the osm core module (which fronts the Rust SDK via FFI).
class OsmAppPlugin : public QObject, public OsmAppInterface
{
    Q_OBJECT
    Q_PLUGIN_METADATA(IID OsmAppInterface_iid FILE "metadata.json")
    Q_INTERFACES(OsmAppInterface PluginInterface)

public:
    OsmAppPlugin();
    ~OsmAppPlugin() override;

    QString name()    const override { return QStringLiteral("osm-app"); }
    QString version() const override { return QStringLiteral("1.0.0"); }

    // RO properties (rep file)
    QString regions() const { return m_regions; }
    void setRegions(const QString& v);
    QString registryEntries() const { return m_registryEntries; }
    void setRegistryEntries(const QString& v);
    QString updates() const { return m_updates; }
    void setUpdates(const QString& v);
    QString phase() const { return m_phase; }
    void setPhase(const QString& v);
    QString status() const { return m_status; }
    void setStatus(const QString& v);
    bool working() const { return m_working; }
    void setWorking(bool v);
    QString lastVerification() const { return m_lastVerification; }
    void setLastVerification(const QString& v);

    // RO slots
    Q_INVOKABLE void hostRegions(const QString& regionsCsv);
    Q_INVOKABLE void importRegion(const QString& region, const QString& filePath,
                                  const QString& md5Override);
    Q_INVOKABLE void downloadRegion(const QString& region, const QString& destDir);
    Q_INVOKABLE void refresh();
    Q_INVOKABLE void checkUpdates();

signals:
    void regionsChanged(const QString&);
    void registryEntriesChanged(const QString&);
    void updatesChanged(const QString&);
    void phaseChanged(const QString&);
    void statusChanged(const QString&);
    void workingChanged(bool);
    void lastVerificationChanged(const QString&);

private:
    QJsonValue callModule(const char* method, const QJsonArray& args, QString* error = nullptr);

    QString m_regions = QStringLiteral("[]");
    QString m_registryEntries = QStringLiteral("[]");
    QString m_updates = QStringLiteral("[]");
    QString m_phase = QStringLiteral("idle");
    QString m_status;
    bool m_working = false;
    QString m_lastVerification;

};

#endif // OSM_APP_PLUGIN_H
