cmake_minimum_required(VERSION 3.14)
project(OsmAppPlugin LANGUAGES CXX)

if(DEFINED ENV{LOGOS_MODULE_BUILDER_ROOT})
    include($ENV{LOGOS_MODULE_BUILDER_ROOT}/cmake/LogosModule.cmake)
else()
    message(FATAL_ERROR "LogosModule.cmake not found. Set LOGOS_MODULE_BUILDER_ROOT.")
endif()

configure_file(${CMAKE_CURRENT_SOURCE_DIR}/metadata.json ${CMAKE_CURRENT_BINARY_DIR}/metadata.json COPYONLY)

logos_module(
    NAME osm-app
    REP_FILE src/osm_app.rep
    SOURCES
        src/osm_app_interface.h
        src/osm_app_plugin.h
        src/osm_app_plugin.cpp
        src/module_json.h
)

include(CTest)
if(BUILD_TESTING)
    find_package(Qt6 REQUIRED COMPONENTS Core)
    add_executable(module_json_test tests/module_json_test.cpp)
    target_link_libraries(module_json_test PRIVATE Qt6::Core)
    target_compile_options(module_json_test PRIVATE -UNDEBUG)
    add_test(NAME module_json_test COMMAND module_json_test)
endif()
