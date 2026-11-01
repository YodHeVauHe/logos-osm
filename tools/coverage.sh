#!/usr/bin/env bash
# One entry point for the LP-0018 testnet v0.3 coverage run.
#
#   tools/coverage.sh setup      install pinned logosctl 0.3.2 (storage_module 3.0.2),
#                                checksum-verified, and build osm + osm_registry_cli
#                                against LEZ v0.3.1 (pinned by git tag in Cargo.toml)
#   tools/coverage.sh node       init + start a Storage node on logos.test (built-in bootstrap)
#   tools/coverage.sh host       download, prepare, upload, register (idempotent; needs a wallet)
#   tools/coverage.sh verify     keyless: registry from testnet RPC -> fetch CID -> MD5 vs on-chain
#   tools/coverage.sh supervise  start the restart-only-if-down watchdog, detached
#   tools/coverage.sh status     daemon + storage_module + supervisor health
#
# Extra arguments after the subcommand go to the underlying tool.
# Everything is configurable through the environment:
#   OSM_TOOLS_DIR        where logosctl is unpacked   (default ~/.cache/logos-osm/tools)
#   LOGOSCTL_CONFIG_DIR  logosctl session directory    (default ~/.cache/logos-osm/storage-session)
#   STORAGE_DATA_DIR     Storage data directory        (default ~/.cache/logos-osm/storage-data)
#   STORAGE_LISTEN_PORT  fixed TCP listen port          (default: module default)
#   STORAGE_NAT          e.g. extip:203.0.113.7 on a public host (default: module default, auto)
#   CARGO_TARGET_DIR     cargo target dir               (default <repo>/target)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REGIONS_FILE="${REGIONS_FILE:-$ROOT/coverage/regions-testnet-v0.3.json}"
CACHE="${XDG_CACHE_HOME:-$HOME/.cache}/logos-osm"
OSM_TOOLS_DIR="${OSM_TOOLS_DIR:-$CACHE/tools}"
export LOGOSCTL_CONFIG_DIR="${LOGOSCTL_CONFIG_DIR:-$CACHE/storage-session}"
STORAGE_DATA_DIR="${STORAGE_DATA_DIR:-$CACHE/storage-data}"
TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"

pin() { python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["pins"][sys.argv[2]])' "$REGIONS_FILE" "$1"; }
LOGOSCTL_VERSION="$(pin logosctl)"
LOGOSCTL_URL="$(pin logosctl_url)"
TAR_SHA256="$(pin logosctl_tar_sha256)"
APPIMAGE_SHA256="$(pin logosctl_appimage_sha256)"
STORAGE_MODULE_VERSION="$(pin storage_module)"
export LOGOSCTL_BIN="${LOGOSCTL_BIN:-$OSM_TOOLS_DIR/logosctl-$LOGOSCTL_VERSION/squashfs-root/AppRun}"
ctl() { "$LOGOSCTL_BIN" --config-dir "$LOGOSCTL_CONFIG_DIR" "$@"; }
die() { echo "coverage.sh: $*" >&2; exit 1; }

cmd_setup() {
  local dir="$OSM_TOOLS_DIR/logosctl-$LOGOSCTL_VERSION"
  mkdir -p "$dir"
  if [ ! -x "$dir/squashfs-root/AppRun" ]; then
    curl -fL --retry 3 -o "$dir/logosctl.tar.gz" "$LOGOSCTL_URL"
    echo "$TAR_SHA256  $dir/logosctl.tar.gz" | sha256sum -c -
    tar -xzf "$dir/logosctl.tar.gz" -C "$dir"
    local img; img="$(find "$dir" -maxdepth 2 -name 'logosctl-x86_64.AppImage' | head -1)"
    [ -n "$img" ] || die "AppImage not found in tarball"
    echo "$APPIMAGE_SHA256  $img" | sha256sum -c -
    chmod +x "$img"
    (cd "$dir" && "$img" --appimage-extract >/dev/null)
  fi
  "$LOGOSCTL_BIN" --version | grep -q "logosctl version $LOGOSCTL_VERSION" || die "logosctl is not $LOGOSCTL_VERSION"
  local mod; mod="$(find "$dir/squashfs-root" -path '*storage_module*' -name 'manifest.json' | head -1 || true)"
  [ -n "$mod" ] && grep -o '"version"[^,]*' "$mod" | head -1
  echo "logosctl $LOGOSCTL_VERSION ready: $LOGOSCTL_BIN"
  if [ "${SKIP_BUILD:-0}" != 1 ]; then
    cargo build --manifest-path "$ROOT/Cargo.toml" -p osm_cli --bin osm --target-dir "$TARGET"
    cargo build --manifest-path "$ROOT/examples/Cargo.toml" --bin osm_registry_cli --target-dir "$TARGET"
    echo "built: $TARGET/debug/osm $TARGET/debug/osm_registry_cli"
  fi
}

