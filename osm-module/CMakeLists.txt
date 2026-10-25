cmake_minimum_required(VERSION 3.14)
project(OsmPlugin LANGUAGES CXX)

if(DEFINED ENV{LOGOS_MODULE_BUILDER_ROOT})
    include($ENV{LOGOS_MODULE_BUILDER_ROOT}/cmake/LogosModule.cmake)
else()
    message(FATAL_ERROR "LogosModule.cmake not found. Set LOGOS_MODULE_BUILDER_ROOT.")
endif()

configure_file(${CMAKE_CURRENT_SOURCE_DIR}/metadata.json ${CMAKE_CURRENT_BINARY_DIR}/metadata.json COPYONLY)

# Universal authoring model: only impl sources are listed; the generated
# glue in generated_code/ is picked up automatically.
logos_module(
    NAME osm
    SOURCES
        src/osm_impl.h
        src/osm_impl.cpp
        src/osm_ffi_client.h
)
