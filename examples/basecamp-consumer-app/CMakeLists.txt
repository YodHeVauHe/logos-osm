cmake_minimum_required(VERSION 3.14)
project(OsmConsumerAppPlugin LANGUAGES CXX)

if(DEFINED ENV{LOGOS_MODULE_BUILDER_ROOT})
    include($ENV{LOGOS_MODULE_BUILDER_ROOT}/cmake/LogosModule.cmake)
else()
    message(FATAL_ERROR "LogosModule.cmake not found. Set LOGOS_MODULE_BUILDER_ROOT.")
endif()

# The backend is built from metadata.json's name as the core module
# `osm-consumer-app_backend` ("backend": "module").
file(READ "${CMAKE_CURRENT_SOURCE_DIR}/metadata.json" METADATA_JSON)
string(JSON MODULE_NAME GET ${METADATA_JSON} name)

logos_module(
    NAME ${MODULE_NAME}
    REP_FILE src/osm_consumer_app.rep
    SOURCES
        src/osm_consumer_app_backend.h
        src/osm_consumer_app_backend.cpp
        src/module_json.h
)
