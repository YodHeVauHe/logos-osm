#!/usr/bin/env bash
# Deploy the OSM registry to a LEZ v0.4 network, initialize the registry PDA,
# read it back, and write a deployment record.
#
# Usage:
#   tools/deploy_registry.sh --rpc-url URL --payer ACCOUNT_ID [--out DIR]
#                            [--write-wallet-config] [--gas-limit N]
#
# Environment:
#   LEZ_WALLET_BIN        LEZ v0.4 `wallet` binary (default: `wallet` on PATH)
#   LEE_WALLET_HOME_DIR   wallet home that holds the funded payer key (required)
#   LEE_WALLET_PASSWORD   wallet storage password, if the storage is encrypted
#
# The payer must already hold enough native balance for 1 header, N segment
# and 1 init transaction. The script creates fresh header and segment
# accounts in the wallet; their creation output is kept in DIR/private (mode
# 700) and never printed, because `account new` can include a recovery phrase.
# It refuses to run when the wallet's built-in programs do not match the
# network (`wallet check-health`), so a v0.4 build is never pointed at a v0.3
# network by mistake.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
RPC_URL="" PAYER="" OUT="" WRITE_CONFIG=0 GAS_LIMIT=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --rpc-url) RPC_URL=$2; shift 2 ;;
    --payer) PAYER=${2#Public/}; shift 2 ;;
    --out) OUT=$2; shift 2 ;;
    --gas-limit) GAS_LIMIT=$2; shift 2 ;;
    --write-wallet-config) WRITE_CONFIG=1; shift ;;
    -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
[[ -n "$RPC_URL" && -n "$PAYER" ]] || { sed -n '2,20p' "$0" >&2; exit 2; }
: "${LEE_WALLET_HOME_DIR:?set LEE_WALLET_HOME_DIR to the wallet home holding the payer key}"
WALLET_BIN=${LEZ_WALLET_BIN:-wallet}
command -v "$WALLET_BIN" >/dev/null || { echo "wallet binary not found: $WALLET_BIN" >&2; exit 2; }
STAMP=$(date -u +%Y%m%dT%H%M%SZ)
OUT=${OUT:-$ROOT/deploy/$STAMP}
mkdir -p "$OUT/private" && chmod 700 "$OUT/private"
log() { printf '[deploy %s] %s\n' "$(date -u +%H:%M:%SZ)" "$*"; }

# 1. Point the wallet at RPC_URL (only with --write-wallet-config).
CONFIG="$LEE_WALLET_HOME_DIR/wallet_config.json"
current=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["sequencer"]["sequencer_addr"])' "$CONFIG")
if [[ "${current%/}" != "${RPC_URL%/}" ]]; then
  if [[ "$WRITE_CONFIG" != 1 ]]; then
    echo "wallet config points at $current, not $RPC_URL; rerun with --write-wallet-config" >&2
    exit 2
  fi
  cp "$CONFIG" "$OUT/private/wallet_config.json.bak"
  python3 - "$CONFIG" "$RPC_URL" <<'PY'
import json, sys
c = json.load(open(sys.argv[1]))
c["sequencer"]["sequencer_addr"] = sys.argv[2]
c["seq_tx_poll_max_blocks"] = max(c.get("seq_tx_poll_max_blocks", 5), 40)
json.dump(c, open(sys.argv[1], "w"), indent=2)
PY
  log "wallet config now points at $RPC_URL (backup in $OUT/private)"
fi

# 2. The wallet build must match the network.
"$WALLET_BIN" check-health >"$OUT/check-health.log" 2>&1 || {
  cat "$OUT/check-health.log" >&2
  echo "check-health failed: this wallet build does not match $RPC_URL" >&2
  exit 1
}
log "check-health: $(tail -n1 "$OUT/check-health.log")"

