CMAKE_MINIMUM_REQUIRED(VERSION 3.12)

PROJECT(voxi NONE)
INCLUDE(GNUInstallDirs)

# Consume rpmbuild-provided install variables so CMake configure stays warning-free.
if(DEFINED INCLUDE_INSTALL_DIR)
  set(CMAKE_INSTALL_INCLUDEDIR "${INCLUDE_INSTALL_DIR}")
endif()
if(DEFINED LIB_INSTALL_DIR)
  set(CMAKE_INSTALL_LIBDIR "${LIB_INSTALL_DIR}")
endif()
if(DEFINED SHARE_INSTALL_PREFIX)
  set(CMAKE_INSTALL_DATAROOTDIR "${SHARE_INSTALL_PREFIX}")
endif()
if(DEFINED SYSCONF_INSTALL_DIR)
  set(CMAKE_INSTALL_SYSCONFDIR "${SYSCONF_INSTALL_DIR}")
endif()
if(DEFINED LIB_SUFFIX)
  set(VOXI_LIB_SUFFIX "${LIB_SUFFIX}")
endif()
option(BUILD_SHARED_LIBS "Build shared libraries when supported." ON)
if(BUILD_SHARED_LIBS)
  set(VOXI_BUILD_MODE "shared")
else()
  set(VOXI_BUILD_MODE "static")
endif()

## Paths
SET(PREFIX ${CMAKE_INSTALL_PREFIX})
SET(BINDIR "${CMAKE_INSTALL_PREFIX}/bin")
SET(APP_DATA_DIR "/opt/usr/share/voxi")

## ═══════════════════════════════════════════
##  Cargo/Rust Build (primary build system)
## ═══════════════════════════════════════════
# OpenSSL is statically linked via vendored build (no system OpenSSL needed)
# Strip LTO flags from CFLAGS because rustc's linker doesn't load the GCC LTO plugin automatically,
# which causes undefined references when linking static C libraries built by the cc crate.
string(REGEX REPLACE "-flto[^ ]*" "" SAFE_CFLAGS "$ENV{CFLAGS}")
string(REGEX REPLACE "-flto[^ ]*" "" SAFE_LDFLAGS "$ENV{LDFLAGS}")

# Voxi's armv7l cross-gcc tries to load a mismatched LTO plugin. Force disable it.
set(SAFE_CFLAGS "${SAFE_CFLAGS} -fno-lto -fno-use-linker-plugin")
set(SAFE_LDFLAGS "${SAFE_LDFLAGS} -Wl,-fno-use-linker-plugin")

ADD_CUSTOM_TARGET(cargo_build ALL
  COMMAND env "CFLAGS=${SAFE_CFLAGS} -Wno-error=missing-field-initializers" "LDFLAGS=${SAFE_LDFLAGS}" "RUSTFLAGS=-C link-arg=-fno-use-linker-plugin -C link-arg=-fno-lto" cargo build --release --offline --manifest-path ${CMAKE_SOURCE_DIR}/Cargo.toml
  WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
  COMMENT "Building Rust workspace with Cargo (LTO disabled for C dependencies and linker)"
)

## ═══════════════════════════════════════════
##  Install Rust binaries
## ═══════════════════════════════════════════
INSTALL(PROGRAMS "${CMAKE_SOURCE_DIR}/target/release/voxi"
        DESTINATION "${BINDIR}")

INSTALL(PROGRAMS "${CMAKE_SOURCE_DIR}/target/release/voxi-cli"
        DESTINATION "${BINDIR}")

INSTALL(PROGRAMS "${CMAKE_SOURCE_DIR}/target/release/voxi-tool-executor"
        DESTINATION "${BINDIR}")

INSTALL(PROGRAMS "${CMAKE_SOURCE_DIR}/target/release/voxi-web-dashboard"
        DESTINATION "${BINDIR}")

## ═══════════════════════════════════════════
##  Install data & config files
## ═══════════════════════════════════════════

# Config sample
INSTALL(FILES "${CMAKE_SOURCE_DIR}/data/sample/tunnel_config.json.sample"
        DESTINATION "${APP_DATA_DIR}/config"
        RENAME "tunnel_config.json")
INSTALL(FILES
        "${CMAKE_SOURCE_DIR}/data/config/agent_roles.json"
        "${CMAKE_SOURCE_DIR}/data/config/system_prompt.txt"
        DESTINATION "${APP_DATA_DIR}/config")



# Systemd service/socket units
INSTALL(FILES
  "${CMAKE_SOURCE_DIR}/packaging/voxi.service"
  "${CMAKE_SOURCE_DIR}/packaging/voxi-tool-executor.service"
  "${CMAKE_SOURCE_DIR}/packaging/voxi-tool-executor.socket"
  DESTINATION "${CMAKE_INSTALL_PREFIX}/lib/systemd/system")

# Web dashboard static files
INSTALL(DIRECTORY "${CMAKE_SOURCE_DIR}/data/web/"
        DESTINATION "${APP_DATA_DIR}/web")
INSTALL(FILES "${CMAKE_SOURCE_DIR}/data/img/voxi.svg"
        DESTINATION "${APP_DATA_DIR}/web/img")

