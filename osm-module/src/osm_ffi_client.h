#ifndef OSM_FFI_CLIENT_H
#define OSM_FFI_CLIENT_H

#include <QObject>
#include <QString>

// dlopen-backed client for libosm_ffi.so (Rust osm_ffi cdylib).
// All calls are JSON-in / JSON-out; a null return means failure and
// lastError() carries the message.
class FfiClient : public QObject
{
    Q_OBJECT
public:
    explicit FfiClient(QObject* parent = nullptr);
    ~FfiClient() override;

    bool load(QString* err = nullptr);

    QString discover();
    QString resolve(const QString& region);
    QString query(const QString& kind, const QString& value);
    QString checkUpdates();
    QString host(const QString& regionsCsv);
    QString importRegion(const QString& region, const QString& filePath, const QString& md5Override);
    QString downloadRegion(const QString& region, const QString& destDir);

    // Shared configuration JSON passed to every call.
    void setConfig(const QString& cfgJson) { m_cfg = cfgJson; }
    QString config() const { return m_cfg; }

    QString lastError() const { return m_lastError; }

private:
    QString call(const char* symbol, int argc, const QString* args);

    QString m_cfg = QStringLiteral("{}");
    QString m_lastError;
    bool m_loaded = false;
};

#endif // OSM_FFI_CLIENT_H
