#include "osm_app_plugin.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>

#include "logos_api.h"
#include "logos_api_client.h"
#include "module_json.h"

OsmAppPlugin::OsmAppPlugin() = default;

OsmAppPlugin::~OsmAppPlugin() = default;

QJsonValue OsmAppPlugin::callModule(const char* method, const QJsonArray& args, QString* error) {
    if (!logosAPI) {
        if (error) *error = QStringLiteral("Logos API is unavailable");
        return {};
    }
    LogosAPIClient* client = logosAPI->getClient(QStringLiteral("osm"));
    if (!client) {
        if (error) *error = QStringLiteral("osm module is unavailable");
        return {};
    }
    const QVariant v = client->invokeRemoteMethod(
        QStringLiteral("osm"), QString::fromLatin1(method), args,
        Timeout(60 * 1000));
    if (v.canConvert<LogosResult>()) {
        const LogosResult lr = v.value<LogosResult>();
        if (!lr.success) {
            if (error) *error = lr.value.toString();
            return {};
        }
        return module_json::parseResponse(lr.value.toString(), error);
    }
    const QString text = v.toString();
    if (text.isEmpty()) {
        if (error) *error = QStringLiteral("osm module returned an empty response");
        return {};
    }
    return module_json::parseResponse(text, error);
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
    QString error;
    const QJsonValue discovered = callModule("discover", {}, &error);
    if (!discovered.isArray()) {
        setStatus(error.isEmpty() ? QStringLiteral("discover returned a non-array response") : error);
        return;
    }
    setRegions(QString::fromUtf8(QJsonDocument(discovered.toArray()).toJson(QJsonDocument::Compact)));

    QJsonArray entries;
    for (const QJsonValue& value : discovered.toArray()) {
        const QString region = value.toObject().value(QStringLiteral("region")).toString();
        if (region.isEmpty()) continue;
        QString queryError;
        const QJsonValue entry = callModule("resolve", {region}, &queryError);
        if (entry.isObject() && !entry.toObject().isEmpty()) {
            entries.append(entry);
        } else if (!queryError.isEmpty()) {
            setStatus(queryError);
        }
    }
    setRegistryEntries(QString::fromUtf8(QJsonDocument(entries).toJson(QJsonDocument::Compact)));
}

void OsmAppPlugin::checkUpdates() {
    setPhase(QStringLiteral("checking"));
    QString error;
    const QJsonValue updates = callModule("checkUpdates", {}, &error);
    if (updates.isArray())
        setUpdates(QString::fromUtf8(QJsonDocument(updates.toArray()).toJson(QJsonDocument::Compact)));
    if (!error.isEmpty()) setStatus(error);
    setPhase(error.isEmpty() ? QStringLiteral("idle") : QStringLiteral("error"));
}

void OsmAppPlugin::hostRegions(const QString& regionsCsv) {
    setWorking(true);
    setPhase(QStringLiteral("hosting"));
    setStatus(QStringLiteral("hosting %1…").arg(regionsCsv));
    QString error;
    callModule("hostRegions", {regionsCsv}, &error);
    setWorking(false);
    setPhase(error.isEmpty() ? QStringLiteral("idle") : QStringLiteral("error"));
    setStatus(error.isEmpty() ? QStringLiteral("hosting complete") : error);
    refresh();
}

void OsmAppPlugin::importRegion(const QString& region, const QString& filePath,
                                const QString& md5Override) {
    setWorking(true);
    setPhase(QStringLiteral("importing"));
    setStatus(QStringLiteral("importing %1…").arg(region));
    QString error;
    const QJsonValue r = callModule("importRegion", {region, filePath, md5Override}, &error);
    setWorking(false);
    const bool verified = r.isObject() && r.toObject().value(QStringLiteral("verified")).toBool();
    // A failed module call is not a checksum mismatch; clear the result instead.
    setLastVerification(!error.isEmpty() ? QString()
                        : verified       ? QStringLiteral("verified")
                                         : QStringLiteral("MISMATCH"));
    setPhase(error.isEmpty() ? QStringLiteral("idle") : QStringLiteral("error"));
    setStatus(error.isEmpty() ? QStringLiteral("imported %1").arg(region) : error);
    refresh();
}

void OsmAppPlugin::downloadRegion(const QString& region, const QString& destDir) {
    setWorking(true);
    setPhase(QStringLiteral("downloading"));
    setStatus(QStringLiteral("downloading %1…").arg(region));
    QString error;
    const QJsonValue r = callModule("downloadRegion", {region, destDir}, &error);
    setWorking(false);
    const bool verified = r.isObject() && r.toObject().value(QStringLiteral("verified")).toBool();
    // A failed module call is not a checksum mismatch; clear the result instead.
    setLastVerification(!error.isEmpty() ? QString()
                        : verified       ? QStringLiteral("verified")
                                         : QStringLiteral("MISMATCH"));
    setPhase(error.isEmpty() ? QStringLiteral("idle") : QStringLiteral("error"));
    setStatus(error.isEmpty() ? QStringLiteral("downloaded %1 (verified: %2)")
                                    .arg(region, verified ? QStringLiteral("yes") : QStringLiteral("no"))
                              : error);
}
