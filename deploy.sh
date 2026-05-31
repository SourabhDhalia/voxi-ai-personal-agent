#!/usr/bin/env bash
# Voxi Host Build & Run Script
# Builds and runs Voxi natively on macOS, Ubuntu, or WSL
#
# Usage:
#   ./deploy.sh                   # Build (release) + install + run
#   ./deploy.sh -d, --debug       # Build in debug mode
#   ./deploy.sh -b, --build-only  # Build only, do not run
#   ./deploy.sh --restart-only    # Restart using installed host files
#   ./deploy.sh -s, --stop        # Stop running daemon
#   ./deploy.sh --status          # Show daemon status
#   ./deploy.sh --log             # Follow daemon logs
#   ./deploy.sh --dry-run         # Print commands without executing
#   ./deploy.sh --test            # Build + run cargo tests
#   ./deploy.sh -h, --help        # Show this help

set -euo pipefail

# ─────────────────────────────────────────────
# Constants
# ─────────────────────────────────────────────
PROJECT_DIR="$(cd "$(dirname "$0")" && pwd)"
PKG_NAME="voxi"
TOOL_EXECUTOR_NAME="voxi-tool-executor"
CLI_NAME="voxi-cli"
WEB_DASHBOARD_NAME="voxi-web-dashboard"
HOST_DASHBOARD_PORT_DEFAULT=9091

HOST_BASE_DIR="${HOME}/.voxi"
INSTALL_DIR="${HOST_BASE_DIR}/bin"
LIB_DIR="${HOST_BASE_DIR}/lib"
INCLUDE_DIR="${HOST_BASE_DIR}/include"
PKGCONFIG_DIR="${LIB_DIR}/pkgconfig"
DATA_DIR="${HOST_BASE_DIR}"
BUILD_ROOT_DIR="${HOST_BASE_DIR}/build"
CARGO_TARGET_DIR_DEFAULT="${BUILD_ROOT_DIR}/cargo-target"
TOOLS_DIR="${DATA_DIR}/tools"
WORKSPACE_DIR="${DATA_DIR}/workspace"
LOG_DIR="${DATA_DIR}/logs"
CONFIG_DIR="${DATA_DIR}/config"
DOCS_SRC="${PROJECT_DIR}/data/docs"
EMBEDDED_TOOLS_SRC="${PROJECT_DIR}/tools/embedded"
WEB_SRC="${PROJECT_DIR}/data/web"
BUNDLED_CONFIG_DIR="${PROJECT_DIR}/data/config"
BUNDLED_WORKFLOWS_DIR="${PROJECT_DIR}/data/workflows"
WORKFLOWS_DIR="${DATA_DIR}/workflows"
BASHRC_PATH="${HOME}/.bashrc"
PATH_EXPORT='export PATH="$HOME/.voxi/bin:$PATH"'

PID_FILE="/tmp/voxi-host.pid"
TOOL_EXECUTOR_PID_FILE="/tmp/voxi-tool-executor-host.pid"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# ─────────────────────────────────────────────
# Defaults
# ─────────────────────────────────────────────
BUILD_MODE="release"
BUILD_ONLY=false
STOP_DAEMON=false
RESTART_ONLY=false
SHOW_STATUS=false
FOLLOW_LOG=false
DRY_RUN=false
RUN_TESTS=false
REMOVE_INSTALL=false
LLM_CONFIG=""
CARGO_TARGET_DIR_HOST="${CARGO_TARGET_DIR:-${CARGO_TARGET_DIR_DEFAULT}}"

# ─────────────────────────────────────────────
# Logging helpers
# ─────────────────────────────────────────────
log()    { echo -e "${CYAN}[HOST]${NC} $*"; }
ok()     { echo -e "${GREEN}[  OK  ]${NC} $*"; }
warn()   { echo -e "${YELLOW}[ WARN ]${NC} $*"; }
fail()   { echo -e "${RED}[ FAIL ]${NC} $*"; exit 1; }
header() {
  echo -e "\n${BOLD}══════════════════════════════════════════${NC}"
  echo -e "${BOLD}  $*${NC}"
  echo -e "${BOLD}══════════════════════════════════════════${NC}"
}

run() {
  if [ "${DRY_RUN}" = true ]; then
    echo -e "  ${YELLOW}[DRY-RUN]${NC} $*"
    return 0
  fi
  "$@"
}

