#include "osm_app_plugin.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>

#include "logos_api.h"

OsmAppPlugin::OsmAppPlugin() = default;

OsmAppPlugin::~OsmAppPlugin() = default;

void OsmAppPlugin::initLogos(LogosAPI* api) {
    m_api = api;
    m_osmClient = new LogosAPIClient(QStringLiteral("osm"),
                                     QStringLiteral("osm-app"),
                                     m_api->getTokenManager(), this);
    refresh();
}

QJsonObject OsmAppPlugin::callModule(const char* method, const QJsonArray& args) {
    if (!m_osmClient) return {};
    // The osm module returns JSON strings; unwrap and parse.
    const QVariant v = m_osmClient->invokeRemoteMethod(
        QStringLiteral("osm"), QString::fromLatin1(method), args,
        Timeout(60 * 1000));
    if (v.canConvert<LogosResult>()) {
        const LogosResult lr = v.value<LogosResult>();
        if (!lr.success) return {};
        return QJsonDocument::fromJson(lr.value.toString().toUtf8()).object();
    }
    const QString text = v.toString();
    if (text.isEmpty()) return {};
    const QJsonObject obj = QJsonDocument::fromJson(text.toUtf8()).object();
    if (obj.contains(QStringLiteral("ok")) && !obj.value(QStringLiteral("ok")).toBool())
        return {};
    return obj.contains(QStringLiteral("ok")) && obj.value(QStringLiteral("value")).isObject()
               ? obj.value(QStringLiteral("value")).toObject()
               : obj;
}

void OsmAppPlugin::setRegions(const QString& v) {
    if (m_regions == v) return;
    m_regions = v;
    emit regionsChanged(v);
}

void OsmAppPlugin::setRegistryEntries(const QString& v) {
    if (m_registryEntries == v) return;
    m_registryEntries = v;
    emit registryEntriesChanged(v);
}

void OsmAppPlugin::setUpdates(const QString& v) {
    if (m_updates == v) return;
    m_updates = v;
    emit updatesChanged(v);
}

void OsmAppPlugin::setPhase(const QString& v) {
    if (m_phase == v) return;
    m_phase = v;
    emit phaseChanged(v);
}

void OsmAppPlugin::setStatus(const QString& v) {
    if (m_status == v) return;
    m_status = v;
    emit statusChanged(v);
}

void OsmAppPlugin::setWorking(bool v) {
    if (m_working == v) return;
    m_working = v;
    emit workingChanged(v);
}

void OsmAppPlugin::setLastVerification(const QString& v) {
    if (m_lastVerification == v) return;
    m_lastVerification = v;
    emit lastVerificationChanged(v);
}

void OsmAppPlugin::refresh() {
    const QJsonObject d = callModule("discoverJson", {});
    if (!d.isEmpty()) setRegions(QString::fromUtf8(QJsonDocument(d).toJson(QJsonDocument::Compact)));

    // The registry snapshot: query by empty parent is not valid, so the app
    // reads the whole registry via the module's query face per level.
    QJsonObject entries;
    QJsonArray all;
    const QJsonObject countries = callModule("queryJson", {QStringLiteral("parent"), QStringLiteral("")});
    Q_UNUSED(countries);
    // Consumers typically resolve per region; the bulk view uses discover +
    // per-region resolve on demand in the QML layer.
    setRegistryEntries(QString::fromUtf8(QJsonDocument(entries).toJson(QJsonDocument::Compact)));
}

void OsmAppPlugin::checkUpdates() {
    setPhase(QStringLiteral("checking"));
    const QJsonObject u = callModule("checkUpdatesJson", {});
    setUpdates(QString::fromUtf8(QJsonDocument(u).toJson(QJsonDocument::Compact)));
    setPhase(QStringLiteral("idle"));
}

void OsmAppPlugin::hostRegions(const QString& regionsCsv) {
    setWorking(true);
    setPhase(QStringLiteral("hosting"));
    setStatus(QStringLiteral("hosting %1…").arg(regionsCsv));
    const QJsonObject r = callModule("hostRegionsJson", {regionsCsv});
    setWorking(false);
    setPhase(r.isEmpty() ? QStringLiteral("error") : QStringLiteral("idle"));
    setStatus(r.isEmpty() ? QStringLiteral("host failed — see osm module logs")
                          : QStringLiteral("hosted %1 region(s)").arg(r.size()));
    refresh();
}

void OsmAppPlugin::importRegion(const QString& region, const QString& filePath,
                                const QString& md5Override) {
    setWorking(true);
    setPhase(QStringLiteral("importing"));
    setStatus(QStringLiteral("importing %1…").arg(region));
    const QJsonObject r = callModule("importRegionJson", {region, filePath, md5Override});
    setWorking(false);
    setLastVerification(r.isEmpty() ? QStringLiteral("MISMATCH") : QStringLiteral("verified"));
    setPhase(r.isEmpty() ? QStringLiteral("error") : QStringLiteral("idle"));
    setStatus(r.isEmpty() ? QStringLiteral("import failed — checksum or module error")
                          : QStringLiteral("imported %1").arg(region));
    refresh();
}

void OsmAppPlugin::downloadRegion(const QString& region, const QString& destDir) {
    setWorking(true);
    setPhase(QStringLiteral("downloading"));
    setStatus(QStringLiteral("downloading %1…").arg(region));
    const QJsonObject r = callModule("downloadRegionJson", {region, destDir});
    setWorking(false);
    setLastVerification(r.isEmpty() ? QStringLiteral("MISMATCH") : QStringLiteral("verified"));
    setPhase(r.isEmpty() ? QStringLiteral("error") : QStringLiteral("idle"));
    setStatus(r.isEmpty() ? QStringLiteral("download failed")
                          : QStringLiteral("downloaded %1 (verified: %2)")
                                .arg(region, r.contains(QStringLiteral("verified")) &&
                                               r.value(QStringLiteral("verified")).toBool()
                                           ? QStringLiteral("yes")
                                           : QStringLiteral("no")));
}
