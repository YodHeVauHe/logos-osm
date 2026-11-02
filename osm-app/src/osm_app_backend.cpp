#include "osm_app_backend.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QTimer>

#include "logos_api.h"
#include "logos_api_client.h"
#include "logos_sdk.h"
#include "module_json.h"

OsmAppBackend::OsmAppBackend(QObject* parent)
    : OsmAppSimpleSource(parent)
{
    setRegions(QStringLiteral("[]"));
    setRegistryEntries(QStringLiteral("[]"));
    setUpdates(QStringLiteral("[]"));
    setPhase(QStringLiteral("idle"));
}

void OsmAppBackend::onContextReady() {
    // modules() is live from here; the osm client may still be connecting.
    QTimer::singleShot(0, this, [this]() { loadWhenReady(0); });
}

void OsmAppBackend::loadWhenReady(int attempt) {
    LogosAPIClient* client = modules().api->getClient(QStringLiteral("osm"));
    if (!client || !client->isConnected()) {
        if (attempt >= 50) {
            setPhase(QStringLiteral("error"));
            setStatus(QStringLiteral("osm module did not become ready"));
            return;
        }
        QTimer::singleShot(200, this, [this, attempt]() { loadWhenReady(attempt + 1); });
        return;
    }
    refresh();
}

QJsonValue OsmAppBackend::callModule(const char* method, const QJsonArray& args, QString* error) {
    if (!isContextReady()) {
        if (error) *error = QStringLiteral("Logos API is unavailable");
        return {};
    }
    LogosAPIClient* client = modules().api->getClient(QStringLiteral("osm"));
    if (!client) {
        if (error) *error = QStringLiteral("osm module is unavailable");
        return {};
    }
    // Pass the arguments as a QVariantList: a QJsonArray would bind to the
    // single-QVariant overload and reach osm as one wrong-typed argument.
    const QVariant v = client->invokeRemoteMethod(
        QStringLiteral("osm"), QString::fromLatin1(method), args.toVariantList(),
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

void OsmAppBackend::refresh() {
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

void OsmAppBackend::checkUpdates() {
    setPhase(QStringLiteral("checking"));
    QString error;
    const QJsonValue updates = callModule("checkUpdates", {}, &error);
    if (updates.isArray())
        setUpdates(QString::fromUtf8(QJsonDocument(updates.toArray()).toJson(QJsonDocument::Compact)));
    if (!error.isEmpty()) setStatus(error);
    setPhase(error.isEmpty() ? QStringLiteral("idle") : QStringLiteral("error"));
}

void OsmAppBackend::hostRegions(QString regionsCsv) {
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

void OsmAppBackend::importRegion(QString region, QString filePath, QString md5Override) {
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

void OsmAppBackend::downloadRegion(QString region, QString destDir) {
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
