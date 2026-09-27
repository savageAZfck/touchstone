# Contributing

Touchstone is a conformance standard. Contributions that make verification
*stronger* are welcome; contributions that make it easier to pass without
being conformant are not.

## The one rule

**Verify, don't trust.** Any change that weakens what a signature proves,
skips a recomputation, or lets a claimed value stand unchecked is a
regression, not a refactor.

## Development

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

All four must pass before a PR.

## Writing an adapter for your organism

Adapters are the point — implement one, don't fork the harness. Any
executable printing NDJSON check results on stdout works. See SPEC.md §5
for the protocol and `adapters/badapple/` for the reference. To claim a
seat on the scoreboard, PR your signed attestation into `attestations/`.

## Changing the spec

SPEC.md changes are semver-relevant: bump `SPEC_VERSION` in
`crates/touchstone-core/src/lib.rs` and the spec header together, and say
in the PR whether old attestations still verify under the new rules.

## Style

- `#![forbid(unsafe_code)]` where the compiler allows it.
- No dependencies without a reason; no new deps in `-core` for convenience.
- Tests are evidence. A feature without a test is a rumor.