process_report() {
  local ps_format="pid,ppid,stat,cmd"
  if [ "$(uname)" = "Darwin" ]; then
    ps_format="pid,ppid,state,command"
  fi
  ps -eo "${ps_format}" \
    | grep -E "(${INSTALL_DIR}/${PKG_NAME}|${INSTALL_DIR}/${TOOL_EXECUTOR_NAME}|${INSTALL_DIR}/${WEB_DASHBOARD_NAME}|(^|/| )${PKG_NAME}($| )|(^|/| )${TOOL_EXECUTOR_NAME}($| )|(^|/| )${WEB_DASHBOARD_NAME}($| ))" \
    | grep -v -E "grep -E|deploy.sh" || true
}

dashboard_port() {
  python3 - <<'PY' "${CONFIG_DIR}/channel_config.json" "${HOST_DASHBOARD_PORT_DEFAULT}"
import json, pathlib, sys
path = pathlib.Path(sys.argv[1])
default_port = int(sys.argv[2])
port = default_port
try:
    if path.exists():
        data = json.loads(path.read_text(encoding="utf-8"))
        for channel in data.get("channels", []):
            if channel.get("name") == "web_dashboard":
                port = int(channel.get("settings", {}).get("port", default_port))
                break
except Exception:
    port = default_port
print(port)
PY
}

normalize_host_dashboard_config() {
  local config_path="${CONFIG_DIR}/channel_config.json"
  log "Normalizing host dashboard port to ${HOST_DASHBOARD_PORT_DEFAULT}"
  if [ "${DRY_RUN}" = false ]; then
    python3 - <<'PY' "${config_path}" "${HOST_DASHBOARD_PORT_DEFAULT}"
import json, pathlib, sys

path = pathlib.Path(sys.argv[1])
port = int(sys.argv[2])

data = {"channels": []}
if path.exists():
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        data = {"channels": []}

channels = data.get("channels")
if not isinstance(channels, list):
    channels = []
    data["channels"] = channels

dashboard = None
for channel in channels:
    if isinstance(channel, dict) and channel.get("name") == "web_dashboard":
        dashboard = channel
        break

if dashboard is None:
    dashboard = {
        "name": "web_dashboard",
        "type": "web_dashboard",
        "enabled": True,
        "settings": {},
    }
    channels.append(dashboard)

settings = dashboard.get("settings")
if not isinstance(settings, dict):
    settings = {}
    dashboard["settings"] = settings

dashboard.setdefault("type", "web_dashboard")
dashboard.setdefault("enabled", True)
settings["port"] = port
settings.setdefault("localhost_only", False)

path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
PY
  else
    echo -e "  ${YELLOW}[DRY-RUN]${NC} normalize ${config_path} to port ${HOST_DASHBOARD_PORT_DEFAULT}"
  fi
  ok "Host dashboard config uses port ${HOST_DASHBOARD_PORT_DEFAULT}"
}

port_report() {
  local port="$1"
  if command -v ss &>/dev/null; then
    ss -ltnp "( sport = :${port} )" 2>/dev/null | sed '1d' || true
  else
    lsof -i :${port} -sTCP:LISTEN -Fp 2>/dev/null || true
  fi
}

warn_if_dashboard_port_busy() {
  local port="$1"
  local listeners
  listeners="$(port_report "${port}")"
  if [ -n "${listeners}" ]; then
    warn "Dashboard port ${port} is already in use before startup:"
    printf '%s\n' "${listeners}"
    warn "The dashboard may exit immediately until the port is freed or reconfigured."
    return 0
  fi
  ok "Dashboard port ${port} is available"
}

wait_for_process_name_exit() {
  local label="$1"
  local binary_name="$2"
  local timeout_secs="${3:-5}"
  local waited=0
  local current_uid
  current_uid="$(id -u)"

  while pgrep -u "${current_uid}" -x "${binary_name}" >/dev/null 2>&1 \
    || pgrep -u "${current_uid}" -f "${INSTALL_DIR}/${binary_name}([[:space:]]|$)" >/dev/null 2>&1; do
    if [ "${waited}" -ge "${timeout_secs}" ]; then
      warn "${label} still appears to be alive after ${timeout_secs}s"
      return 1
    fi
    sleep 1
    waited=$((waited + 1))
  done

  return 0
}

