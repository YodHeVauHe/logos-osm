#include "osm_consumer_app_backend.h"

#include <QDir>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcessEnvironment>
#include <QTimer>
#include <QUrl>

#include "logos_api.h"
#include "logos_api_client.h"
#include "logos_sdk.h"
#include "module_json.h"

namespace {
const QString kConsumer = QStringLiteral("osm_consumer_example");
}

OsmConsumerAppBackend::OsmConsumerAppBackend(QObject* parent)
    : OsmConsumerAppSimpleSource(parent)
{
    setStatus(QStringLiteral("connecting to osm_consumer_example…"));
}

void OsmConsumerAppBackend::onContextReady() {
    QTimer::singleShot(0, this, [this]() { configureWhenReady(0); });
}

// The osm module starts with an empty config. Hand it the deployment from
// the same environment variables the osm CLI reads, through the consumer.
void OsmConsumerAppBackend::configureWhenReady(int attempt) {
    LogosAPIClient* client = modules().api->getClient(kConsumer);
    if (!client || !client->isConnected()) {
        if (attempt >= 50) {
            setStatus(QStringLiteral("osm_consumer_example module did not become ready"));
            return;
        }
        QTimer::singleShot(200, this, [this, attempt]() { configureWhenReady(attempt + 1); });
        return;
    }
    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    QJsonObject cfg;
    const QString programId = env.value(QStringLiteral("OSM_PROGRAM_ID"));
    if (!programId.isEmpty()) cfg.insert(QStringLiteral("program_id"), programId);
    const QString cliBin = env.value(QStringLiteral("OSM_REGISTRY_CLI"));
    if (!cliBin.isEmpty()) cfg.insert(QStringLiteral("cli_bin"), cliBin);
    if (cfg.isEmpty()) {
        setStatus(QStringLiteral("ready (osm module default config)"));
        return;
    }
    QString error;
    callConsumer("configure",
                 {QString::fromUtf8(QJsonDocument(cfg).toJson(QJsonDocument::Compact))},
                 20 * 1000, &error);
    setStatus(error.isEmpty() ? QStringLiteral("ready (program %1)").arg(programId) : error);
}

QJsonValue OsmConsumerAppBackend::callConsumer(const char* method, const QJsonArray& args,
                                               int timeoutMs, QString* error) {
    if (!isContextReady()) {
        if (error) *error = QStringLiteral("Logos API is unavailable");
        return {};
    }
    LogosAPIClient* client = modules().api->getClient(kConsumer);
    if (!client) {
        if (error) *error = QStringLiteral("osm_consumer_example module is unavailable");
        return {};
    }
    const QVariant v = client->invokeRemoteMethod(
        kConsumer, QString::fromLatin1(method), args.toVariantList(), Timeout(timeoutMs));
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
        if (error) *error = QStringLiteral("osm_consumer_example returned an empty response");
        return {};
    }
    return module_json::parseResponse(text, error);
}

void OsmConsumerAppBackend::resolveRegion(QString region) {
    setWorking(true);
    setStatus(QStringLiteral("resolving %1…").arg(region));
    QString error;
    const QJsonValue entry = callConsumer("resolveRegion", {region}, 60 * 1000, &error);
    setWorking(false);
    if (!error.isEmpty()) {
        setEntry(QString());
        setStatus(error);
        return;
    }
    if (!entry.isObject() || entry.toObject().isEmpty()) {
        setEntry(QString());
        setStatus(QStringLiteral("%1 is not hosted in the registry").arg(region));
        return;
    }
    setEntry(QString::fromUtf8(QJsonDocument(entry.toObject()).toJson(QJsonDocument::Indented)));
    setStatus(QStringLiteral("resolved %1").arg(region));
}

void OsmConsumerAppBackend::downloadRegion(QString region, QString destDir) {
    setWorking(true);
    setLastVerification(QString());
    setLastPath(QString());
    QString dir = destDir;
    if (dir.startsWith(QStringLiteral("file://"))) dir = QUrl(dir).toLocalFile();
    if (dir.isEmpty()) dir = QDir::homePath();
    QDir().mkpath(dir);
    setStatus(QStringLiteral("downloading %1 into %2…").arg(region, dir));
    QString error;
    const QJsonValue r = callConsumer("downloadRegion", {region, dir}, 30 * 60 * 1000, &error);
    setWorking(false);
    if (!error.isEmpty()) {
        setStatus(error);
        return;
    }
    const QJsonObject o = r.toObject();
    const bool verified = o.value(QStringLiteral("verified")).toBool();
    setLastVerification(verified ? QStringLiteral("verified") : QStringLiteral("MISMATCH"));
    setLastPath(o.value(QStringLiteral("path")).toString());
    setStatus(QStringLiteral("downloaded %1 from %2, %3 bytes (verified: %4)")
                  .arg(region, o.value(QStringLiteral("source")).toString(),
                       QString::number(o.value(QStringLiteral("bytes")).toDouble(), 'f', 0),
                       verified ? QStringLiteral("yes") : QStringLiteral("no")));
}
