# touchstone

[![ci](https://github.com/savageAZfck/touchstone/actions/workflows/ci.yml/badge.svg)](https://github.com/savageAZfck/touchstone/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Spec](https://img.shields.io/badge/spec-v0.1-purple.svg)](SPEC.md)

The verify-don't-trust toolkit for personal AGI claims.

A touchstone is the stone used to test gold — the original instrument for
checking a claim instead of trusting it. This project is that instrument for
personal AGI: a normative conformance spec, a language-agnostic battery any
agent can run against, device-signed attestations, and a browser explorer that
verifies the receipts.

It exists so that "I built a personal AGI" is a checkable claim, not a vibe.
The first conformant subject is [Bad Apple](https://github.com/savageAZfck/Bad_Apple)
— a sovereign personal AGI organism for Apple Silicon. The spec is open so
that anyone else's organism can run the same gauntlet.

## What's inside

- `SPEC.md` — the normative standard: required organs, verdict rules,
  attestation format, adapter protocol
- `crates/touchstone-core` — types and verdict rules (no_std-friendly)
- `crates/touchstone-harness` — the battery: exec protocol + adapter trait +
  scoring
- `crates/touchstone-identity` — device-bound signing (Secure Enclave on
  macOS, Ed25519 fallback)
- `crates/touchstone-cli` — the `touchstone` binary
- `crates/touchstone-wasm` + `web/` — browser verification explorer
- `adapters/badapple` — reference adapter for Bad Apple
- `testvectors/` — golden attestations and tamper cases
- `verifiers/` — independent JavaScript verifier
- `attestations/` — the registry. The scoreboard.

## Quickstart

```bash
cargo build --release

# print the checklist
./target/release/touchstone spec

# run the battery against a subject (ships the badapple adapter)
./target/release/touchstone run --adapter badapple

# sign + verify an attestation
./target/release/touchstone attest attestation.json --key <64-hex-chars>
./target/release/touchstone verify attestation.signed.json

# pretty-print the organ scoreboard
./target/release/touchstone explore attestation.signed.json
```

`verify` re-derives the verdict from the check list and validates the
document before checking the signature — a valid signature over a false
verdict is still a failed attestation.

## The claim this backs

Bad Apple's published attestation (45 PASS / 1 planted-FAIL control /
1 OPTIONAL) is reproducible here: `touchstone run --adapter badapple` on a
Mac running her daemon re-executes the organ battery and emits a
device-signed conformance document. Read `SPEC.md`, then check the receipts.

## Adapter protocol

Any subject can be tested by writing one executable that prints NDJSON check
results — any language, no Rust required:

```
{"check":"memory.store_recall","organ":"memory","status":"pass","evidence":{...}}
{"check":"planted_negative","organ":"audit","status":"fail","control":true}
```

The harness spawns it once, bounds it to a timeout, ignores non-JSON chatter,
scores the run, and emits the attestation. Spec §5 has the full protocol and
error semantics.

## Feature flags and targets

- `touchstone-identity` default: Ed25519 signing + verification, all platforms.
- `--features secure-enclave`: adds the Secure Enclave signer (macOS).
  Enclave signatures verify everywhere — including wasm32 — via `p256`.
- `touchstone-wasm` and `adapters/badapple` are not published to crates.io.

## Publishing status

Not yet on crates.io — the workspace is being hardened first. Publish order
will be `touchstone-core` → `touchstone-harness` → `touchstone-identity` →
`touchstone-cli`.

License: MIT OR Apache-2.0.