# ─────────────────────────────────────────────
# Usage
# ─────────────────────────────────────────────
usage() {
  cat <<EOF
${BOLD}Voxi Host Linux Build & Run${NC}

${CYAN}Usage:${NC}
  $(basename "$0") [options]

${CYAN}Options:${NC}
  -d, --debug             Build in debug mode (default: release)
  -b, --build-only        Build only, do not install or run
      --test              Build + run cargo tests (offline)
      --restart-only      Restart the installed host daemon only
  -s, --stop              Stop the running host daemon
      --remove            Stop host processes and remove ~/.voxi install
      --status            Show current daemon status
      --log               Follow daemon log output
      --dry-run           Print commands without executing
      --build-root <dir>  Override host Cargo target dir
      --llm-config <path> Use specified llm_config.json (sets VOXI_DATA_DIR)
  -h, --help              Show this help

${CYAN}Examples:${NC}
  $(basename "$0")                           # Release build + install + run
  $(basename "$0") -d                        # Debug build + install + run
  $(basename "$0") -b                        # Build only
  $(basename "$0") --test                    # Run unit/integration tests
  $(basename "$0") --status                  # Check daemon status
  $(basename "$0") --log                     # Tail daemon logs
  $(basename "$0") -s                        # Stop the daemon
  $(basename "$0") --remove                  # Remove host install and stop tools
  $(basename "$0") --build-root /tmp/voxi-build  # Use external build root
  $(basename "$0") --llm-config /path/to/llm_config.json  # Use custom LLM config
EOF
  exit 0
}

# ─────────────────────────────────────────────
# Argument parsing
# ─────────────────────────────────────────────
parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -d|--debug)       BUILD_MODE="debug"; shift ;;
      -b|--build-only)  BUILD_ONLY=true; shift ;;
      --test)           RUN_TESTS=true; shift ;;
      --restart-only)   RESTART_ONLY=true; shift ;;
      -s|--stop)        STOP_DAEMON=true; shift ;;
      --remove)         REMOVE_INSTALL=true; shift ;;
      --status)         SHOW_STATUS=true; shift ;;
      --log)            FOLLOW_LOG=true; shift ;;
      --dry-run)        DRY_RUN=true; shift ;;
      --build-root)
        [[ $# -lt 2 ]] && fail "--build-root requires a path argument"
        mkdir -p "$2"
        CARGO_TARGET_DIR_HOST="$(realpath "$2")"; shift 2 ;;
      --llm-config)
        [[ $# -lt 2 ]] && fail "--llm-config requires a path argument"
        LLM_CONFIG="$(realpath "$2")"; shift 2 ;;
      -h|--help)        usage ;;
      *) fail "Unknown option: $1 (use --help)" ;;
    esac
  done
}

# ─────────────────────────────────────────────
# Pre-flight checks
# ─────────────────────────────────────────────
check_prerequisites() {
  header "Pre-flight Checks"

  if ! command -v cargo &>/dev/null; then
    fail "cargo not found. Install Rust: https://rustup.rs"
  fi
  ok "cargo found: $(cargo --version)"

  local rust_ver
  rust_ver=$(rustc --version 2>/dev/null || echo "unknown")
  ok "rustc: ${rust_ver}"

  log "Build mode  : ${BUILD_MODE}"
  log "Project dir : ${PROJECT_DIR}"
  log "Build only  : ${BUILD_ONLY}"
  log "Data dir    : ${DATA_DIR}"
  log "Build root  : ${CARGO_TARGET_DIR_HOST}"
}

ensure_shell_path() {
  header "PATH Bootstrap"

  if [ ! -f "${BASHRC_PATH}" ]; then
    run touch "${BASHRC_PATH}"
  fi

  if grep -Fqx "${PATH_EXPORT}" "${BASHRC_PATH}" 2>/dev/null; then
    ok "~/.bashrc already contains host PATH export"
  else
    log "Appending host PATH export to ${BASHRC_PATH}"
    if [ "${DRY_RUN}" = true ]; then
      echo -e "  ${YELLOW}[DRY-RUN]${NC} printf '\\n%s\\n' '${PATH_EXPORT}' >> '${BASHRC_PATH}'"
    else
      printf '\n%s\n' "${PATH_EXPORT}" >> "${BASHRC_PATH}"
    fi
    ok "Added PATH export to ~/.bashrc"
  fi

  log "Please run 'source ~/.bashrc' or restart your terminal to update your PATH."
}

# ─────────────────────────────────────────────
# Step 1: Build
# ─────────────────────────────────────────────
do_build() {
  header "Step 1/3: Cargo Build (Host — Generic Linux)"

  local cargo_args=("build" "--offline")
  if [ "${BUILD_MODE}" = "release" ]; then
    cargo_args+=("--release")
  fi

  # Build daemon + tool-executor + CLI + web-dashboard + shared client library
  cargo_args+=(
    "-p" "${PKG_NAME}"
    "-p" "voxi-client"
    "-p" "${TOOL_EXECUTOR_NAME}"
    "-p" "${CLI_NAME}"
    "-p" "${WEB_DASHBOARD_NAME}"
  )

  log "Running: cargo ${cargo_args[*]}"
  cd "${PROJECT_DIR}"

  if [ "${DRY_RUN}" = true ]; then
    echo -e "  ${YELLOW}[DRY-RUN]${NC} export CARGO_TARGET_DIR='${CARGO_TARGET_DIR_HOST}'"
    echo -e "  ${YELLOW}[DRY-RUN]${NC} cargo ${cargo_args[*]}"
    ok "Build succeeded (dry-run)"
    return 0
  fi

  mkdir -p "${CARGO_TARGET_DIR_HOST}"

  if CARGO_TARGET_DIR="${CARGO_TARGET_DIR_HOST}" cargo "${cargo_args[@]}"; then
    ok "Cargo build succeeded (${BUILD_MODE})"
  else
    fail "Cargo build failed"
  fi
}

