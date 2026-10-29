#!/usr/bin/env bash
set -euo pipefail

# Run a deployed registry against the LEZ v0.3.1 standalone sequencer.
# Wallet output is captured because `account new` includes a recovery phrase.
ROOT=$(cd "$(dirname "$0")/.." && pwd)
LEZ_ROOT=${LEZ_ROOT:-/tmp/lez}
WALLET_BIN=${LEZ_WALLET_BIN:-$LEZ_ROOT/target/debug/wallet}
SEQUENCER_BIN=${LEZ_SEQUENCER_BIN:-$LEZ_ROOT/target/debug/sequencer_service}
RPC_URL=${OSM_RPC_URL:-http://127.0.0.1:3040}
TMP_HOME=$(mktemp -d /tmp/osm-lez-e2e.XXXXXX)
chmod 700 "$TMP_HOME"
cleanup() {
  if [[ -n "${SEQUENCER_PID:-}" ]]; then kill "$SEQUENCER_PID" 2>/dev/null || true; wait "$SEQUENCER_PID" 2>/dev/null || true; fi
  rm -rf "$TMP_HOME"
}
trap cleanup EXIT

test -x "$WALLET_BIN" && test -x "$SEQUENCER_BIN"
test -f "$ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/osm_registry.bin"
mkdir -p "$TMP_HOME/wallet"
cp "$LEZ_ROOT/lez/wallet/configs/debug/wallet_config.json" "$TMP_HOME/wallet/wallet_config.json"
cp "$TMP_HOME/wallet/wallet_config.json" "$TMP_HOME/wallet/config.json"
export LEE_WALLET_HOME_DIR="$TMP_HOME/wallet"

new_account() {
  local log_path="$TMP_HOME/account-$1.log"
  "$WALLET_BIN" account new public --label "lp0018-$1" >"$log_path" 2>&1
  python3 - "$log_path" <<'PY'
import re, sys
text = open(sys.argv[1], encoding="utf-8").read()
match = re.search(r"Generated new account with account_id Public/([1-9A-HJ-NP-Za-km-z]+)", text)
if not match:
    raise SystemExit("wallet did not report a generated public account")
print(match.group(1))
PY
}

PAYER=$(new_account payer)
HEADER=$(new_account header)
SEGMENTS=()
for index in 1 2 3 4; do SEGMENTS+=("$(new_account "segment-$index")"); done

python3 - "$LEZ_ROOT/lez/sequencer/service/configs/debug/sequencer_config.json" "$TMP_HOME/sequencer.json" "$TMP_HOME/sequencer" "$PAYER" <<'PY'
import json, sys
config = json.load(open(sys.argv[1], encoding="utf-8"))
config["home"] = sys.argv[3]
config["genesis"].append({"supply_account": {"account_id": sys.argv[4], "balance": 10000000000000}})
json.dump(config, open(sys.argv[2], "w", encoding="utf-8"))
PY

# This config is local-only; the funded throwaway payer is supplied at genesis.
"$SEQUENCER_BIN" "$TMP_HOME/sequencer.json" --listen-address 127.0.0.1 >"$TMP_HOME/sequencer.log" 2>&1 &
SEQUENCER_PID=$!
for _ in $(seq 1 90); do
  if "$WALLET_BIN" chain-info current-block-id >/dev/null 2>&1; then break; fi
  kill -0 "$SEQUENCER_PID" 2>/dev/null || { cat "$TMP_HOME/sequencer.log"; exit 1; }
  sleep 1
done

"$WALLET_BIN" program-loader deploy \
  --elf "$ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/osm_registry.bin" \
  --header "Public/$HEADER" --segments "${SEGMENTS[@]/#/Public/}" \
  --immutable --payer "Public/$PAYER" >"$TMP_HOME/deploy.log" 2>&1

cd "$ROOT"
make cli
export LEZ_LOCALNET=1 OSM_RPC_URL="$RPC_URL" OSM_PROGRAM_ID="$HEADER"
export OSM_CLI_BIN="$ROOT/examples/target/debug/osm_registry_cli"
export OSM_SIGNER_ACCOUNT_ID="$PAYER" RISC0_DEV_MODE=1
RUSTUP_TOOLCHAIN=${RUSTUP_TOOLCHAIN:-1.98.1} cargo test -p osm_e2e --features e2e localnet_registry_host_query -- --nocapture
cat "$TMP_HOME/sequencer.log"
