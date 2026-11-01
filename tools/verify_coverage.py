#!/usr/bin/env python3
"""Keyless verification of the LP-0018 coverage set.

1. Reads the registry account straight from the LEZ sequencer JSON-RPC
   (`getAccount` on the registry PDA) and borsh-decodes it. No wallet,
   password or signing key is involved.
2. For every region in the checked-in data file, fetches the on-chain CID from
   Logos Storage through `logosctl` (`downloadToUrl`, local=false) and compares
   the MD5 of the bytes with the on-chain checksum (Geofabrik's MD5).
3. Prints a pass/fail table; exits 1 if any row fails.

Only the Python standard library and a running logosctl 0.3.2 / storage_module
3.0.2 session are needed. `--registry-only` skips step 2.
"""
import argparse
import hashlib
import json
import os
import queue
import subprocess
import sys
import threading
import time
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_REGIONS_FILE = ROOT / "coverage" / "regions-testnet-v0.3.json"


class Reader:
    def __init__(self, data):
        self.b, self.i = data, 0

    def take(self, n):
        if self.i + n > len(self.b):
            raise ValueError("truncated registry account data")
        out = self.b[self.i:self.i + n]
        self.i += n
        return out

    def u8(self):
        return self.take(1)[0]

    def u32(self):
        return int.from_bytes(self.take(4), "little")

    def i64(self):
        return int.from_bytes(self.take(8), "little", signed=True)

    def string(self):
        return self.take(self.u32()).decode("utf-8")


def decode_registry(data):
    """Borsh layout of osm_registry_core::Registry { entries: BTreeMap<String, RegionEntry> }."""
    r = Reader(bytes(data))
    entries = {}
    for _ in range(r.u32()):
        key = r.string()
        e = {"region": r.string()}
        e["parent"] = r.string() if r.u8() else None
        e["level"] = r.u8()
        e["cid"] = r.string()
        e["source_url"] = r.string()
        e["checksum"] = r.string()
        e["version"] = r.string()
        e["hosted"] = bool(r.u8())
        e["timestamp"] = r.i64()
        e["registered_by"] = r.take(32).hex()
        e["record_version"] = r.u8()
        entries[key] = e
    if r.i != len(r.b):
        raise ValueError(f"{len(r.b) - r.i} trailing bytes after registry")
    return entries


def read_registry(rpc, pda, program_id):
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": "getAccount", "params": [pda]}).encode()
    req = urllib.request.Request(rpc, data=body, headers={"content-type": "application/json"})
    with urllib.request.urlopen(req, timeout=60) as resp:
        reply = json.load(resp)
    if "error" in reply:
        raise RuntimeError(f"getAccount: {reply['error']}")
    shard = reply["result"]["data"]["shards"].get(program_id)
    if not shard:
        raise RuntimeError(f"registry account {pda} has no data for program {program_id}")
    return decode_registry(shard)


def md5_of(path):
    h, size = hashlib.md5(), 0
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
            size += len(chunk)
    return h.hexdigest(), size


