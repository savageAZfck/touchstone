# Changelog

All notable changes to touchstone. Spec versions are independent of crate
versions — see SPEC.md §1.2.

## [Unreleased]

### Added
- `SPEC_VERSION` constant in touchstone-core, decoupled from crate semver.
- `Attestation::validate()` — structural validation (spec tag, unique check
  ids, RFC3339 timestamp, verdict recomputation, signature field shape).
- `touchstone explore` — pretty-printed organ scoreboard for an attestation.
- `HarnessError::Timeout` — ExecAdapter now enforces its declared timeout;
  an adapter that outlives it is killed instead of hanging the run.
- Integration test: shell mock adapter driving run → attest → verify →
  tamper-rejection and verdict-lie rejection.
- SPEC.md: deterministic verdict pseudocode, canonicalization algorithm,
  NDJSON protocol error semantics, termination/timeout rules, versioning.

### Changed
- `verify` now recomputes the verdict from checks and fails on mismatch —
  a valid signature over a false verdict no longer verifies.
- `attest` refuses empty check lists and warns when re-signing.
- Attestation spec field reads `touchstone/0.1` (spec version), not the
  crate version.
- `touchstone-identity`: `OsRng`/`generate` are gated off wasm32 —
  verification compiles for browsers, keygen stays native.

## [0.1.0] — initial public commit

First working battery: 10-organ spec, NDJSON exec protocol, Ed25519 +
Secure Enclave signing, independent JS verifier, wasm explorer skeleton,
badapple reference adapter. First live attestation: Bad Apple 0.4.3,
Conformant.