# ─────────────────────────────────────────────
# Step 1 (alt): Run tests
# ─────────────────────────────────────────────
do_test() {
  header "Running Tests (Host — Generic Linux)"

  log "Stopping running host processes before test cycle"
  stop_daemon
  if [ "${DRY_RUN}" = false ]; then
    process_report || true
  fi

  log "Running: cargo test --workspace --offline"
  cd "${PROJECT_DIR}"

  if [ "${DRY_RUN}" = true ]; then
    echo -e "  ${YELLOW}[DRY-RUN]${NC} export CARGO_TARGET_DIR='${CARGO_TARGET_DIR_HOST}'"
    echo -e "  ${YELLOW}[DRY-RUN]${NC} cargo test --workspace --offline"
    return 0
  fi

  mkdir -p "${CARGO_TARGET_DIR_HOST}"

  if CARGO_TARGET_DIR="${CARGO_TARGET_DIR_HOST}" cargo test --workspace --offline -- --test-threads=1 2>&1; then
    ok "All tests passed"
  else
    warn "Some tests failed (see output above)"
  fi
}

# ─────────────────────────────────────────────
# Step 2: Install binaries and data
# ─────────────────────────────────────────────
do_install() {
  header "Step 2/3: Install Binaries"

  local build_dir="${CARGO_TARGET_DIR_HOST}/${BUILD_MODE}"

  log "Preparing host install tree under ${DATA_DIR}"
  run mkdir -p "${INSTALL_DIR}" "${LIB_DIR}" "${INCLUDE_DIR}/voxi" \
    "${INCLUDE_DIR}/voxi/core" "${PKGCONFIG_DIR}" "${CONFIG_DIR}" "${TOOLS_DIR}/cli" \
    "${WORKSPACE_DIR}/skills" "${TOOLS_DIR}" "${DATA_DIR}/embedded" "${DATA_DIR}/web" \
    "${DATA_DIR}/workflows" "${DATA_DIR}/pipelines" "${DATA_DIR}/codes" \
    "${DATA_DIR}/memory" "${DATA_DIR}/plugins" "${LOG_DIR}"

  if [ -d "${TOOLS_DIR}/skills" ] && [ ! -e "${WORKSPACE_DIR}/skills" ]; then
    log "Migrating legacy skills dir → ${WORKSPACE_DIR}/skills"
    run mv "${TOOLS_DIR}/skills" "${WORKSPACE_DIR}/skills"
  fi
  if [ "${DRY_RUN}" = false ]; then
    run mkdir -p "${WORKSPACE_DIR}/skills"
    if [ -L "${TOOLS_DIR}/skills" ] || [ -d "${TOOLS_DIR}/skills" ] || [ -f "${TOOLS_DIR}/skills" ]; then
      run rm -rf "${TOOLS_DIR}/skills"
    fi
    run ln -s "${WORKSPACE_DIR}/skills" "${TOOLS_DIR}/skills"
  else
    echo -e "  ${YELLOW}[DRY-RUN]${NC} mkdir -p '${WORKSPACE_DIR}/skills'"
    echo -e "  ${YELLOW}[DRY-RUN]${NC} ln -s '${WORKSPACE_DIR}/skills' '${TOOLS_DIR}/skills'"
  fi

  for bin in "${PKG_NAME}" "${TOOL_EXECUTOR_NAME}" "${CLI_NAME}" "${WEB_DASHBOARD_NAME}"; do
    local bin_path="${build_dir}/${bin}"
    if [ "${DRY_RUN}" = false ] && [ ! -f "${bin_path}" ]; then
      fail "Binary not found: ${bin_path}"
    fi
    log "Installing ${bin} → ${INSTALL_DIR}/${bin}"
    run install -m 755 "${bin_path}" "${INSTALL_DIR}/${bin}"
    ok "Installed: ${bin}"
  done

  # Rename output library files based on the named voxi library
  local lib_candidates=(
    "libvoxi.so"
    "libvoxi.dylib"
    "libvoxi.rlib"
    "libvoxi_core.so"
    "libvoxi_core.dylib"
    "libvoxi_core.rlib"
  )
  for lib_name in "${lib_candidates[@]}"; do
    local lib_path="${build_dir}/${lib_name}"
    if [ ! -f "${lib_path}" ]; then
      continue
    fi
    log "Installing ${lib_name} → ${LIB_DIR}/${lib_name}"
    run install -m 755 "${lib_path}" "${LIB_DIR}/${lib_name}"
    ok "Installed library: ${lib_name}"
  done

  log "Installing public headers → ${INCLUDE_DIR}/voxi"
  run install -m 644 "${PROJECT_DIR}/src/voxi-client/include/voxi.h" \
    "${INCLUDE_DIR}/voxi/voxi.h"
  run install -m 644 "${PROJECT_DIR}/src/voxi-core/include/voxi_error.h" \
    "${INCLUDE_DIR}/voxi/voxi_error.h"
  run install -m 644 "${PROJECT_DIR}/src/voxi-core/include/voxi_channel.h" \
    "${INCLUDE_DIR}/voxi/core/voxi_channel.h"
  run install -m 644 "${PROJECT_DIR}/src/voxi-core/include/voxi_llm_backend.h" \
    "${INCLUDE_DIR}/voxi/core/voxi_llm_backend.h"
  run install -m 644 "${PROJECT_DIR}/src/voxi-core/include/voxi_curl.h" \
    "${INCLUDE_DIR}/voxi/core/voxi_curl.h"
  ok "Headers installed"

  log "Generating host pkg-config metadata"
  if [ "${DRY_RUN}" = false ]; then
    cat > "${PKGCONFIG_DIR}/voxi.pc" <<EOF
prefix=${HOST_BASE_DIR}
libdir=\${prefix}/lib
includedir=\${prefix}/include

Name: voxi
Description: Voxi Agent C API library
Version: 1.0.0
Libs: -L\${libdir} -Wl,-rpath,\${libdir} -lvoxi
Cflags: -I\${includedir} -I\${includedir}/voxi
EOF

    cat > "${PKGCONFIG_DIR}/voxi-core.pc" <<EOF
prefix=${HOST_BASE_DIR}
libdir=\${prefix}/lib
includedir=\${prefix}/include

Name: voxi-core
Description: Voxi Plugin SDK
Version: 1.0.0
Libs: -L\${libdir} -Wl,-rpath,\${libdir} -lvoxi_core
Cflags: -I\${includedir}/voxi/core -I\${includedir}/voxi
Requires: voxi, libcurl
EOF
  else
    echo -e "  ${YELLOW}[DRY-RUN]${NC} write ${PKGCONFIG_DIR}/voxi.pc"
    echo -e "  ${YELLOW}[DRY-RUN]${NC} write ${PKGCONFIG_DIR}/voxi-core.pc"
  fi
  ok "pkg-config metadata installed"

  # Deploy web dashboard
  if [ -d "${WEB_SRC}" ]; then
    log "Installing web dashboard → ${DATA_DIR}/web"
    run cp -r "${WEB_SRC}/." "${DATA_DIR}/web/"
    ok "Web dashboard installed"
  fi

  if [ -d "${DOCS_SRC}" ]; then
    log "Installing docs → ${DATA_DIR}/docs"
    run mkdir -p "${DATA_DIR}/docs"
    run cp -r "${DOCS_SRC}/." "${DATA_DIR}/docs/"
    ok "Docs installed"
  fi

  if [ -d "${BUNDLED_CONFIG_DIR}" ]; then
    log "Seeding default config files into ${CONFIG_DIR} when missing"
    while IFS= read -r config_path; do
      local file_name
      file_name="$(basename "${config_path}")"
      local target_path="${CONFIG_DIR}/${file_name}"
      if [ ! -f "${target_path}" ]; then
        run install -m 644 "${config_path}" "${target_path}"
      fi
    done < <(find "${BUNDLED_CONFIG_DIR}" -maxdepth 1 -type f ! -name '*.sample' | sort)
    ok "Default config seeding complete"
  fi

  if [ -d "${BUNDLED_WORKFLOWS_DIR}" ]; then
    log "Seeding default workflows into ${WORKFLOWS_DIR} when missing"
    run mkdir -p "${WORKFLOWS_DIR}"
    while IFS= read -r workflow_path; do
      local file_name
      file_name="$(basename "${workflow_path}")"
      local target_path="${WORKFLOWS_DIR}/${file_name}"
      if [ ! -f "${target_path}" ]; then
        run install -m 644 "${workflow_path}" "${target_path}"
      fi
    done < <(find "${BUNDLED_WORKFLOWS_DIR}" -maxdepth 1 -type f | sort)
    ok "Default workflow seeding complete"
  fi

  normalize_host_dashboard_config

  if [ -d "${EMBEDDED_TOOLS_SRC}" ]; then
    log "Installing embedded tool descriptors → ${DATA_DIR}/embedded"
    run cp -r "${EMBEDDED_TOOLS_SRC}/." "${DATA_DIR}/embedded/"
    ok "Embedded tool descriptors installed"
  fi

  # Voice model assets are optional and user-supplied (downloaded via
  # `voxi-cli model install`). When a pre-populated `data/models/voice` tree is
  # present in the checkout, bundle it; otherwise just ensure the target dir and
  # the registry exist so the CLI can populate it later. The voice channel is
  # disabled by default and degrades to null STT/TTS when no models are found,
  # so a missing voice tree never breaks the daemon.
  local VOICE_MODELS_SRC="${PROJECT_DIR}/data/models/voice"
  local VOICE_MODELS_DIR="${DATA_DIR}/models/voice"
  run mkdir -p "${VOICE_MODELS_DIR}"
  if [ -d "${VOICE_MODELS_SRC}" ] && [ -n "$(ls -A "${VOICE_MODELS_SRC}" 2>/dev/null)" ]; then
    log "Bundling voice model assets → ${VOICE_MODELS_DIR}"
    run cp -r "${VOICE_MODELS_SRC}/." "${VOICE_MODELS_DIR}/"
    ok "Voice model assets installed"
  else
    log "No bundled voice models; install later with: voxi-cli model install <id>"
  fi
  if [ -f "${BUNDLED_CONFIG_DIR}/models.voice.json" ]; then
    run install -m 644 "${BUNDLED_CONFIG_DIR}/models.voice.json" \
      "${VOICE_MODELS_DIR}/models.voice.json"
  fi

  ensure_shell_path
}

