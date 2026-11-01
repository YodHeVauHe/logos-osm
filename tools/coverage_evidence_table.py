#!/usr/bin/env python3
"""Render the evidence table in docs/coverage-testnet-v0.3.md from a registry read-back.

Usage: coverage_evidence_table.py STATE_JSON READBACK_JSON DOC_MD
Every row is checked: the on-chain CID and checksum must equal the locally
verified CID and Geofabrik MD5, and the entry must be marked hosted.
"""
import json
import sys
from pathlib import Path

BEGIN, END = "<!-- regions:begin -->", "<!-- regions:end -->"


def main(state_path, readback_path, doc_path):
    state = json.loads(Path(state_path).read_text())["regions"]
    chain = json.loads(Path(readback_path).read_text())
    if isinstance(chain, list):
        chain = {e["region"]: e for e in chain}
    rows, countries = [], set()
    for region, entry in sorted(chain.items()):
        rec = state.get(region, {})
        if entry["cid"] != rec.get("cid") or entry["checksum"] != rec.get("md5") or not entry.get("hosted"):
            raise SystemExit(f"{region}: on-chain entry does not match the verified local record")
        country = region.split("/")[0]
        countries.add(country)
        tx = rec["tx_hash"]
        rows.append(
            f"| `{region}` | {country.replace('-', ' ').title()} | {rec['size_bytes'] / 1e6:.1f} MB "
            f"| `{rec['md5']}` | `{entry['cid']}` | {rec.get('version', '')} | {rec.get('block', '')} "
            f"| [`{tx[:12]}…`](https://explorer.testnet.lez.logos.co/transaction/{tx}) |"
        )
    table = "\n".join([
        f"**{len(rows)} regions across {len(countries)} countries** (from the registry read-back).",
        "",
        "| Region | Country | Size | Geofabrik MD5 (= on-chain checksum) | Storage CID | Snapshot | Block | Tx |",
        "|---|---|---|---|---|---|---|---|",
        *rows,
    ])
    doc = Path(doc_path)
    text = doc.read_text()
    head, rest = text.split(BEGIN)
    _, tail = rest.split(END)
    doc.write_text(f"{head}{BEGIN}\n{table}\n{END}{tail}")
    print(f"{len(rows)} regions, {len(countries)} countries")


if __name__ == "__main__":
    main(*sys.argv[1:4])
