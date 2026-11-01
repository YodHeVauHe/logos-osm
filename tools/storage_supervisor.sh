#!/usr/bin/env bash
# Keep a logosctl Storage session up. Checks every INTERVAL seconds and only acts
# when something is down: starts the daemon if it is not running, and calls
# storage_module start if the module reports isRunning=false. A healthy node is
# never restarted. Run it detached so it survives the shell:
#   setsid nohup tools/storage_supervisor.sh >> "$LOG" 2>&1 < /dev/null &
# On a host with systemd, prefer the unit in docs/coverage-testnet-v0.3.md.
set -u
LOGOSCTL="${LOGOSCTL_BIN:-logosctl}"
CFG="${LOGOSCTL_CONFIG_DIR:?set LOGOSCTL_CONFIG_DIR to the session directory}"
INTERVAL="${INTERVAL:-60}"
ctl() { "$LOGOSCTL" --config-dir "$CFG" "$@"; }
ts() { date '+%Y-%m-%dT%H:%M:%S%z'; }
echo "$(ts) supervisor up: config-dir=$CFG interval=${INTERVAL}s pid=$$"
while true; do
  if ! ctl status 2>/dev/null | grep -q '"status":"running"'; then
    echo "$(ts) daemon not running; starting it"
    ctl daemon start --detach || echo "$(ts) daemon start failed"
    sleep 15
  fi
  if ! ctl call storage_module isRunning 2>/dev/null | grep -q '"result":true'; then
    echo "$(ts) storage_module not running; calling start"
    ctl call storage_module start || echo "$(ts) storage_module start failed"
  fi
  sleep "$INTERVAL"
done
