#include "../src/module_json.h"

#include <cassert>

int main() {
    QString error;
    const QJsonValue regions = module_json::parseResponse(
        QStringLiteral("[{\"region\":\"germany\"}]"), &error);
    assert(regions.isArray());
    assert(regions.toArray().size() == 1);
    assert(error.isEmpty());

    const QJsonValue entries = module_json::parseResponse(
        QStringLiteral("{\"ok\":true,\"value\":[{\"region\":\"germany\"}]}"), &error);
    assert(entries.isArray());
    assert(entries.toArray().size() == 1);

    const QJsonValue updates = module_json::parseResponse(
        QStringLiteral("[{\"region\":\"kenya\",\"central_version\":\"261029\"}]"), &error);
    assert(updates.isArray());
    assert(updates.toArray().first().toObject().value(QStringLiteral("region")).toString() ==
           QStringLiteral("kenya"));

    const QJsonValue failure = module_json::parseResponse(
        QStringLiteral("{\"ok\":false,\"error\":\"storage offline\"}"), &error);
    assert(failure.isUndefined());
    assert(error == QStringLiteral("storage offline"));

    const QJsonValue absent = module_json::parseResponse(QStringLiteral("null"), &error);
    assert(absent.isNull());
    assert(error.isEmpty());
}
