#!/usr/bin/env python3
"""Check that LP-0018 status claims match tracked evidence and known gaps."""

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
readme = (ROOT / "README.md").read_text()
furps = (ROOT / "docs/furps.md").read_text()
cycles = (ROOT / "docs/cycles.md").read_text()
catalog = (ROOT / "docs/catalog.md").read_text()
idl_path = ROOT / "idl/osm-registry.idl.json"

assert idl_path.is_file(), "tracked registry IDL is missing"
json.loads(idl_path.read_text())

for label, content in (("README", readme), ("FURPS", furps)):
    assert not re.search(r"IDL.{0,50}(?:absent|missing|warning-only)", content, re.I | re.S), (
        f"{label} contains a stale claim that IDL is absent or warning-only"
    )

assert re.search(r"register_region.{0,120}2,145", cycles, re.S)
assert re.search(r"register_batch.{0,120}22,785", cycles, re.S)
assert "localnet" in cycles.lower()
assert not re.search(r"placeholder|filled by", cycles, re.I)

for label, content in (("README", readme), ("FURPS", furps)):
    assert "pending" in content.lower(), f"{label} must make pending status explicit"
    assert re.search(r"(?:testnet 0\.4.{0,120}(?:pending|not recorded)|(?:pending|not recorded).{0,120}testnet 0\.4)", content, re.I | re.S), (
        f"{label} must keep testnet deployment pending"
    )
    assert re.search(r"(?:catalog.{0,120}(?:pending|not published|no catalog)|(?:pending|not published|no catalog).{0,120}catalog)", content, re.I | re.S), (
        f"{label} must keep catalog publication pending"
    )
    assert re.search(r"(?:adoption.{0,160}(?:pending|no external|no public)|(?:pending|no external|no public).{0,160}adoption)", content, re.I | re.S), (
        f"{label} must keep adoption evidence pending"
    )

assert re.search(r"No catalog fork or.*URL has been published", catalog, re.I | re.S)
assert "uploadFile" in furps and "uploadUrl" in furps
print("Submission documentation status is consistent with tracked IDL, localnet measurements, and pending owner actions.")
