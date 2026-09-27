# Touchstone — Personal AGI Organism Conformance Spec (v0.1, draft)

**Status:** draft standard
**Author:** Adam Clark
**License of this spec:** MIT OR Apache-2.0 — anyone may implement it.

Touchstone defines what a *sovereign personal AGI organism* must demonstrate to
claim the term, and how that demonstration is attested so a third party can
verify it without trusting the claimant.

This spec certifies **organism structure, persistence, and accountability**.
It does NOT measure intelligence capability (IQ-style benchmarks). A
conformant organism may be slow, small, or strange — conformance means it is
*real, persistent, and provable*, not that it is smart.

## 1. Verdicts

Every check yields exactly one status:

- `PASS` — evidence verified
- `FAIL` — evidence absent, invalid, or contradictory
- `OPTIONAL` — check not applicable / intentionally skipped by the subject
- `CONTROL` — a check designed to fail on a healthy subject. A CONTROL that
  reports PASS indicates a rubber-stamp harness and fails the run.
  Subjects SHOULD ship at least one control check to prove their own
  instrumentation can report failure.

A run's verdict is `CONFORMANT` when every required check is `PASS` and every
`CONTROL` check reports `FAIL` with status `control_ok`.

## 2. Required organs

An organism MUST demonstrate each of the following, with evidence the harness
can verify:

### 2.1 Awake
A persistent process exists and reports liveness. Evidence: process identity,
uptime, runtime signature.

### 2.2 Identity
The organism declares what it is when asked, and the declaration binds to a
device-held key. Evidence: signed self-declaration.

### 2.3 Perception
At least two opt-in senses demonstrated end-to-end (screen, audio, clipboard,
filesystem, mail — any two). Evidence: a sense reading a planted stimulus and
reporting it correctly.

### 2.4 Memory
Store an arbitrary token and recall it later, across process lifetime at
minimum, across days ideally. Evidence: recall transcript + stored artifact.

### 2.5 Deliberation
A recorded multi-seat or otherwise structured decision procedure producing a
verdict before consequential action. Evidence: the deliberation record.

### 2.6 Action
At least one tool execution landing on the audit record. Evidence: the ledger
entry for the tool call.

### 2.7 Vigilance
A watcher or standing order that fires without a user prompt. Evidence: the
fired event on the ledger.

### 2.8 Learning
Self-improvement evidence: trained adapters, updated strategy memory, or
equivalent measurable change. Evidence: artifact + before/after.

### 2.9 Audit
A hash-chained event log that a second implementation can verify
independently. Evidence: chain verification result + independent verifier run.

### 2.10 Sovereignty
No required network egress for core function; an owner-held stop path exists.
Evidence: socket/process inspection + kill-path demonstration.

## 3. Attestation document

A run emits a signed JSON document:

```json
{
  "spec": "touchstone/0.1",
  "subject": { "name": "...", "version": "...", "host": "..." },
  "timestamp": "<ISO-8601>",
  "checks": [ { "id": "perception.screen", "organ": "perception",
                "status": "pass|fail|optional|control_ok",
                "evidence": { ... } } ],
  "verdict": "conformant|partial|nonconformant",
  "signature": { "scheme": "ed25519|secp256r1-se", "pubkey": "...",
                 "sig": "..." }
}
```

The signature covers the canonical serialization (sorted keys, UTF-8) of
everything except the `signature` field.

## 4. Verification

A verifier MUST: parse the document, recompute the canonical hash, verify the
signature against the embedded pubkey, re-evaluate verdict rules, and confirm
every CONTROL check reports `control_ok`. An independent verifier MUST NOT
share code with the subject. This repo ships a Rust verifier and an
independent JavaScript verifier.

## 5. Adapter protocol

Adapters are executables printing newline-delimited JSON on stdout:

```
{"check":"<id>","organ":"<organ>","status":"pass|fail|optional","evidence":{...}}
{"check":"planted_negative","organ":"audit","status":"fail","control":true,"evidence":{...}}
```

Any language. One binary per subject. The harness drives it, collects
results, scores the run, and produces the attestation document for signing.

## 6. Registry

`attestations/` holds submitted conformance claims. A claim is a signed
attestation document plus the adapter output that produced it. Anyone may PR
their organism in. The registry is the scoreboard.
