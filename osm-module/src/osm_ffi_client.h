// Qt-free dlopen client for libosm_ffi.so (Rust osm_ffi cdylib).
//
// Every osm_* function is `unsafe extern "C"` taking/returning NUL-terminated
// UTF-8 JSON strings. A null return means failure; osm_last_error() carries
// the message. Strings returned by the library are freed with osm_free.

#pragma once

#include <dlfcn.h>

#include <string>

class OsmFfiClient
{
public:
    OsmFfiClient() = default;
    ~OsmFfiClient()
    {
        if (lib_)
            dlclose(lib_);
    }

    OsmFfiClient(const OsmFfiClient&) = delete;
    OsmFfiClient& operator=(const OsmFfiClient&) = delete;

    bool load(std::string* err = nullptr)
    {
        if (lib_)
            return true;
        // The vendored .so sits next to this module's plugin library.
        if (!lib_)
            lib_ = dlopen("libosm_ffi.so", RTLD_NOW | RTLD_LOCAL);
        if (!lib_ && err)
            *err = std::string("cannot load libosm_ffi.so: ") + dlerror();
        return lib_ != nullptr;
    }

    void setConfig(const std::string& cfgJson) { cfg_ = cfgJson; }
    const std::string& config() const { return cfg_; }

    std::string lastError() const { return lastError_; }

    std::string discover() { return call("osm_discover", {cfg_}); }
    std::string resolve(const std::string& region) { return call("osm_resolve", {cfg_, region}); }
    std::string query(const std::string& kind, const std::string& value)
    {
        return call("osm_query", {cfg_, kind, value});
    }
    std::string checkUpdates() { return call("osm_check_updates", {cfg_}); }
    std::string host(const std::string& regionsCsv) { return call("osm_host", {cfg_, regionsCsv}); }
    std::string importRegion(const std::string& region, const std::string& filePath,
                             const std::string& md5Override)
    {
        return call("osm_import", {cfg_, region, filePath, md5Override});
    }
    std::string downloadRegion(const std::string& region, const std::string& destDir)
    {
        return call("osm_download", {cfg_, region, destDir});
    }

private:
    using Fn1 = const char* (*)(const char*);
    using Fn2 = const char* (*)(const char*, const char*);
    using Fn3 = const char* (*)(const char*, const char*, const char*);
    using Fn4 = const char* (*)(const char*, const char*, const char*, const char*);

    std::string call(const char* symbol, const std::initializer_list<std::string>& args)
    {
        if (!load(&lastError_))
            return {};
        dlsym(RTLD_DEFAULT, nullptr); // no-op keeps dlerror state clean per call
        void* sym = dlsym(lib_, symbol);
        if (!sym)
        {
            lastError_ = std::string("symbol not found: ") + symbol;
            return {};
        }

        std::string raw[4];
        const char* ptrs[4] = {nullptr, nullptr, nullptr, nullptr};
        int argc = 0;
        for (const auto& a : args)
        {
            raw[argc] = a;
            ptrs[argc] = raw[argc].c_str();
            ++argc;
        }

        const char* out = nullptr;
        switch (argc)
        {
            case 1: out = reinterpret_cast<Fn1>(sym)(ptrs[0]); break;
            case 2: out = reinterpret_cast<Fn2>(sym)(ptrs[0], ptrs[1]); break;
            case 3: out = reinterpret_cast<Fn3>(sym)(ptrs[0], ptrs[1], ptrs[2]); break;
            case 4: out = reinterpret_cast<Fn4>(sym)(ptrs[0], ptrs[1], ptrs[2], ptrs[3]); break;
            default:
                lastError_ = "unsupported argument count";
                return {};
        }

        if (!out)
        {
            if (void* errFn = dlsym(lib_, "osm_last_error"))
            {
                const char* msg = reinterpret_cast<Fn0>(errFn)();
                lastError_ = msg ? msg : "ffi call failed";
            }
            else
            {
                lastError_ = "ffi call failed";
            }
            return {};
        }

        std::string result(out);
        if (void* freeFn = dlsym(lib_, "osm_free"))
            reinterpret_cast<void (*)(const char*)>(freeFn)(out);
        lastError_.clear();
        return result;
    }

    using Fn0 = const char* (*)();

    void* lib_ = nullptr;
    std::string cfg_ = "{}";
    std::string lastError_;
};