# ─────────────────────────────────────────────
# Step 3: Run daemon
# ─────────────────────────────────────────────
stop_daemon() {
  force_kill_by_pid() {
    local pid="$1"
    local label="$2"
    if [ -z "${pid}" ]; then
      return 0
    fi
    if kill -0 "${pid}" 2>/dev/null; then
      warn "${label} still running after graceful stop; sending SIGKILL to pid ${pid}"
      run kill -9 "${pid}" || true
      sleep 1
    fi
  }

  if [ -f "${TOOL_EXECUTOR_PID_FILE}" ]; then
    local pid
    pid=$(cat "${TOOL_EXECUTOR_PID_FILE}" 2>/dev/null || true)
    if [ -n "${pid}" ] && kill -0 "${pid}" 2>/dev/null; then
      log "Stopping voxi-tool-executor (pid ${pid})..."
      run kill "${pid}" || true
      sleep 1
      force_kill_by_pid "${pid}" "voxi-tool-executor"
    fi
    rm -f "${TOOL_EXECUTOR_PID_FILE}"
  fi

  if [ -f "${PID_FILE}" ]; then
    local pid
    pid=$(cat "${PID_FILE}" 2>/dev/null || true)
    if [ -n "${pid}" ] && kill -0 "${pid}" 2>/dev/null; then
      log "Stopping voxi daemon (pid ${pid})..."
      run kill "${pid}" || true
      sleep 1
      force_kill_by_pid "${pid}" "voxi"
    fi
    rm -f "${PID_FILE}"
    ok "Daemon stopped"
  else
    warn "No PID file found at ${PID_FILE}. Daemon may not be running."
    # Try by name as fallback
    if pgrep -x "${PKG_NAME}" &>/dev/null; then
      run pkill -x "${PKG_NAME}" || true
      ok "Daemon killed by name"
    fi
  fi

  if pgrep -f "${INSTALL_DIR}/${TOOL_EXECUTOR_NAME}" &>/dev/null; then
    run pkill -f "${INSTALL_DIR}/${TOOL_EXECUTOR_NAME}" || true
  fi
  if pgrep -f "${INSTALL_DIR}/${PKG_NAME}" &>/dev/null; then
    run pkill -f "${INSTALL_DIR}/${PKG_NAME}" || true
  fi
  if pgrep -f "${INSTALL_DIR}/${CLI_NAME}" &>/dev/null; then
    run pkill -f "${INSTALL_DIR}/${CLI_NAME}" || true
  fi
  if pgrep -f "${INSTALL_DIR}/${WEB_DASHBOARD_NAME}" &>/dev/null; then
    run pkill -f "${INSTALL_DIR}/${WEB_DASHBOARD_NAME}" || true
  fi

  if pgrep -x "${TOOL_EXECUTOR_NAME}" &>/dev/null; then
    run pkill -x "${TOOL_EXECUTOR_NAME}" || true
  fi
  if pgrep -x "${CLI_NAME}" &>/dev/null; then
    run pkill -x "${CLI_NAME}" || true
  fi
  if pgrep -x "${WEB_DASHBOARD_NAME}" &>/dev/null; then
    run pkill -x "${WEB_DASHBOARD_NAME}" || true
  fi

  wait_for_process_name_exit "voxi-tool-executor" "${TOOL_EXECUTOR_NAME}" 5 || true
  wait_for_process_name_exit "voxi" "${PKG_NAME}" 5 || true
  wait_for_process_name_exit "voxi-web-dashboard" "${WEB_DASHBOARD_NAME}" 5 || true

  if [ "${DRY_RUN}" = false ]; then
    local remaining
    remaining="$(process_report)"
    if [ -n "${remaining}" ]; then
      warn "Remaining host process entries detected after stop:"
      printf '%s\n' "${remaining}"
    else
      ok "All known host processes were stopped"
    fi
  fi
}

