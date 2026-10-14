"""Independent check of testvectors/v1.json (no Rust involved).

Needs `pip install cbor2`. For each vector it verifies that
  1. the canonical bytes are exactly what a standard RFC 8949 section 4.2
     deterministic encoder produces for the decoded value, and
  2. id == SHA-256(domain || 0x00 || canonical_bytes).
"""
import hashlib
import json
import pathlib
import sys

import cbor2

path = pathlib.Path(__file__).with_name("v1.json")
failed = 0
for v in json.loads(path.read_text())["vectors"]:
    raw = bytes.fromhex(v["canonical_hex"])
    magic, version, _payload = cbor2.loads(raw)
    canonical = cbor2.dumps(cbor2.loads(raw), canonical=True) == raw
    ident = hashlib.sha256(v["domain"].encode() + b"\x00" + raw).hexdigest() == v["id"]
    ok = canonical and ident and magic == "tpt-canonical" and version == v["schema_version"]
    failed += not ok
    print(("ok  " if ok else "FAIL"), v["name"])
sys.exit(1 if failed else 0)
