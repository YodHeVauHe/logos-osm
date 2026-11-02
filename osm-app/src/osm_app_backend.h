#ifndef OSM_APP_BACKEND_H
#define OSM_APP_BACKEND_H

#include <QJsonArray>
#include <QJsonValue>
#include <QString>

#include "logos_module_context.h"
#include "rep_osm_app_source.h"

// Backend of the OSM distribution app. It implements the osm_app.rep view
// contract, so its PROPs reach the QML view as a replica
// (`logos.module("osm-app")`). With "backend": "module" it runs as the core
// module `osm-app_backend`; all heavy lifting lives in the osm core module
// (which fronts the Rust SDK via FFI).
class OsmAppBackend : public OsmAppSimpleSource,
                      public LogosModuleContext
{
    Q_OBJECT

public:
    explicit OsmAppBackend(QObject* parent = nullptr);
    ~OsmAppBackend() override = default;

public slots:
    void hostRegions(QString regionsCsv) override;
    void importRegion(QString region, QString filePath, QString md5Override) override;
    void downloadRegion(QString region, QString destDir) override;
    void refresh() override;
    void checkUpdates() override;

protected:
    void onContextReady() override;

private:
    QJsonValue callModule(const char* method, const QJsonArray& args, QString* error = nullptr);
    void loadWhenReady(int attempt);
};

#endif // OSM_APP_BACKEND_H