remove_installation() {
  header "Remove Host Installation"

  stop_daemon

  if [ -d "${DATA_DIR}" ]; then
    log "Removing ${DATA_DIR}"
    run rm -rf "${DATA_DIR}"
    ok "Removed host data tree"
  else
    warn "Host data tree not found: ${DATA_DIR}"
  fi

  if [ -d "${BUILD_ROOT_DIR}" ]; then
    log "Removing host build tree ${BUILD_ROOT_DIR}"
    run rm -rf "${BUILD_ROOT_DIR}"
    ok "Removed host build tree"
  fi

  for legacy_bin in "${PKG_NAME}" "${TOOL_EXECUTOR_NAME}" "${CLI_NAME}" "${WEB_DASHBOARD_NAME}"; do
    if [ -f "${INSTALL_DIR}/${legacy_bin}" ]; then
      log "Removing binary ${INSTALL_DIR}/${legacy_bin}"
      run rm -f "${INSTALL_DIR}/${legacy_bin}"
    fi
  done

  if [ -f "${BASHRC_PATH}" ] && grep -Fqx "${PATH_EXPORT}" "${BASHRC_PATH}" 2>/dev/null; then
    log "Removing PATH export from ${BASHRC_PATH}"
    if [ "${DRY_RUN}" = false ]; then
      grep -Fvx "${PATH_EXPORT}" "${BASHRC_PATH}" > "${BASHRC_PATH}.tmp" || true
      mv "${BASHRC_PATH}.tmp" "${BASHRC_PATH}"
      # Sourcing is not needed on uninstall
      :
    else
      echo -e "  ${YELLOW}[DRY-RUN]${NC} remove '${PATH_EXPORT}' from '${BASHRC_PATH}'"
    fi
    ok "Removed PATH export from ~/.bashrc"
  fi
}

