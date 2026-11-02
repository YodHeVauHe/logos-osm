#ifndef OSM_CONSUMER_APP_BACKEND_H
#define OSM_CONSUMER_APP_BACKEND_H

#include <QJsonArray>
#include <QJsonValue>
#include <QString>

#include "logos_module_context.h"
#include "rep_osm_consumer_app_source.h"

// Backend of the OSM consumer example app. It talks only to the
// osm_consumer_example core module (never to osm directly), which in turn
// uses the osm SDK module — the embedding path a third-party app follows.
class OsmConsumerAppBackend : public OsmConsumerAppSimpleSource,
                              public LogosModuleContext
{
    Q_OBJECT

public:
    explicit OsmConsumerAppBackend(QObject* parent = nullptr);
    ~OsmConsumerAppBackend() override = default;

public slots:
    void resolveRegion(QString region) override;
    void downloadRegion(QString region, QString destDir) override;

protected:
    void onContextReady() override;

private:
    QJsonValue callConsumer(const char* method, const QJsonArray& args, int timeoutMs,
                            QString* error = nullptr);
    void configureWhenReady(int attempt);
};

#endif // OSM_CONSUMER_APP_BACKEND_H