cmd_node() {
  mkdir -p "$LOGOSCTL_CONFIG_DIR" "$STORAGE_DATA_DIR"
  local fresh=0
  ctl status 2>/dev/null | grep -q '"status":"running"' || { ctl daemon start --detach; sleep 15; fresh=1; }
  ctl status | grep -q "\"name\":\"storage_module\",\"status\":\"loaded\"[^}]*\"version\":\"$STORAGE_MODULE_VERSION\"" \
    || die "storage_module $STORAGE_MODULE_VERSION is not loaded (see: $LOGOSCTL_BIN --config-dir $LOGOSCTL_CONFIG_DIR status)"
  if ctl call storage_module isRunning | grep -q '"result":true'; then
    if [ "$fresh" = 0 ]; then
      echo "storage_module already running in an existing session; leaving it alone"; return
    fi
    # The bundled package downloader auto-starts Storage with the default config;
    # stop it so it can be started with ours (per the Logos Storage docs).
    ctl call storage_module stop
    for _ in $(seq 30); do
      sleep 2
      ctl call storage_module isRunning | grep -q '"result":false' && break
    done
    for _ in $(seq 30); do
      ctl call storage_module destroy | grep -q '"success":true' && break
      sleep 2
    done
  fi
  local cfg="$LOGOSCTL_CONFIG_DIR/storage-config.json"
  ctl call storage_module loadConfigOrDefault | python3 -c '
import json, os, sys
c = json.loads(json.load(sys.stdin)["result"]["value"])
c["data-dir"] = os.environ["STORAGE_DATA_DIR"]
if os.environ.get("STORAGE_LISTEN_PORT"): c["listen-port"] = int(os.environ["STORAGE_LISTEN_PORT"])
if os.environ.get("STORAGE_NAT"): c["nat"] = os.environ["STORAGE_NAT"]
print(json.dumps(c))' > "$cfg"
  ctl call storage_module init "@$cfg" | tee /dev/stderr | grep -q '"result":true' || die "storage_module init failed"
  ctl call storage_module start
  sleep 10
  ctl call storage_module isRunning
}

cmd_host() {
  [ -x "$TARGET/debug/osm" ] || die "run setup first"
  python3 "$ROOT/tools/host_coverage_regions.py" \
    --osm "$TARGET/debug/osm" --registry-cli "$TARGET/debug/osm_registry_cli" \
    --logosctl "$LOGOSCTL_BIN" --storage-config "$LOGOSCTL_CONFIG_DIR" \
    --scratch "${COVERAGE_SCRATCH:-$CACHE/scratch}" --regions-file "$REGIONS_FILE" \
    --gas-limit "${GAS_LIMIT:-500000}" "$@"
}

cmd_verify() {
  python3 "$ROOT/tools/verify_coverage.py" --regions-file "$REGIONS_FILE" \
    --logosctl "$LOGOSCTL_BIN" --storage-config "$LOGOSCTL_CONFIG_DIR" "$@"
}

cmd_supervise() {
  local log="$LOGOSCTL_CONFIG_DIR/supervisor.log"
  mkdir -p "$LOGOSCTL_CONFIG_DIR"
  if pgrep -f "storage_supervisor.sh" >/dev/null && grep -qs "config-dir=$LOGOSCTL_CONFIG_DIR " "$log"; then
    echo "supervisor already running for $LOGOSCTL_CONFIG_DIR"; return
  fi
  setsid nohup "$ROOT/tools/storage_supervisor.sh" >> "$log" 2>&1 < /dev/null &
  echo "supervisor started (log: $log)"
}

cmd_status() {
  ctl status || true
  ctl call storage_module isRunning || true
  pgrep -af storage_supervisor.sh || echo "no supervisor running"
}

sub="${1:-}"; shift || true
case "$sub" in
  setup|node|host|verify|supervise|status) "cmd_$sub" "$@" ;;
  *) sed -n '2,21p' "$0"; exit 2 ;;
esac