# 3. Build the guest and the registry CLI.
cd "$ROOT"
make build >"$OUT/build.log" 2>&1 || { tail -n 40 "$OUT/build.log" >&2; exit 1; }
make cli >"$OUT/cli-build.log" 2>&1 || { tail -n 40 "$OUT/cli-build.log" >&2; exit 1; }
BIN="$ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/osm_registry.bin"
CLI="$ROOT/examples/target/debug/osm_registry_cli"
IMAGE_ID=$(grep -oE 'ProgramId: [0-9a-f]+' "$OUT/build.log" | tail -n1 | cut -d' ' -f2)
GUEST_ELF=$(find methods/target/riscv-guest -type f -name osm_registry | head -1)
# The loader splits the user ELF into 96 KiB segments (MAX_SEGMENT_DATA_LEN).
SEGMENT_COUNT=$(( ($(stat -c %s "$GUEST_ELF") + 98303) / 98304 ))
log "guest image $IMAGE_ID, $SEGMENT_COUNT segments"

# 4. Fresh header and segment accounts.
new_account() {
  "$WALLET_BIN" account new public --label "osm-registry-$STAMP-$1" >"$OUT/private/account-$1.log" 2>&1
  grep -oE 'account_id Public/[1-9A-HJ-NP-Za-km-z]+' "$OUT/private/account-$1.log" | head -1 | cut -d/ -f2
}
HEADER=$(new_account header)
SEGMENTS=()
for i in $(seq 1 "$SEGMENT_COUNT"); do SEGMENTS+=("$(new_account "segment-$i")"); done
log "header $HEADER"

# 5. Deploy (immutable header).
"$WALLET_BIN" program-loader deploy --elf "$BIN" --header "Public/$HEADER" \
  --segments "${SEGMENTS[@]/#/Public/}" --immutable --payer "Public/$PAYER" >"$OUT/deploy.log" 2>&1 || {
  tail -n 40 "$OUT/deploy.log" >&2; exit 1; }
grep -q "Program deployed. Header account id: $HEADER" "$OUT/deploy.log"
log "deployed"

# 6. Initialize and read back the registry PDA.
export OSM_RPC_URL="$RPC_URL" OSM_PROGRAM_ID="$HEADER"
[[ -n "$GAS_LIMIT" ]] && export OSM_GAS_LIMIT="$GAS_LIMIT"
PDA=$("$CLI" pda)
"$CLI" init --payer "$PAYER" >"$OUT/init.log" 2>&1 || { cat "$OUT/init.log" >&2; exit 1; }
RAW=$("$CLI" get --raw)
[[ "$RAW" != null ]] || { echo "registry PDA $PDA is still empty after init (reverted?)" >&2; exit 1; }
log "registry PDA $PDA initialized ($RAW)"

# 7. Deployment record (public data only).
python3 - "$OUT" "$RPC_URL" "$HEADER" "$PDA" "$IMAGE_ID" "$STAMP" "$(git rev-parse HEAD)" "${SEGMENTS[@]}" <<'PY'
import json, re, sys, pathlib
out, rpc, header, pda, image, stamp, commit, *segments = sys.argv[1:]
d = pathlib.Path(out)
def txs(name):
    text = (d / name).read_text()
    return re.findall(r"Transaction: ([0-9a-f]{64})", text) or re.findall(r"\b([0-9a-f]{64})\b", text)
blocks = re.findall(r"Transaction is included in block (\d+)", (d / "deploy.log").read_text())
record = {"rpc": rpc, "program_id": header, "registry_pda": pda, "image_id": image,
          "segments": segments, "deploy_blocks": blocks, "init_tx": txs("init.log"),
          "deployed_at_utc": stamp, "logos_osm_commit": commit}
(d / "deployment.json").write_text(json.dumps(record, indent=2) + "\n")
print(json.dumps(record, indent=2))
PY
log "record: $OUT/deployment.json"
echo "Next: register regions with osm (OSM_PROGRAM_ID=$HEADER OSM_RPC_URL=$RPC_URL), e.g. 'osm host <region>...'."
