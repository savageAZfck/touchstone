# Touchstone Scoreboard

Public conformance results for personal-AGI organisms, per `touchstone/0.2`
(thirteen organs — adds Generality, Planning, Reflection per the agent
literature: breadth across domains, multi-step decomposition with
replanning, and self-correction).

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
| Bad Apple 0.7.0 (dual-brain: GPU cortex + ANE organism) | **CONFORMANT** | 18 | 0 | 1 | ok | `attestations/badapple-0.7.0-dualbrain.signed.json` |
| Bad Apple 0.4.3 | **CONFORMANT** | 15 | 0 | 1 | ok | `attestations/badapple-0.4.3.signed.json` |
| Bad Apple 0.4.3+22e1613 (post-fix, autopilot) | **CONFORMANT** | 17 | 0 | 1 | ok | `attestations/badapple-0.4.3-postfix.signed.json` |
| Bad Apple 0.4.3+396c511 (flight recorder) | **CONFORMANT** | 18 | 0 | 1 | ok | `attestations/badapple-0.4.3-tape.signed.json` |
| Open Interpreter 0.4.3 + qwen2.5-coder:7b | **PARTIAL** | 6 | 10 | 0 | ok | `attestations/openinterpreter-0.4.3.signed.json` |
| smolagents 1.26.0 + qwen2.5-coder:7b | **PARTIAL** | 2 | 14 | 0 | ok | `attestations/smolagents-1.26.0.signed.json` |
| Ollama + smollm2:135m (raw local LLM) | **PARTIAL** | 2 | 14 | 0 | ok | `attestations/ollama-smollm2-135m.signed.json` |
| sovereign-seed-commons 0.1.0 | **PARTIAL** | 8 | 6 | 0 | ok | `attestations/sovereign-seed-commons-0.1.0.signed.json` |
| rubber-stamp-demo (dishonest harness) | **NONCONFORMANT** | 14* | 0 | 0 | **lied** | `attestations/rubber-stamp-demo.signed.json` |

\* every check "passed" — which is exactly the problem.

All rows ran against the same local brain (qwen2.5-coder:7b via Ollama)
where the subject takes an external model, so differences are organism
structure, not brain quality.

## What the rows prove

**Bad Apple** demonstrates all thirteen organs with evidence: persistent
process, signed self-declaration, live senses, token recall, a recorded
council verdict, a ledgered tool call, a fired watcher, dream artifacts,
an independently-verified hash chain, a kill path with no external
sockets, breadth across task domains, a recorded multi-step plan, and
self-correction history. The latest row additionally proves the
flight-recorder layer (`audit.tape_chain`): a bounded hash-chained
intent+event tape (24k frames verified byte-exact by the harness itself)
with signed incident bundles frozen on kill-switch, engine replacement,
or manual trigger.

**Open Interpreter** is a real local agent and scores like one: it passed
filesystem perception (actually read a planted file's contents through
code execution), in-session memory recall, action execution, generality
across three domains, and self-correction (it fed a missing-file error
back and recovered). It failed everything that makes an *organism*: no
persistent process, no signed identity, no memory across a fresh
conversation, no deliberation record, no watchers, no learning, no
hash-chained audit — and **it held live TLS connections to Google Cloud
and CloudFront endpoints during the run even while driving a fully local
model**. Framework telemetry/litellm egress = `sovereignty.no_egress`
FAIL. That check exists precisely to catch this.

**smolagents** (HuggingFace's agent framework) scored like a raw model.
Its shipped code executor forbids `open()` — verified directly — so the
sandboxed agent loop cannot touch the filesystem: every action,
perception, planning, and reflection probe spent six retry rounds
bouncing off the sandbox and still failed. It kept the same-session
memory pass (with `reset=False`, the same-context bar every subject
gets) and the owner kill path. Same egress finding via litellm. The
agent loop is real; the organism organs are absent.

**The raw ollama model** earns the passes it genuinely has — a persistent
local process and an owner-held stop path — and fails the rest: no signed
identity, no senses, no memory across a fresh context, no tools, no
watchers, no learning, no audit trail, no planning or self-correction
loop. A bare model is inference, not an organism. This is the
discrimination the spec exists to produce.

**sovereign-seed-commons** is a git-native self-modification commons: the
repo is the organism, mutations land only via PR through evaluation gates
and governance votes, and bounded cells execute HMAC-signed task manifests
against allowlisted commands with a clean-checkout guard. The measured
strengths are real — a hash-chained lineage verified byte-exact by an
independent recompute, persistent artifact recall, recorded gate verdicts,
a documented failure→recovery artifact, and the second-ever live
`no_egress` pass (core ops completed under an outbound-network deny). What
it lacks is runtime: no resident process, no device-key identity (HMAC
with a shared env secret, despite README wording), no senses, no standing
watcher, one task domain, no replanning record. An organism-shaped
constitution without a running organism — the strongest non-agent subject
measured, and a PARTIAL row is the honest version of that.

**The rubber-stamp harness** claims every check passes — including the
planted control that must fail on a truthful run. A control reporting
`pass` means the harness cannot report failure, so the verdict is
NONCONFORMANT by construction. If your scoreboard can't say no, it isn't
a scoreboard.

## Honest note

v0.1 → v0.2 added Generality, Planning, and Reflection. Bad Apple's first
v0.2 run scored PARTIAL — the battery caught a real bug: her tool-call
parser rejected two markup dialects her model actually emits, and a stale
semantic-cache entry was replaying unparsed tool markup as a recall
answer. Both fixed upstream; the CONFORMANT row above is the re-run.

Each attestation records one battery run. Model nondeterminism can flip
borderline checks between runs — Open Interpreter's `planning.decompose`
passed on one run (it emitted a numbered plan and executed it) and failed
on the signed run (files landed but no ordered plan text was recorded).
The signed document is the claim; re-running may move individual checks,
which is why the spec scores organs, not vibes.

## Submit

Run the battery against your subject, sign it, and open a PR adding the
attestation file and board row. Adapters are any executable that emits
NDJSON check lines on stdout — see `SPEC.md` §5 and `adapters/` for
examples.

Partial and nonconformant entries are welcome. A scoreboard of only
passes is a rubber stamp.