show_status() {
  header "Daemon Status"
  local host_dashboard_port
  host_dashboard_port="$(dashboard_port)"

  if [ -f "${PID_FILE}" ]; then
    local pid
    pid=$(cat "${PID_FILE}" 2>/dev/null || true)
    if [ -n "${pid}" ] && kill -0 "${pid}" 2>/dev/null; then
      ok "voxi is running (pid ${pid})"
    else
      warn "PID file exists but process ${pid} is not running"
    fi
  else
    warn "voxi is not running (no PID file)"
  fi

  if [ -f "${TOOL_EXECUTOR_PID_FILE}" ]; then
    local pid
    pid=$(cat "${TOOL_EXECUTOR_PID_FILE}" 2>/dev/null || true)
    if [ -n "${pid}" ] && kill -0 "${pid}" 2>/dev/null; then
      ok "voxi-tool-executor is running (pid ${pid})"
    else
      warn "tool-executor PID file exists but process is not running"
    fi
  fi

  if pgrep -f "${INSTALL_DIR}/${WEB_DASHBOARD_NAME}" >/dev/null 2>&1 || pgrep -x "${WEB_DASHBOARD_NAME}" >/dev/null 2>&1; then
    ok "voxi-web-dashboard is running"
  else
    warn "voxi-web-dashboard is not running"
  fi

  local dashboard_listeners
  dashboard_listeners="$(port_report "${host_dashboard_port}")"
  if [ -n "${dashboard_listeners}" ]; then
    log "Port ${host_dashboard_port} listeners:"
    printf '%s\n' "${dashboard_listeners}"
  else
    log "Port ${host_dashboard_port} has no active listeners"
  fi

  local dashboard_zombies
  local ps_format="pid,ppid,stat,cmd"
  if [ "$(uname)" = "Darwin" ]; then
    ps_format="pid,ppid,state,command"
  fi
  dashboard_zombies="$(ps -eo "${ps_format}" | grep '\[voxi-web-d\] <defunct>' | grep -v grep || true)"
  if [ -n "${dashboard_zombies}" ]; then
    warn "Detected defunct dashboard process entries:"
    printf '%s\n' "${dashboard_zombies}"
  fi

  if [ -f "${LOG_DIR}/voxi.log" ]; then
    echo ""
    log "Recent logs (last 20 lines):"
    tail -20 "${LOG_DIR}/voxi.log" 2>/dev/null || true
  fi
}

