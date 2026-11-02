#!/usr/bin/env bash
# Hosting-node side of the storage-retrieval-tunnel workflow.
#
# Reads the downloader's tunnel info (WebSocket URL and peer ID) from the
# workflow's `tunnel-info` artifact, bridges a local TCP port to that WebSocket
# with websocat, and asks the local Storage node to dial the downloader through
# it. Only outbound HTTPS is needed on this side, so it works from behind NAT.
#
#   tools/storage_tunnel_peer.sh <run-id>
#
# Environment: LOGOSCTL_BIN, LOGOSCTL_CONFIG_DIR (the hosting session),
# WEBSOCAT (default: websocat), REPO (default: YodHeVauHe/logos-osm),
# LOCAL_PORT (default: 18070).
set -euo pipefail
RUN_ID="${1:?usage: $0 <run-id>}"
REPO="${REPO:-YodHeVauHe/logos-osm}"
WEBSOCAT="${WEBSOCAT:-websocat}"
LOCAL_PORT="${LOCAL_PORT:-18070}"
CTL=("${LOGOSCTL_BIN:-logosctl}" --config-dir "${LOGOSCTL_CONFIG_DIR:?set LOGOSCTL_CONFIG_DIR}")
work="$(mktemp -d)"

for _ in $(seq 90); do
  gh run download "$RUN_ID" -R "$REPO" -n tunnel-info -D "$work" 2>/dev/null && break
  sleep 10
done
[ -s "$work/tunnel.json" ] || { echo "no tunnel-info artifact for run $RUN_ID" >&2; exit 1; }
wss="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["wss"])' "$work/tunnel.json")"
peer="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["peer"])' "$work/tunnel.json")"
echo "downloader peer $peer via $wss"

setsid nohup "$WEBSOCAT" -E -b "tcp-l:127.0.0.1:$LOCAL_PORT" "$wss" > "$work/websocat.log" 2>&1 < /dev/null &
echo "websocat bridge pid $! (log $work/websocat.log)"
sleep 3
echo "hosting node: $("${CTL[@]}" call storage_module peerId)"
"${CTL[@]}" call storage_module connect "$peer" "[\"/ip4/127.0.0.1/tcp/$LOCAL_PORT\"]"
