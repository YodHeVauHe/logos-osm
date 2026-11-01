#!/usr/bin/env python3
"""Real Geofabrik -> Logos Storage -> LEZ v0.3 coverage runner.

All state and transient PBF files stay under --scratch. Requires built `osm`
and `osm_registry_cli` binaries and the configured real Logos Storage daemon.
"""
import argparse
import hashlib
import email.utils
import json
import os
import re
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_REGIONS_FILE = ROOT / "coverage" / "regions-testnet-v0.3.json"


def load_selection(path):
    """Region list and chain pins from the checked-in coverage data file."""
    data = json.loads(Path(path).read_text())
    return [r["region"] for r in data["regions"]], data


def digest(path):
    md5, sha, size = hashlib.md5(), hashlib.sha256(), 0
    with path.open("rb") as f:
        while True:
            chunk = f.read(1024 * 1024)
            if not chunk:
                break
            md5.update(chunk); sha.update(chunk); size += len(chunk)
    return size, md5.hexdigest(), sha.hexdigest()


def fetch(url, dest):
    req = urllib.request.Request(url, headers={"User-Agent": "logos-osm-regions/coverage-evidence"})
    with urllib.request.urlopen(req, timeout=180) as response, dest.open("wb") as out:
        final_url = response.geturl()
        last_modified = response.headers.get("Last-Modified", "")
        while True:
            chunk = response.read(1024 * 1024)
            if not chunk:
                break
            out.write(chunk)
    match = re.search(r"-(\d{6})\.osm\.pbf$", final_url)
    version = match.group(1) if match else ""
    if not version and last_modified:
        try:
            version = email.utils.parsedate_to_datetime(last_modified).strftime("%y%m%d")
        except (TypeError, ValueError):
            pass
    if not version:
        raise RuntimeError(f"cannot determine dated Geofabrik snapshot version (URL={final_url}, Last-Modified={last_modified})")
    return version, final_url


def fetch_with_retry(url, dest, attempts=4):
    """Geofabrik transfers occasionally stall mid-body; drop the partial file and retry."""
    for attempt in range(1, attempts + 1):
        try:
            return fetch(url, dest)
        except (OSError, TimeoutError) as exc:
            dest.unlink(missing_ok=True)
            if attempt == attempts:
                raise
            print(f"RETRY download {url}: {exc} (attempt {attempt}/{attempts})", flush=True)
            time.sleep(10 * attempt)


def published_md5(url):
    with urllib.request.urlopen(url + ".md5", timeout=60) as response:
        raw = response.read().decode("ascii", "replace").strip()
    match = re.search(r"\b([0-9a-fA-F]{32})\b", raw)
    if not match:
        raise RuntimeError(f"could not parse Geofabrik MD5 sidecar: {raw[:200]}")
    return match.group(1).lower()


def run(cmd, env, capture=False, stdin_file=None):
    if stdin_file:
        with stdin_file.open("rb") as secret_input:
            result = subprocess.run(cmd, check=False, text=True, env=env, stdin=secret_input,
                                    stdout=subprocess.PIPE if capture else None,
                                    stderr=subprocess.PIPE if capture else None)
    else:
        result = subprocess.run(cmd, check=False, text=True, env=env,
                                stdout=subprocess.PIPE if capture else None,
                                stderr=subprocess.PIPE if capture else None)
    if result.returncode:
        detail = result.stderr or ""
        if stdin_file:
            secret = stdin_file.read_text().strip()
            if secret:
                detail = detail.replace(secret, "[REDACTED]")
        raise RuntimeError(f"{Path(cmd[0]).name} exited {result.returncode}: {detail[-1200:]}")
    return result


def rpc_call(rpc, method, params):
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode()
    req = urllib.request.Request(rpc, data=body, headers={"content-type": "application/json"})
    with urllib.request.urlopen(req, timeout=60) as response:
        reply = json.load(response)
    if "error" in reply:
        raise RuntimeError(f"{method}: {reply['error']}")
    return reply.get("result")


def included_block(rpc, tx_hash):
    """Block id the sequencer reports for tx_hash, or None while it is not included."""
    result = rpc_call(rpc, "getTransaction", [tx_hash])
    return result[1] if isinstance(result, list) and len(result) > 1 else None