# Packaged skill reference docs
INSTALL(DIRECTORY "${CMAKE_SOURCE_DIR}/data/docs/"
        DESTINATION "${APP_DATA_DIR}/docs")

# tools.md is generated at runtime by the daemon's startup indexer

# Embedded tool descriptors
INSTALL(DIRECTORY "${CMAKE_SOURCE_DIR}/tools/embedded/"
        DESTINATION "${APP_DATA_DIR}/embedded")

# CLI tool scripts (Enabled)
enable_language(C CXX)
set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} "${CMAKE_CURRENT_SOURCE_DIR}/cmake/Modules")
include(ApplyPkgConfig)
ADD_SUBDIRECTORY(tools/cli)

# pkgmgr metadata parser plugins (Rust cdylib — built by cargo_build)
# Install the Rust-built .so files from target/release/
INSTALL(FILES "${CMAKE_SOURCE_DIR}/target/release/libvoxi_metadata_llm_backend_plugin.so"
        DESTINATION "/etc/package-manager/parserlib/metadata"
        RENAME "libvoxi-metadata-llm-backend-plugin.so")
INSTALL(FILES "${CMAKE_SOURCE_DIR}/target/release/libvoxi_metadata_skill_plugin.so"
        DESTINATION "/etc/package-manager/parserlib/metadata"
        RENAME "libvoxi-metadata-skill-plugin.so")
INSTALL(FILES "${CMAKE_SOURCE_DIR}/target/release/libvoxi_metadata_cli_plugin.so"
        DESTINATION "/etc/package-manager/parserlib/metadata"
        RENAME "libvoxi-metadata-cli-plugin.so")

# Install .info files for pkgmgr parser plugin registration
INSTALL(FILES "${CMAKE_SOURCE_DIR}/src/voxi-metadata-llm-backend-plugin/data/voxi-metadata-llm-backend-plugin.info"
        DESTINATION "/usr/share/parser-plugins")
INSTALL(FILES "${CMAKE_SOURCE_DIR}/src/voxi-metadata-skill-plugin/data/voxi-metadata-skill-plugin.info"
        DESTINATION "/usr/share/parser-plugins")
INSTALL(FILES "${CMAKE_SOURCE_DIR}/src/voxi-metadata-cli-plugin/data/voxi-metadata-cli-plugin.info"
        DESTINATION "/usr/share/parser-plugins")

# system_cli removed — tools are discovered dynamically at runtime

# Code executor removed (now pure shell)

# MCP tunnel script
INSTALL(PROGRAMS "${CMAKE_SOURCE_DIR}/scripts/start_mcp_tunnel.sh"
        DESTINATION "${BINDIR}")


## ═══════════════════════════════════════════
##  Install libvoxi_core.so (framework & SDK)
## ═══════════════════════════════════════════
INSTALL(FILES "${CMAKE_SOURCE_DIR}/target/release/libvoxi_core.so"
        DESTINATION "${CMAKE_INSTALL_FULL_LIBDIR}"
        RENAME "libvoxi-core.so")

# Plugin SDK headers
INSTALL(FILES
    "${CMAKE_SOURCE_DIR}/src/libvoxi-core/include/voxi_channel.h"
    "${CMAKE_SOURCE_DIR}/src/libvoxi-core/include/voxi_llm_backend.h"
    "${CMAKE_SOURCE_DIR}/src/libvoxi-core/include/voxi_curl.h"
    DESTINATION "${CMAKE_INSTALL_PREFIX}/include/voxi/core")

INSTALL(FILES
    "${CMAKE_SOURCE_DIR}/src/libvoxi-core/include/voxi_error.h"
    DESTINATION "${CMAKE_INSTALL_PREFIX}/include/voxi")

# pkgconfig for voxi-core
CONFIGURE_FILE(
    "${CMAKE_SOURCE_DIR}/packaging/voxi-core.pc.in"
    "${CMAKE_BINARY_DIR}/voxi-core.pc"
    @ONLY
)
INSTALL(FILES "${CMAKE_BINARY_DIR}/voxi-core.pc"
        DESTINATION "${CMAKE_INSTALL_FULL_LIBDIR}/pkgconfig")

## ═══════════════════════════════════════════
##  Install libvoxi.so (C API Client) & headers
## ═══════════════════════════════════════════
INSTALL(FILES "${CMAKE_SOURCE_DIR}/target/release/libvoxi.so"
        DESTINATION "${CMAKE_INSTALL_FULL_LIBDIR}")

# C header
INSTALL(FILES "${CMAKE_SOURCE_DIR}/src/libvoxi/include/voxi.h"
        DESTINATION "${CMAKE_INSTALL_PREFIX}/include/voxi")

# pkgconfig
CONFIGURE_FILE(
    "${CMAKE_SOURCE_DIR}/packaging/voxi.pc.in"
    "${CMAKE_BINARY_DIR}/voxi.pc"
    @ONLY
)
INSTALL(FILES "${CMAKE_BINARY_DIR}/voxi.pc"
        DESTINATION "${CMAKE_INSTALL_FULL_LIBDIR}/pkgconfig")
