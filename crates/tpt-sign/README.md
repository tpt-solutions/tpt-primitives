# tpt-sign

Ed25519 signatures over TPT canonical object identities.

`tpt-primitives` says *what* an object is. `tpt-sign` lets a key holder say
*"I vouch for exactly that"*. A `Signed<T>` carries the object, the signer's
`PublicKey`, and an Ed25519 `Signature` over a domain-separated message:

```text
message = "tpt-sign.v1" || 0x00 || identity_domain || 0x00 || identity (32 bytes)
```

The signature covers the object's identity, not a copy of its bytes. Domain
separation stops a signature on one class of object from being replayed as
another.

## Usage

```rust
use tpt_sign::Signer;

let signer = Signer::from_seed(seed); // 32-byte seed
let signed = signer.sign(evidence);   // Signed<Evidence>
assert!(signed.verify().is_ok());
```

Decoding a `Signed<T>` from canonical bytes verifies the signature, so a forged
or corrupted envelope never becomes a value.

```sh
cargo run -p tpt-sign --example signed_evidence
```

## Not in scope

This crate does not decide who a key *is*, or whether it may sign a given kind
of object. Mapping keys to principals, key distribution and revocation belong
to the wider ecosystem. Trust boundaries are described in
`docs/trust-boundaries.md` in the repository.

## License

Dual-licensed under MIT or Apache-2.0, at your option.