def read_registry(args, env):
    raw = run([str(args.registry_cli), "--rpc-url", args.rpc, "--program-id", args.program_id, "get"],
              env, True, args.password_file)
    entries = json.loads(raw.stdout or "{}") or {}
    if isinstance(entries, list):
        entries = {e["region"]: e for e in entries}
    return entries, raw.stdout


def entry_matches(actual, expected):
    return bool(actual) and all(actual.get(k) == expected.get(k) for k in ("cid", "checksum", "hosted"))


def confirm_registration(region, tx_hash, rec, args, env, scratch, attempts):
    """Poll the sequencer until tx_hash is included, then require a matching registry read-back.

    The wallet's own poll gives up after a few blocks, so a transaction it reports
    as "not found" can still land; this check is what decides success.
    """
    for i in range(attempts):
        if i:
            time.sleep(15)
        try:
            block = included_block(args.rpc, tx_hash)
        except Exception as exc:  # transient RPC failure: retry
            print(f"WAIT {region}: getTransaction failed: {exc}", flush=True)
            continue
        if block is None:
            continue
        entries, _ = read_registry(args, env)
        if entry_matches(entries.get(region), rec["entry"]):
            rec.update({"registered": True, "tx_hash": tx_hash, "block": block,
                        "confirmation": "sequencer getTransaction inclusion and registry read-back"})
            return True
    return False


def submitted_hash(text):
    m = re.search(r"(?:Submitted|Transaction):\s*([0-9a-fA-F]{64})", text or "")
    return m.group(1).lower() if m else None


def reconcile(state, args, env, state_path):
    """Mark regions whose earlier submit timed out but did land (inclusion + read-back)."""
    entries, _ = read_registry(args, env)
    failures = state.setdefault("failures", {})
    for region, rec in state["regions"].items():
        if rec.get("registered") and rec.get("block"):
            continue
        tx_hash = rec.get("tx_hash") or rec.get("pending_tx") or submitted_hash(failures.get(region))
        if not tx_hash or not rec.get("entry"):
            continue
        block = included_block(args.rpc, tx_hash)
        if block is not None and entry_matches(entries.get(region), rec["entry"]):
            rec.update({"registered": True, "tx_hash": tx_hash, "block": block,
                        "confirmation": "sequencer getTransaction inclusion and registry read-back"})
            rec.pop("pending_tx", None)
            failures.pop(region, None)
            print(f"RECONCILED {region}: tx {tx_hash} in block {block}", flush=True)
    state_path.write_text(json.dumps(state, indent=2) + "\n")


