#!/usr/bin/env bash
# Hosting-node side of the storage-retrieval-tunnel workflow.
#
# Reads the downloader's tunnel info (WebSocket URL, peer ID, SPR, port) from
# the workflow's `tunnel-info` artifact and bridges 127.0.0.1:<port> to that
# WebSocket with websocat. The runner's SPR advertises 127.0.0.1:<port>, so
# re-initialising the local Storage node with that SPR as its bootstrap node
# makes it dial the downloader through the bridge. Only outbound HTTPS is needed
# on this side, so it works from behind NAT. (logosctl 0.3.2 cannot pass the
# [tstr] argument of storage_module connect, which is why the bootstrap path is
# used.) The node keeps its data dir, so content it already stores stays put.
#
#   tools/storage_tunnel_peer.sh <run-id>
#
# Environment: LOGOSCTL_BIN, LOGOSCTL_CONFIG_DIR (the hosting session),
# WEBSOCAT (default: websocat), REPO (default: YodHeVauHe/logos-osm).
set -euo pipefail
RUN_ID="${1:?usage: $0 <run-id>}"
REPO="${REPO:-YodHeVauHe/logos-osm}"
WEBSOCAT="${WEBSOCAT:-websocat}"
CTL=("${LOGOSCTL_BIN:-logosctl}" --config-dir "${LOGOSCTL_CONFIG_DIR:?set LOGOSCTL_CONFIG_DIR}")
work="$(mktemp -d)"
field() { python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))[sys.argv[2]])' "$work/tunnel.json" "$1"; }

for _ in $(seq 90); do
  gh run download "$RUN_ID" -R "$REPO" -n tunnel-info -D "$work" 2>/dev/null && break
  sleep 10
done
[ -s "$work/tunnel.json" ] || { echo "no tunnel-info artifact for run $RUN_ID" >&2; exit 1; }
wss="$(field wss)"; peer="$(field peer)"; spr="$(field spr)"; port="$(field port)"
echo "downloader peer $peer via $wss (port $port)"

setsid nohup "$WEBSOCAT" -E -b "tcp-l:127.0.0.1:$port" "$wss" > "$work/websocat.log" 2>&1 < /dev/null &
echo "websocat bridge pid $! (log $work/websocat.log)"
sleep 3

"${CTL[@]}" call storage_module loadConfigOrDefault | SPR="$spr" python3 -c '
import json, os, sys
c = json.loads(json.load(sys.stdin)["result"]["value"])
c["bootstrap-node"] = [os.environ["SPR"]]
print(json.dumps(c))' > "$work/storage-config.json"
"${CTL[@]}" call storage_module stop
for _ in $(seq 30); do
  sleep 2
  "${CTL[@]}" call storage_module isRunning | grep -q '"result":false' && break
done
for _ in $(seq 30); do
  "${CTL[@]}" call storage_module destroy | grep -q '"success":true' && break
  sleep 2
done
"${CTL[@]}" call storage_module init "@$work/storage-config.json" | grep -q '"result":true' || { echo "init failed" >&2; exit 1; }
"${CTL[@]}" call storage_module start
sleep 10
echo "hosting node: $("${CTL[@]}" call storage_module peerId)"
"${CTL[@]}" call storage_module manifests | python3 -c 'import json,sys; [print("serves", m["cid"], m["datasetSize"]) for m in json.load(sys.stdin)["result"]["value"]]'
