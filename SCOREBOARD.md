# Touchstone Scoreboard

Public conformance results for personal-AGI organisms, per `touchstone/0.1`.
Every entry is a signed attestation you can verify yourself:

```bash
cargo install touchstone-cli
touchstone verify attestations/<file>.signed.json
```

Signer identity for entries on this board: Ed25519 pubkey
`c093c0301090eef7…` (full key inside each attestation's `signature` field).

## Board

| Subject | Verdict | Pass | Fail | Optional | Control | Attestation |
|---|---|---|---|---|---|---|
| Bad Apple 0.4.3 | **CONFORMANT** | 12 | 0 | 1 | ok | `attestations/badapple-0.4.3.signed.json` |
| Ollama + smollm2:135m (raw local LLM) | **PARTIAL** | 3 | 10 | 0 | ok | `attestations/ollama-smollm2-135m.signed.json` |
| rubber-stamp-demo (dishonest harness) | **NONCONFORMANT** | 11* | 0 | 0 | **lied** | `attestations/rubber-stamp-demo.signed.json` |

\* every check "passed" — which is exactly the problem.

## What the rows prove

**Bad Apple** demonstrates all ten organs with evidence: persistent
process, signed self-declaration, two live senses, token recall across the
ledger, a recorded council verdict, a ledgered tool call, a fired watcher,
dream artifacts, an independently-verified hash chain, and a kill path with
no external sockets.

**The raw ollama model** earns the three passes it genuinely has — a
persistent local process, no required egress, an owner-held stop path —
and fails the rest: no signed identity, no senses, no memory across a
fresh context, no tools, no watchers, no learning, no audit trail. A bare
model is inference, not an organism. This is the discrimination the spec
exists to produce.

**The rubber-stamp harness** claims every check passes — including the
planted control that must fail on a truthful run. A control reporting
`pass` means the harness cannot report failure, so the verdict is
NONCONFORMANT by construction. If your scoreboard can't say no, it isn't
a scoreboard.

## Submit

Run the battery against your subject, sign it, and open a PR adding the
attestation file and board row. Adapters are any executable that emits
NDJSON check lines on stdout — see `SPEC.md` §5 and `adapters/` for
examples.

Partial and nonconformant entries are welcome. A scoreboard of only
passes is a rubber stamp.