def finish_confirmed(region, rec, state, state_path, failures):
    rec.pop("pending_tx", None)
    failures.pop(region, None)
    state_path.write_text(json.dumps(state, indent=2) + "\n")
    print(f"DONE {region} cid={rec['cid']} tx={rec['tx_hash']} (read-back confirmed)", flush=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--scratch", type=Path, default=Path("/workspace/codex-regions"))
    ap.add_argument("--osm", type=Path, required=True)
    ap.add_argument("--registry-cli", type=Path, required=True)
    ap.add_argument("--wallet-cli", type=Path, default=Path.home() / ".logos-osm/lez-v0.3.1/bin/wallet")
    ap.add_argument("--logosctl", type=Path, default=Path("/workspace/storage-tools/squashfs-root/AppRun"))
    ap.add_argument("--storage-config", type=Path, default=Path("/workspace/storage-tools/cfg"))
    ap.add_argument("--wallet-home", type=Path, default=Path.home() / ".logos-osm/testnet-v0.3/wallet")
    ap.add_argument("--password-file", type=Path, default=Path.home() / ".logos-osm/testnet-v0.3/password")
    ap.add_argument("--program-id", default="C31bCPXKa5uy1hzmegqMABWLbavoSJbY4Rhrc5yNXPrY")
    ap.add_argument("--payer", default="59VSwCJemEPWXBFGwQ8gzyZroNZ4AuQ81hJwpGUqMoEL")
    ap.add_argument("--rpc", default="https://testnet.lez.logos.co")
    ap.add_argument("--regions-file", type=Path, default=DEFAULT_REGIONS_FILE,
                    help="checked-in coverage data file (region list and pinned chain/tool versions)")
    ap.add_argument("--only", action="append")
    ap.add_argument("--gas-limit", type=int, default=1_000_000,
                    help="declared execution gas per registration; the sequencer reserves gas_limit*base_fee "
                         "from the payer before admission, so the wallet default (2,000,000) is refused as "
                         "'Incorrect fee' once the payer holds less than ~16M")
    args = ap.parse_args()
    scratch = args.scratch.resolve(); dl = scratch / "dl"
    dl.mkdir(parents=True, exist_ok=True)
    state_path = scratch / "state.json"
    state = json.loads(state_path.read_text()) if state_path.exists() else {"regions": {}}
    env = os.environ.copy()
    env.update({"OSM_PROGRAM_ID": args.program_id, "OSM_RPC_URL": args.rpc,
                "OSM_SIGNER": args.payer, "LEE_WALLET_HOME_DIR": str(args.wallet_home),
                "LOGOSCTL_BIN": str(args.logosctl), "LOGOSCTL_CONFIG_DIR": str(args.storage_config),
                "OSM_GAS_LIMIT": str(args.gas_limit)})
    cfg = scratch / "osm-coverage.toml"
    cfg.write_text("\n".join([
        f'program_id = "{args.program_id}"', f'rpc_url = "{args.rpc}"',
        f'wallet_home = "{args.wallet_home}"', f'signer_account_id = "{args.payer}"',
        f'cli_bin = "{args.registry_cli}"',
        f'logosctl_bin = "{args.logosctl}"', f'storage_config_dir = "{args.storage_config}"', "",
    ]))
    failures = state.setdefault("failures", {})
    defs = {d["region"]: d for d in json.loads((ROOT / "regions.json").read_text())}
    selection, _ = load_selection(args.regions_file)
    if len(set(selection)) != len(selection) or any(x not in defs for x in selection):
        raise RuntimeError("regions file must list unique predefined regions from regions.json")
    unknown = [x for x in (args.only or []) if x not in selection]
    if unknown:
        raise RuntimeError(f"--only regions not in {args.regions_file}: {', '.join(unknown)}")
    selected = args.only or selection
    reconcile(state, args, env, state_path)
    on_chain, _ = read_registry(args, env)
    for region in selected:
        rec = state["regions"].setdefault(region, {})
        if rec.get("roundtrip_verified") and rec.get("registered"):
            print(f"SKIP {region}: already roundtrip-verified and registered", flush=True); continue
        existing = on_chain.get(region)
        if existing and existing.get("hosted") and not rec.get("entry"):
            # Idempotent across machines: an entry already on chain is never re-downloaded or re-submitted.
            print(f"SKIP {region}: already registered on chain (cid={existing.get('cid')})", flush=True); continue
        print(f"START {region}", flush=True)
        d = defs[region]
        pbf = dl / (region.replace("/", "__") + ".osm.pbf")
        roundtrip = dl / (region.replace("/", "__") + ".roundtrip.pbf")
        try:
            if not rec.get("roundtrip_verified"):
                for attempt in range(2):
                    expected = published_md5(d["pbf_url"])
                    version, final_url = fetch_with_retry(d["pbf_url"], pbf)
                    size, actual, sha = digest(pbf)
                    if actual == expected:
                        break
                    pbf.unlink(missing_ok=True)
                    if attempt:
                        raise RuntimeError(f"MD5 mismatch after retry: expected {expected}, got {actual}")
                    print(f"RETRY {region}: sidecar mismatch; re-fetching snapshot", flush=True)
                rec.update({"size_bytes": size, "md5": expected, "sha256": sha,
                            "version": version, "source_url": d["pbf_url"], "download_url": final_url})
                state_path.write_text(json.dumps(state, indent=2) + "\n")
                prepared = run([str(args.osm), "--config", str(cfg), "prepare", region, str(pbf)], env, True)
                entry = json.loads(prepared.stdout)
                if entry["checksum"] != expected:
                    raise RuntimeError(f"SDK checksum changed during prepare: {entry['checksum']} != {expected}")
                entry["version"] = version
                rec["entry"] = entry
                state_path.write_text(json.dumps(state, indent=2) + "\n")
                # Fetch by CID directly; registry publication follows the roundtrip.
                run([str(args.osm), "--config", str(cfg), "fetch-cid", entry["cid"], str(roundtrip)], env)
                rt_size, rt_md5, _ = digest(roundtrip)
                if (rt_size, rt_md5) != (size, expected):
                    raise RuntimeError(f"Storage roundtrip mismatch: size {rt_size}/{size}, MD5 {rt_md5}/{expected}")
                rec["roundtrip_verified"] = True
                rec["cid"] = entry["cid"]
                state_path.write_text(json.dumps(state, indent=2) + "\n")
                pbf.unlink(missing_ok=True); roundtrip.unlink(missing_ok=True)
            if not rec.get("registered") and rec.get("pending_tx"):
                # A previous run submitted a tx whose wallet poll timed out; confirm it before resubmitting.
                if confirm_registration(region, rec["pending_tx"], rec, args, env, scratch, attempts=4):
                    finish_confirmed(region, rec, state, state_path, failures); continue
            if not rec.get("registered"):
                entry_json = json.dumps(rec["entry"], separators=(",", ":"))
                # The CLI waits for inclusion; SDK forwards its wallet tx hash to this log.
                with args.password_file.open("rb") as secret_input:
                    result = subprocess.run([str(args.osm), "--config", str(cfg), "register-entry", entry_json],
                                            check=False, text=True, env=env, stdin=secret_input,
                                            stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                if result.returncode:
                    detail = result.stderr or ""
                    secret = args.password_file.read_text().strip()
                    if secret:
                        detail = detail.replace(secret, "[REDACTED]")
                    tx_hash = submitted_hash(detail)
                    if tx_hash:
                        rec["pending_tx"] = tx_hash
                        state_path.write_text(json.dumps(state, indent=2) + "\n")
                        if confirm_registration(region, tx_hash, rec, args, env, scratch, attempts=20):
                            finish_confirmed(region, rec, state, state_path, failures); continue
                    raise RuntimeError(f"register-entry exited {result.returncode}: {detail[-1200:]}")
                sys.stdout.write(result.stderr)
                tx_line = re.search(r"Transaction:\s*(.+)", result.stderr)
                tx = re.search(r"\b[0-9a-fA-F]{64}\b", tx_line.group(1)) if tx_line else None
                if not tx:
                    raise RuntimeError("registration returned success but no waited transaction hash was surfaced")
                tx_hash = tx.group(0).lower()
                rec.update({"registered": True, "tx_hash": tx_hash, "block": included_block(args.rpc, tx_hash)})
                state_path.write_text(json.dumps(state, indent=2) + "\n")
            failures.pop(region, None)
            print(f"DONE {region} cid={rec['cid']} tx={rec['tx_hash']}", flush=True)
        except Exception as e:
            failures[region] = str(e)
            state_path.write_text(json.dumps(state, indent=2) + "\n")
            if rec.get("roundtrip_verified"):
                pbf.unlink(missing_ok=True); roundtrip.unlink(missing_ok=True)
            print(f"FAILED {region}: {e}", file=sys.stderr, flush=True)
    # Raw public registry read-back is saved even when one or more regions failed.
    chain_entries, raw_readback = read_registry(args, env)
    readback_path = scratch / "registry-readback.json"
    readback_path.write_text(raw_readback)
    mismatches = []
    for region, rec in state["regions"].items():
        if not rec.get("registered"):
            continue
        actual = chain_entries.get(region)
        expected = rec.get("entry", {})
        if not actual or any(actual.get(k) != expected.get(k) for k in ("cid", "checksum", "hosted")):
            mismatches.append(region)
    state["readback_verified"] = not mismatches
    state["readback_mismatches"] = mismatches
    state_path.write_text(json.dumps(state, indent=2) + "\n")
    print(f"Read-back saved to {scratch / 'registry-readback.json'}", flush=True)
    if mismatches:
        print(f"READ-BACK MISMATCH: {', '.join(mismatches)}", file=sys.stderr, flush=True)
    return 1 if failures or mismatches else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as exc:
        print(f"FATAL: {exc}", file=sys.stderr)
        sys.exit(2)
