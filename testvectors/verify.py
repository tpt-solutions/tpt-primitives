"""Independent check of the golden vectors (no Rust involved).

Needs `pip install cbor2 cryptography`.

v1.json, for each vector:
  1. the canonical bytes are exactly what a standard RFC 8949 section 4.2
     deterministic encoder produces for the decoded value, and
  2. id == SHA-256(domain || 0x00 || canonical_bytes).

signed-v1.json:
  3. the object identity matches (as above),
  4. the signature verifies over
     "tpt-sign.v1" || 0x00 || identity_domain || 0x00 || identity,
  5. re-signing with the seed reproduces the signature (Ed25519 is
     deterministic), and the signed envelope is canonical CBOR.
"""
import hashlib
import json
import pathlib
import sys

import cbor2
from cryptography.hazmat.primitives.asymmetric.ed25519 import (
    Ed25519PrivateKey,
    Ed25519PublicKey,
)

here = pathlib.Path(__file__).parent
failed = 0


def report(name, ok):
    global failed
    failed += not ok
    print(("ok  " if ok else "FAIL"), name)


for v in json.loads((here / "v1.json").read_text())["vectors"]:
    raw = bytes.fromhex(v["canonical_hex"])
    magic, version, _payload = cbor2.loads(raw)
    canonical = cbor2.dumps(cbor2.loads(raw), canonical=True) == raw
    ident = hashlib.sha256(v["domain"].encode() + b"\x00" + raw).hexdigest() == v["id"]
    report(
        v["name"],
        canonical and ident and magic == "tpt-canonical" and version == v["schema_version"],
    )

s = json.loads((here / "signed-v1.json").read_text())
obj = bytes.fromhex(s["object_canonical_hex"])
identity = hashlib.sha256(s["object_domain"].encode() + b"\x00" + obj).digest()
message = (
    s["signature_domain"].encode()
    + b"\x00"
    + s["object_domain"].encode()
    + b"\x00"
    + identity
)
try:
    Ed25519PublicKey.from_public_bytes(bytes.fromhex(s["signer"])).verify(
        bytes.fromhex(s["signature"]), message
    )
    verifies = True
except Exception:
    verifies = False
key = Ed25519PrivateKey.from_private_bytes(bytes.fromhex(s["seed_hex"]))
envelope = bytes.fromhex(s["signed_canonical_hex"])
report(
    "signed lease",
    identity.hex() == s["object_id"]
    and verifies
    and key.sign(message).hex() == s["signature"]
    and key.public_key().public_bytes_raw().hex() == s["signer"]
    and cbor2.dumps(cbor2.loads(envelope), canonical=True) == envelope,
)

sys.exit(1 if failed else 0)
