#!/usr/bin/env bash
set -euo pipefail

# Run a deployed registry against the LEZ v0.4 standalone sequencer.
# Wallet output is captured because `account new` includes a recovery phrase.
ROOT=$(cd "$(dirname "$0")/.." && pwd)
LEZ_ROOT=${LEZ_ROOT:-/tmp/lez}
WALLET_BIN=${LEZ_WALLET_BIN:-$LEZ_ROOT/target/debug/wallet}
SEQUENCER_BIN=${LEZ_SEQUENCER_BIN:-$LEZ_ROOT/target/debug/sequencer_service}
RPC_URL=${OSM_RPC_URL:-http://127.0.0.1:3040}
# The local Bedrock funding key from LEZ's Justfile; standalone mode mocks Bedrock.
BEDROCK_FUNDING_PUBLIC_KEY=2e03b2eff5a45478e7e79668d2a146cf2c5c7925bce927f2b1c67f2ab4fc0d26
TMP_HOME=$(mktemp -d "${TMPDIR:-/tmp}/osm-lez-e2e.XXXXXX")
chmod 700 "$TMP_HOME"
cleanup() {
  if [[ -n "${FIXTURE_PID:-}" ]]; then kill "$FIXTURE_PID" 2>/dev/null || true; fi
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
# v0.4 wallets ask for a storage password; empty keeps this throwaway wallet unencrypted.
export LEE_WALLET_PASSWORD=""

# Prints "<account id> <public key hex>" for a new public account.
new_account() {
  local log_path="$TMP_HOME/account-$1.log"
  "$WALLET_BIN" account new public --label "lp0018-$1" >"$log_path" 2>&1 || { grep -v -i "recovery\|mnemonic" "$log_path" >&2; exit 1; }
  python3 - "$log_path" <<'PY'
import re, sys
text = open(sys.argv[1], encoding="utf-8").read()
match = re.search(r"Generated new account with account_id Public/([1-9A-HJ-NP-Za-km-z]+)", text)
key = re.search(r"With pk ([0-9a-f]{64})", text)
if not match or not key:
    raise SystemExit("wallet did not report a generated public account and key")
print(match.group(1), key.group(1))
PY
}

read -r PAYER PAYER_KEY < <(new_account payer)
read -r HEADER _ < <(new_account header)
SEGMENTS=()
for index in 1 2 3 4; do read -r seg _ < <(new_account "segment-$index"); SEGMENTS+=("$seg"); done

# Fund the throwaway payer at genesis (v0.4: deposit recipient by public key).
python3 - "$LEZ_ROOT/lez/sequencer/service/networks/local.json" "$TMP_HOME/local.json" "$PAYER_KEY" <<'PY'
import json, sys
config = json.load(open(sys.argv[1], encoding="utf-8"))
config["genesis"]["actions"].append({"supply_account": {
    "recipient": {"identified": {"key": sys.argv[3]}}, "balance": 10000000000000}})
json.dump(config, open(sys.argv[2], "w", encoding="utf-8"))
PY

# This config is local-only.
"$SEQUENCER_BIN" setup "$TMP_HOME/sequencer" --config "$TMP_HOME/local.json" \
  --funding-public-key "$BEDROCK_FUNDING_PUBLIC_KEY" --create-channel >"$TMP_HOME/setup.log" 2>&1 || { cat "$TMP_HOME/setup.log"; exit 1; }
RUST_LOG=${RUST_LOG:-info,risc0_zkvm=warn,kameo=warn} "$SEQUENCER_BIN" start "$TMP_HOME/sequencer" \
  --rpc-listen-address 127.0.0.1:3040 >"$TMP_HOME/sequencer.log" 2>&1 &
SEQUENCER_PID=$!
for _ in $(seq 1 120); do
  if "$WALLET_BIN" chain-info current-block-id >/dev/null 2>&1; then break; fi
  kill -0 "$SEQUENCER_PID" 2>/dev/null || { cat "$TMP_HOME/sequencer.log"; exit 1; }
  sleep 1
done
# Genesis credits land in the first blocks; wait until the payer is funded.
for _ in $(seq 1 60); do
  if "$WALLET_BIN" account get --account-id "Public/$PAYER" 2>/dev/null | grep -qiE '"?balance"?: *[1-9]'; then break; fi
  sleep 2
done

"$WALLET_BIN" program-loader deploy \
  --elf "$ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/osm_registry.bin" \
  --header "Public/$HEADER" --segments "${SEGMENTS[@]/#/Public/}" \
  --immutable --payer "Public/$PAYER" >"$TMP_HOME/deploy.log" 2>&1 || { cat "$TMP_HOME/deploy.log"; tail -n 100 "$TMP_HOME/sequencer.log"; exit 1; }

cd "$ROOT"
make cli
export LEZ_LOCALNET=1 OSM_RPC_URL="$RPC_URL" OSM_PROGRAM_ID="$HEADER"
export OSM_CLI_BIN="$ROOT/examples/target/debug/osm_registry_cli"
export OSM_SIGNER_ACCOUNT_ID="$PAYER" RISC0_DEV_MODE=1
RUSTUP_TOOLCHAIN=${RUSTUP_TOOLCHAIN:-1.98.1} cargo test -p osm_e2e --features e2e localnet_registry_host_query -- --nocapture

# Real Logos Storage leg (OSM_E2E_STORAGE=1): a local logosctl/storage_module
# node started by `tools/coverage.sh node` stores and serves every object.
fixture_md5() { curl -fsS "$1/$2-latest.osm.pbf.md5" | cut -d' ' -f1; }

check_entry() { # lookup JSON file, region, expected md5
  python3 - "$@" <<'PY'
import json, sys
entries = json.load(open(sys.argv[1], encoding="utf-8"))
assert len(entries) == 1, f"expected one entry for {sys.argv[2]}, got {entries}"
e = entries[0]
assert e["region"] == sys.argv[2], e
assert e["checksum"] == sys.argv[3], f"{sys.argv[2]}: checksum {e['checksum']} != {sys.argv[3]}"
assert e["hosted"] is True, e
assert e["cid"], e
print(f"registry: {e['region']} cid={e['cid']} checksum={e['checksum']} version={e['version']}")
PY
}

download_and_check() { # base, region, geofabrik path
  local out="$WORK/dl-${2//\//_}"
  mkdir -p "$out"
  run_osm download "$2" --dest "$out" | tee "$WORK/download.out"
  grep -q "from LogosStorage" "$WORK/download.out" || { echo "$2 was not served by Logos Storage" >&2; exit 1; }
  grep -q "checksum verified: true" "$WORK/download.out" || { echo "$2 checksum not verified" >&2; exit 1; }
  local files=("$out"/*)
  [[ ${#files[@]} -eq 1 ]] || { echo "expected one downloaded file for $2" >&2; exit 1; }
  local got want
  got=$(md5sum "${files[0]}" | cut -d' ' -f1)
  want=$(fixture_md5 "$1" "$3")
  [[ "$got" == "$want" ]] || { echo "$2: downloaded md5 $got != $want" >&2; exit 1; }
  echo "download: $2 md5 $got matches"
}

storage_cli_e2e() {
  : "${LOGOSCTL_BIN:?OSM_E2E_STORAGE=1 needs LOGOSCTL_BIN}" "${LOGOSCTL_CONFIG_DIR:?OSM_E2E_STORAGE=1 needs LOGOSCTL_CONFIG_DIR}"
  "$LOGOSCTL_BIN" --config-dir "$LOGOSCTL_CONFIG_DIR" call storage_module isRunning | grep -q '"result":true'

  # SDK roundtrip: host -> real Storage -> real registry -> download by CID.
  RUSTUP_TOOLCHAIN=${RUSTUP_TOOLCHAIN:-1.98.1} cargo test -p osm_e2e --features e2e localnet_host_query_download -- --nocapture

  # CLI: bulk host with --exclude (one register_batch), local import, lookup, download.
  RUSTUP_TOOLCHAIN=${RUSTUP_TOOLCHAIN:-1.98.1} cargo build -p osm_cli --bin osm
  WORK="$TMP_HOME/cli-e2e"
  mkdir -p "$WORK"
  python3 "$ROOT/tools/e2e_geofabrik_fixture.py" "$WORK/port" 2>"$WORK/fixture.log" &
  FIXTURE_PID=$!
  for _ in $(seq 50); do [[ -s "$WORK/port" ]] && break; sleep 0.2; done
  local base="http://127.0.0.1:$(cat "$WORK/port")"
  printf 'cli_bin = "%s"\n' "$OSM_CLI_BIN" >"$WORK/osm.toml"
  run_osm() { OSM_SIGNER="$OSM_SIGNER_ACCOUNT_ID" "$ROOT/target/debug/osm" --config "$WORK/osm.toml" --geofabrik-base "$base" "$@"; }

  run_osm host greece chile belarus --exclude belarus >"$WORK/host.out" 2>"$WORK/host.err" || { cat "$WORK/host.out" "$WORK/host.err"; exit 1; }
  cat "$WORK/host.out" "$WORK/host.err"
  grep -q "2 registered in one batch tx, 0 skipped" "$WORK/host.out"
  local txs
  txs=$(grep -E "Transaction:|Submitted:" "$WORK/host.err" | grep -oE "[0-9a-fA-F]{64}" | sort -u | wc -l)
  [[ "$txs" -eq 1 ]] || { echo "bulk host must submit exactly one transaction, saw $txs" >&2; exit 1; }
  for pair in greece:europe/greece chile:south-america/chile; do
    run_osm lookup --region "${pair%%:*}" --json >"$WORK/lookup.json"
    check_entry "$WORK/lookup.json" "${pair%%:*}" "$(fixture_md5 "$base" "${pair#*:}")"
  done
  run_osm lookup --region belarus --json >"$WORK/lookup.json"
  python3 -c 'import json,sys; assert json.load(open(sys.argv[1])) == [], "excluded region was registered"' "$WORK/lookup.json"
  echo "registry: belarus absent after --exclude"

  # Local import: a tampered file is refused (exit 2) and nothing is registered;
  # the genuine file is verified against the published MD5, stored, registered.
  curl -fsSL "$base/europe/belarus-latest.osm.pbf" -o "$WORK/belarus.osm.pbf"
  cp "$WORK/belarus.osm.pbf" "$WORK/belarus-tampered.osm.pbf"
  printf 'x' >>"$WORK/belarus-tampered.osm.pbf"
  set +e; run_osm import belarus "$WORK/belarus-tampered.osm.pbf"; rc=$?; set -e
  [[ "$rc" -eq 2 ]] || { echo "tampered import must exit 2 (checksum mismatch), got $rc" >&2; exit 1; }
  run_osm lookup --region belarus --json >"$WORK/lookup.json"
  python3 -c 'import json,sys; assert json.load(open(sys.argv[1])) == [], "tampered import was registered"' "$WORK/lookup.json"
  run_osm import belarus "$WORK/belarus.osm.pbf" | tee "$WORK/import.out"
  grep -q "^imported belarus cid=.* hosted=true" "$WORK/import.out"
  run_osm lookup --region belarus --json >"$WORK/lookup.json"
  check_entry "$WORK/lookup.json" belarus "$(fixture_md5 "$base" europe/belarus)"

  download_and_check "$base" greece europe/greece
  download_and_check "$base" chile south-america/chile
  download_and_check "$base" belarus europe/belarus
  echo "storage e2e: bulk host, local import, lookup and download passed against the real sequencer and Storage node"
}

if [[ "${OSM_E2E_STORAGE:-0}" == 1 ]]; then storage_cli_e2e; fi
cat "$TMP_HOME/sequencer.log"
