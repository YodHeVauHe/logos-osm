cmake_minimum_required(VERSION 3.14)
project(OsmAppPlugin LANGUAGES CXX)

if(DEFINED ENV{LOGOS_MODULE_BUILDER_ROOT})
    include($ENV{LOGOS_MODULE_BUILDER_ROOT}/cmake/LogosModule.cmake)
else()
    message(FATAL_ERROR "LogosModule.cmake not found. Set LOGOS_MODULE_BUILDER_ROOT.")
endif()

# The backend is built from metadata.json's name as the core module
# `osm-app_backend` ("backend": "module").
file(READ "${CMAKE_CURRENT_SOURCE_DIR}/metadata.json" METADATA_JSON)
string(JSON MODULE_NAME GET ${METADATA_JSON} name)

logos_module(
    NAME ${MODULE_NAME}
    REP_FILE src/osm_app.rep
    SOURCES
        src/osm_app_backend.h
        src/osm_app_backend.cpp
        src/module_json.h
        src/local_path.h
)

include(CTest)
if(BUILD_TESTING)
    find_package(Qt6 REQUIRED COMPONENTS Core)
    add_executable(module_json_test tests/module_json_test.cpp)
    target_link_libraries(module_json_test PRIVATE Qt6::Core)
    target_compile_options(module_json_test PRIVATE -UNDEBUG)
    add_test(NAME module_json_test COMMAND module_json_test)
    add_executable(local_path_test tests/local_path_test.cpp)
    target_link_libraries(local_path_test PRIVATE Qt6::Core)
    target_compile_options(local_path_test PRIVATE -UNDEBUG)
    add_test(NAME local_path_test COMMAND local_path_test)
endif()
