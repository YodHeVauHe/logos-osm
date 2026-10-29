#ifndef OSM_APP_MODULE_JSON_H
#define OSM_APP_MODULE_JSON_H

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QJsonValue>
#include <QString>

namespace module_json {

inline QJsonValue parseResponse(const QString& text, QString* error = nullptr) {
    if (error) error->clear();

    const QByteArray json = text.toUtf8().trimmed();
    const bool scalarRoot = !json.startsWith('{') && !json.startsWith('[');
    const QByteArray parseInput = scalarRoot ? QByteArray("[") + json + QByteArray("]") : json;
    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(parseInput, &parseError);
    if (parseError.error != QJsonParseError::NoError || document.isNull()) {
        if (error) *error = QStringLiteral("invalid module JSON: %1").arg(parseError.errorString());
        return QJsonValue(QJsonValue::Undefined);
    }

    QJsonValue value = scalarRoot ? document.array().first()
                                  : (document.isArray() ? QJsonValue(document.array())
                                                        : QJsonValue(document.object()));
    if (value.isObject()) {
        const QJsonObject object = value.toObject();
        if (object.value(QStringLiteral("ok")).isBool()) {
            if (!object.value(QStringLiteral("ok")).toBool()) {
                if (error) {
                    *error = object.value(QStringLiteral("error")).toString(
                        QStringLiteral("module call failed"));
                }
                return QJsonValue(QJsonValue::Undefined);
            }
            if (object.contains(QStringLiteral("value")))
                return object.value(QStringLiteral("value"));
        }
    }
    return value;
}

} // namespace module_json

#endif // OSM_APP_MODULE_JSON_H