class Storage:
    def __init__(self, logosctl, config_dir):
        self.base = [str(logosctl)] + (["--config-dir", str(config_dir)] if config_dir else [])

    def call(self, *args, timeout=120):
        out = subprocess.run(self.base + ["call", "storage_module", *args], text=True,
                             capture_output=True, timeout=timeout)
        try:
            return json.loads(out.stdout.strip() or "{}")
        except json.JSONDecodeError:
            return {"status": "error", "message": (out.stdout + out.stderr)[-500:]}

    def fetch(self, cid, dest, timeout):
        """downloadToUrl(cid, dest, local=false, 64KiB, mix=false, advertise=false); wait for its event."""
        watcher = subprocess.Popen(self.base + ["watch", "storage_module"], text=True,
                                   stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        lines = queue.Queue()
        threading.Thread(target=lambda: [lines.put(l) for l in watcher.stdout], daemon=True).start()
        try:
            time.sleep(0.5)
            reply = self.call("downloadToUrl", cid, str(dest), "false", "65536", "false", "false")
            result = reply.get("result") if isinstance(reply.get("result"), dict) else None
            if result and result.get("success") is False and "time" not in str(result.get("error")).lower():
                return False, f"downloadToUrl rejected: {result.get('error')}"
            # The daemon's 20 s call timeout can fire while the manifest is still being
            # located; the download keeps running under a session named after the CID.
            session = (result or {}).get("value") or cid
            deadline = time.monotonic() + timeout
            while time.monotonic() < deadline:
                try:
                    line = lines.get(timeout=max(0.1, min(5, deadline - time.monotonic())))
                except queue.Empty:
                    continue
                try:
                    event = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if event.get("event") != "storageDownloadDone":
                    continue
                payload = json.loads(event.get("data", {}).get("arg0") or "{}")
                if payload.get("sessionId") != session:
                    continue
                return bool(payload.get("success")), payload.get("error") or ""
            return False, f"no storageDownloadDone within {timeout}s"
        finally:
            watcher.kill()
            watcher.wait()


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--regions-file", type=Path, default=DEFAULT_REGIONS_FILE)
    ap.add_argument("--rpc")
    ap.add_argument("--logosctl", default=os.environ.get("LOGOSCTL_BIN", "logosctl"))
    ap.add_argument("--storage-config", default=os.environ.get("LOGOSCTL_CONFIG_DIR"),
                    help="logosctl --config-dir of the Storage session to fetch through")
    ap.add_argument("--work-dir", type=Path, default=Path("/tmp/osm-verify"))
    ap.add_argument("--only", action="append", help="verify only these regions (repeatable)")
    ap.add_argument("--timeout", type=int, default=900, help="seconds per CID")
    ap.add_argument("--registry-only", action="store_true", help="check the on-chain entries, skip Storage")
    ap.add_argument("--evict", action="store_true",
                    help="remove each CID from this Storage node after checking (for non-hosting verifier nodes)")
    ap.add_argument("--json-out", type=Path)
    ap.add_argument("--control-cid", action="append", default=[], metavar="LABEL=CID",
                    help="also fetch a CID that is not in the registry (e.g. a public reference file) "
                         "to tell 'network unreachable' apart from 'our CID unavailable'; reported only")
    args = ap.parse_args()

    data = json.loads(args.regions_file.read_text())
    chain = data["chain"]
    rpc = args.rpc or chain["rpc"]
    regions = [r["region"] for r in data["regions"]]
    country_of = {r["region"]: r["country"] for r in data["regions"]}
    if args.only:
        regions = [r for r in regions if r in args.only]
    entries = read_registry(rpc, chain["registry_pda"], chain["program_id"])
    print(f"registry {chain['registry_pda']} via {rpc}: {len(entries)} entries on chain", flush=True)
    storage = None if args.registry_only else Storage(args.logosctl, args.storage_config)
    args.work_dir.mkdir(parents=True, exist_ok=True)
    if storage:
        dbg = storage.call("debug").get("result", {}).get("value") or {}
        if isinstance(dbg, str):
            try:
                dbg = json.loads(dbg)
            except json.JSONDecodeError:
                dbg = {}
        print(f"fetching through Storage node {dbg.get('id', '?')}", flush=True)
    controls = []
    for spec in args.control_cid if storage else []:
        label, _, cid = spec.partition("=")
        dest = args.work_dir / ("control-" + label)
        dest.unlink(missing_ok=True)
        t0 = time.monotonic()
        ok, err = storage.fetch(cid, dest, args.timeout)
        size = dest.stat().st_size if dest.exists() else 0
        controls.append({"label": label, "cid": cid, "ok": ok and size > 0, "bytes": size,
                         "seconds": round(time.monotonic() - t0, 1), "detail": err})
        dest.unlink(missing_ok=True)
        if args.evict:
            storage.call("remove", cid)
        print(f"control {label}: {'OK' if ok and size else 'FAIL'} {size} bytes {err}".rstrip(),
              file=sys.stderr, flush=True)
    rows = []
    for region in regions:
        e = entries.get(region)
        row = {"region": region, "cid": e and e["cid"], "checksum": e and e["checksum"],
               "md5": None, "bytes": None, "seconds": None, "result": "FAIL", "detail": ""}
        if not e:
            row["detail"] = "not on chain"
        elif not e["hosted"]:
            row["detail"] = "on chain but hosted=false"
        elif storage is None:
            row["result"], row["detail"] = "PASS", "registry only"
        else:
            dest = args.work_dir / (region.replace("/", "__") + ".osm.pbf")
            dest.unlink(missing_ok=True)
            t0 = time.monotonic()
            ok, err = storage.fetch(e["cid"], dest, args.timeout)
            row["seconds"] = round(time.monotonic() - t0, 1)
            if ok and dest.exists():
                row["md5"], row["bytes"] = md5_of(dest)
                if row["md5"] == e["checksum"]:
                    row["result"] = "PASS"
                else:
                    row["detail"] = "MD5 differs from on-chain checksum"
            else:
                row["detail"] = err or "download failed"
            dest.unlink(missing_ok=True)
            if args.evict:
                storage.call("remove", e["cid"])
        rows.append(row)
        print(f"{row['result']} {region} {row['detail']}".rstrip(), file=sys.stderr, flush=True)

    passed = sum(r["result"] == "PASS" for r in rows)
    countries = {country_of[r["region"]] for r in rows if r["result"] == "PASS"}
    print()
    print("| Region | Storage CID | On-chain checksum | Fetched MD5 | Bytes | Seconds | Result |")
    print("|---|---|---|---|---|---|---|")
    for r in rows:
        md5 = r["md5"] or "-"
        print(f"| `{r['region']}` | `{r['cid'] or '-'}` | `{r['checksum'] or '-'}` | `{md5}` "
              f"| {r['bytes'] or '-'} | {r['seconds'] or '-'} | {r['result']}{' (' + r['detail'] + ')' if r['detail'] else ''} |")
    print()
    for c in controls:
        print(f"control `{c['label']}` `{c['cid']}`: {'fetched' if c['ok'] else 'FAILED'}, "
              f"{c['bytes']} bytes in {c['seconds']} s {c['detail']}".rstrip())
    if controls:
        print()
    print(f"{passed}/{len(rows)} PASS across {len(countries)} countries")
    if args.json_out:
        args.json_out.write_text(json.dumps({"rpc": rpc, "registry_pda": chain["registry_pda"],
                                             "rows": rows, "controls": controls, "passed": passed}, indent=2) + "\n")
    return 0 if passed == len(rows) else 1


if __name__ == "__main__":
    sys.exit(main())
