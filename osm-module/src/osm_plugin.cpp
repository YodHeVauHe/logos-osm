#include "osm_plugin.h"

#include <QJsonDocument>
#include <QJsonObject>
#include <QStringList>

#include "logos_api.h"

namespace {
constexpr int kDownloadTimeoutMs = 10 * 60 * 1000; // PBF files are large

QString compact(const QJsonObject& obj) {
    return QString::fromUtf8(QJsonDocument(obj).toJson(QJsonDocument::Compact));
}
} // namespace

OsmPlugin::OsmPlugin() = default;

OsmPlugin::~OsmPlugin() = default;

void OsmPlugin::initLogos(LogosAPI* api) {
    m_api = api;
    // Headless-friendly token seeding, mirroring the reference module.
    if (auto* tm = m_api->getTokenManager()) {
        tm->seedToken(QStringLiteral("storage_module"),
                      QStringLiteral("osm_headless_storage_token_v1"));
    }
    QString err;
    m_ffi.load(&err);
}

QString OsmPlugin::okJson(const QString& valueJson) {
    return compact(QJsonObject{
        {QStringLiteral("ok"), true},
        {QStringLiteral("value"), QJsonDocument::fromJson(valueJson.toUtf8()).object().isEmpty()
             ? valueJson
             : QJsonValue::fromVariant(QJsonDocument::fromJson(valueJson.toUtf8()).toVariant())},
    });
}

QString OsmPlugin::errJson(const QString& error) {
    return compact(QJsonObject{
        {QStringLiteral("ok"), false},
        {QStringLiteral("error"), error},
    });
}

QString OsmPlugin::discoverJson() {
    const QString r = m_ffi.discover();
    return r.isEmpty() ? errJson(m_ffi.lastError()) : r;
}

QString OsmPlugin::resolveJson(const QString& region) {
    const QString r = m_ffi.resolve(region);
    return r.isEmpty() ? errJson(m_ffi.lastError()) : r;
}

QString OsmPlugin::queryJson(const QString& kind, const QString& value) {
    const QString r = m_ffi.query(kind, value);
    return r.isEmpty() ? errJson(m_ffi.lastError()) : r;
}

QString OsmPlugin::downloadRegionJson(const QString& region, const QString& destDir) {
    const QString r = m_ffi.downloadRegion(region, destDir);
    return r.isEmpty() ? errJson(m_ffi.lastError()) : r;
}

QString OsmPlugin::hostRegionsJson(const QString& regionsCsv) {
    const QString r = m_ffi.host(regionsCsv);
    return r.isEmpty() ? errJson(m_ffi.lastError()) : r;
}

QString OsmPlugin::importRegionJson(const QString& region, const QString& filePath,
                                    const QString& md5Override) {
    const QString r = m_ffi.importRegion(region, filePath, md5Override);
    return r.isEmpty() ? errJson(m_ffi.lastError()) : r;
}

QString OsmPlugin::checkUpdatesJson() {
    const QString r = m_ffi.checkUpdates();
    return r.isEmpty() ? errJson(m_ffi.lastError()) : r;
}

QString OsmPlugin::getConfigJson() {
    return compact(QJsonObject{
        {QStringLiteral("ok"), true},
        {QStringLiteral("config"), QJsonDocument::fromJson(m_ffi.config().toUtf8()).object()},
    });
}

QString OsmPlugin::setConfigJson(const QString& cfgJson) {
    m_ffi.setConfig(cfgJson);
    return getConfigJson();
}
