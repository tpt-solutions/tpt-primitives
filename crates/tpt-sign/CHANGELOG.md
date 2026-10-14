# Changelog

All notable changes to `tpt-sign` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `Signer`, `Signed<T>`, `PublicKey` and `Signature`: Ed25519 over a
  domain-separated identity. Decoding a `Signed<T>` verifies the signature.
- ADR 0004 (signing scheme), `testvectors/signed-v1.json`, and the
  `signed_evidence` example.

[Unreleased]: https://tpt.example/tpt-primitives/compare/v0.1.0...HEAD