follow_log() {
  local log_file="${LOG_DIR}/voxi.log"
  if [ ! -f "${log_file}" ]; then
    fail "Log file not found: ${log_file}"
  fi
  log "Following log: ${log_file} (Ctrl+C to stop)"
  tail -F "${log_file}"
}

do_run() {
  header "Step 3/3: Start Host Daemon"
  local host_dashboard_port
  host_dashboard_port="$(dashboard_port)"

  if [ -n "${LLM_CONFIG}" ]; then
    if [ ! -f "${LLM_CONFIG}" ]; then
      fail "llm_config.json not found: ${LLM_CONFIG}"
    fi
    log "Linking custom LLM config → ${CONFIG_DIR}/llm_config.json"
    mkdir -p "${CONFIG_DIR}"
    ln -sf "${LLM_CONFIG}" "${CONFIG_DIR}/llm_config.json"
  fi
  export VOXI_DATA_DIR="${DATA_DIR}"
  export VOXI_TOOLS_DIR="${TOOLS_DIR}"
  export PATH="${INSTALL_DIR}:${PATH}"
  export LD_LIBRARY_PATH="${LIB_DIR}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"
  export PKG_CONFIG_PATH="${PKGCONFIG_DIR}${PKG_CONFIG_PATH:+:${PKG_CONFIG_PATH}}"

  # Stop existing instance if running
  stop_daemon
  if [ "${DRY_RUN}" = false ]; then
    process_report || true
  fi

  warn_if_dashboard_port_busy "${host_dashboard_port}"

  log "Starting ${TOOL_EXECUTOR_NAME}..."
  local daemon_runner=()
  local runner_name="nohup"
  if command -v setsid &>/dev/null; then
    daemon_runner+=("setsid")
    runner_name="setsid"
  elif command -v nohup &>/dev/null; then
    daemon_runner+=("nohup")
    runner_name="nohup"
  fi

  if [ "${DRY_RUN}" = false ]; then
    "${daemon_runner[@]}" "${INSTALL_DIR}/${TOOL_EXECUTOR_NAME}" \
      >> "${LOG_DIR}/voxi-tool-executor.log" 2>&1 < /dev/null &
    echo $! > "${TOOL_EXECUTOR_PID_FILE}"
  else
    echo -e "  ${YELLOW}[DRY-RUN]${NC} ${runner_name} ${INSTALL_DIR}/${TOOL_EXECUTOR_NAME} >> ${LOG_DIR}/voxi-tool-executor.log 2>&1 < /dev/null &"
  fi
  ok "${TOOL_EXECUTOR_NAME} started"

  log "Starting ${PKG_NAME} daemon..."
  if [ "${DRY_RUN}" = false ]; then
    "${daemon_runner[@]}" "${INSTALL_DIR}/${PKG_NAME}" \
      >> "${LOG_DIR}/voxi.stdout.log" 2>&1 < /dev/null &
    echo $! > "${PID_FILE}"
    sleep 1
    if kill -0 "$(cat "${PID_FILE}")" 2>/dev/null; then
      ok "${PKG_NAME} daemon started (pid $(cat "${PID_FILE}"))"
    else
      fail "${PKG_NAME} daemon failed to start — inspect ${LOG_DIR}/voxi.stdout.log"
    fi
  else
    echo -e "  ${YELLOW}[DRY-RUN]${NC} ${runner_name} ${INSTALL_DIR}/${PKG_NAME} >> ${LOG_DIR}/voxi.stdout.log 2>&1 < /dev/null &"
  fi
}

# ─────────────────────────────────────────────
# Main execution flow
# ─────────────────────────────────────────────
main() {
  parse_args "$@"

  if [ "${REMOVE_INSTALL}" = true ]; then
    remove_installation
    exit 0
  fi

  if [ "${STOP_DAEMON}" = true ]; then
    stop_daemon
    exit 0
  fi

  if [ "${SHOW_STATUS}" = true ]; then
    show_status
    exit 0
  fi

  if [ "${FOLLOW_LOG}" = true ]; then
    follow_log
    exit 0
  fi

  if [ "${RESTART_ONLY}" = true ]; then
    do_run
    exit 0
  fi

  if [ "${RUN_TESTS}" = true ]; then
    do_test
    exit 0
  fi

  check_prerequisites
  do_build

  if [ "${BUILD_ONLY}" = true ]; then
    exit 0
  fi

  do_install
  do_run
}

main "$@"
