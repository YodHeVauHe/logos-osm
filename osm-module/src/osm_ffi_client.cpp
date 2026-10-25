#include "osm_ffi_client.h"

#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcessEnvironment>
#include <QLibrary>

namespace {
const char* kLib = "libosm_ffi.so";
const char* kEnvKey = "OSM_FFI_PATH";

QString errJson(const QString& msg) {
    return QString::fromUtf8(
        QJsonDocument(QJsonObject{
            {QStringLiteral("ok"), false},
            {QStringLiteral("error"), msg},
        }).toJson(QJsonDocument::Compact));
}

// Marker whose address dladdr maps back to osm_plugin.so, letting us locate
// the install directory to find the FFI sibling .so.
static void ffi_client_marker() {}

QString pluginDir() {
    Dl_info info{};
    if (dladdr(reinterpret_cast<void*>(&ffi_client_marker), &info) && info.dli_fname)
        return QFileInfo(QString::fromUtf8(info.dli_fname)).absolutePath();
    return {};
}
} // namespace

typedef const char* (*SimpleFn)();
typedef const char* (*StrFn)(const char*);
typedef const char* (*Str2Fn)(const char*, const char*);
typedef const char* (*Str3Fn)(const char*, const char*, const char*);
typedef const char* (*Str4Fn)(const char*, const char*, const char*, const char*);

FfiClient::FfiClient(QObject* parent) : QObject(parent) {}

FfiClient::~FfiClient() {
    if (m_lib.isLoaded()) m_lib.unload();
}

bool FfiClient::load(QString* err) {
    if (m_loaded) return true;

    const QString envPath =
        QProcessEnvironment::systemEnvironment().value(QLatin1String(kEnvKey));
    QString libPath;
    if (!envPath.isEmpty()) {
        libPath = envPath;
    } else {
        const QString dir = pluginDir();
        if (!dir.isEmpty()) {
            const QString candidate = dir + QLatin1Char('/') + QLatin1String(kLib);
            if (QFileInfo::exists(candidate)) libPath = candidate;
        }
        if (libPath.isEmpty()) libPath = QLatin1String(kLib);
    }

    m_lib.setFileName(libPath);
    if (!m_lib.load()) {
        if (err) *err = QStringLiteral("cannot load %1: %2").arg(libPath, m_lib.errorString());
        return false;
    }
    m_loaded = true;
    return true;
}

QString FfiClient::call(const char* symbol, int argc, const QString* args) {
    if (!m_loaded && !load(nullptr)) {
        m_lastError = QStringLiteral("ffi not loaded");
        return {};
    }
    QFunctionPointer fp = m_lib.resolve(symbol);
    if (!fp) {
        m_lastError = QStringLiteral("symbol %1 not found").arg(QLatin1String(symbol));
        return {};
    }

    QByteArray raw[4];
    const char* ptrs[4] = {nullptr, nullptr, nullptr, nullptr};
    for (int i = 0; i < argc; ++i) {
        raw[i] = args[i].toUtf8();
        ptrs[i] = raw[i].constData();
    }

    const char* out = nullptr;
    switch (argc) {
        case 0: out = reinterpret_cast<SimpleFn>(fp)(); break;
        case 1: out = reinterpret_cast<StrFn>(fp)(ptrs[0]); break;
        case 2: out = reinterpret_cast<Str2Fn>(fp)(ptrs[0], ptrs[1]); break;
        case 3: out = reinterpret_cast<Str3Fn>(fp)(ptrs[0], ptrs[1], ptrs[2]); break;
        case 4: out = reinterpret_cast<Str4Fn>(fp)(ptrs[0], ptrs[1], ptrs[2], ptrs[3]); break;
        default: m_lastError = QStringLiteral("unsupported argc"); return {};
    }

    // Failure convention: null return + osm_last_error().
    if (!out) {
        QFunctionPointer errFn = m_lib.resolve("osm_last_error");
        const char* msg = errFn ? reinterpret_cast<SimpleFn>(errFn)() : nullptr;
        m_lastError = msg ? QString::fromUtf8(msg) : QStringLiteral("ffi call failed");
        return {};
    }

    QFunctionPointer freeFn = m_lib.resolve("osm_free");
    const QString result = QString::fromUtf8(out);
    if (freeFn) reinterpret_cast<void (*)(const char*)>(freeFn)(out);
    m_lastError.clear();
    return result;
}

QString FfiClient::discover() {
    const QString a[] = {m_cfg};
    return call("osm_discover", 1, a);
}

QString FfiClient::resolve(const QString& region) {
    const QString a[] = {m_cfg, region};
    return call("osm_resolve", 2, a);
}

QString FfiClient::query(const QString& kind, const QString& value) {
    const QString a[] = {m_cfg, kind, value};
    return call("osm_query", 3, a);
}

QString FfiClient::checkUpdates() {
    const QString a[] = {m_cfg};
    return call("osm_check_updates", 1, a);
}

QString FfiClient::host(const QString& regionsCsv) {
    const QString a[] = {m_cfg, regionsCsv};
    return call("osm_host", 2, a);
}

QString FfiClient::importRegion(const QString& region, const QString& filePath, const QString& md5Override) {
    const QString a[] = {m_cfg, region, filePath, md5Override};
    return call("osm_import", 4, a);
}

QString FfiClient::downloadRegion(const QString& region, const QString& destDir) {
    const QString a[] = {m_cfg, region, destDir};
    return call("osm_download", 3, a);
}
