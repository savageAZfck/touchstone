# Touchstone — Personal AGI Organism Conformance Spec (v0.2, draft)

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

### 1.1 Deterministic verdict computation

Given the multiset of check results, the verdict MUST be computed exactly as:

```
if the check list is empty                       -> NONCONFORMANT
if any control check reports PASS                -> NONCONFORMANT
if all thirteen organs have >=1 PASS
   and no non-control check is FAIL              -> CONFORMANT
else                                             -> PARTIAL
```

Notes:

- Conformance is per-organ: each of the thirteen organs needs at least one `PASS`
  check. A missing organ (no check for it at all) yields `PARTIAL`.
- A `CONTROL` check reporting `control_ok` is success; reporting `pass`
  proves the harness cannot fail checks, which is `NONCONFORMANT`.
- `OPTIONAL` checks never block conformance — an organ passes on its `PASS`
  checks alone.
- Verdict is a pure function of check results. Two conformant implementations
  MUST produce identical verdicts from identical check lists.

### 1.2 Spec versioning

The spec version (`touchstone/<major.minor>`) is independent of any
implementation's semver. The minor version bumps when checks are added or
verdict rules change; major bumps on incompatible format changes. An
attestation's `spec` field records which version produced it. Verifiers MUST
accept `touchstone/0.x` documents and SHOULD warn on unknown majors rather
than silently accept.

## 2. Required organs

An organism MUST demonstrate each of the following, with evidence the harness
can verify:

### 2.1 Awake
A persistent process exists and reports liveness. Evidence: process identity,
uptime, runtime signature.

*Grounding:* Wooldridge & Jennings' agent notion distinguishes an agent — a
continuous, running entity — from a function invoked per call [W2].

### 2.2 Identity
The organism declares what it is when asked, and the declaration binds to a
device-held key. Evidence: signed self-declaration.

*Grounding:* an agent must be identifiable to be accountable; the social-
ability property in Wooldridge & Jennings presumes a stable self [W2]. The
key binding applies Haber & Stornetta's commitment technique to the
self-declaration itself [H1].

### 2.3 Perception
At least two opt-in senses demonstrated end-to-end (screen, audio, clipboard,
filesystem, mail — any two). Evidence: a sense reading a planted stimulus and
reporting it correctly.

*Grounding:* Russell & Norvig's canonical definition — an agent perceives
its environment through sensors and acts through actuators [R1].

### 2.4 Memory
Store an arbitrary token and recall it later, across process lifetime at
minimum, across days ideally. Evidence: recall transcript + stored artifact.

*Grounding:* every general cognitive architecture requires episodic memory —
Soar's chunked experience memory and ACT-R's declarative store both exist
because no architecture claiming generality runs on working memory alone
[L1, A1].

### 2.5 Deliberation
A recorded multi-seat or otherwise structured decision procedure producing a
verdict before consequential action. Evidence: the deliberation record.

*Grounding:* Rao & Georgeff's BDI model requires an explicit deliberation
between beliefs/desires and committed intention; unmediated stimulus→action
is reflex, not agency [G1].

### 2.6 Action
At least one tool execution landing on the audit record. Evidence: the ledger
entry for the tool call.

*Grounding:* Russell & Norvig's actuator half of the agent loop [R1]; the
record requirement follows from the accountability literature — an
unrecorded effect is unattributable [H1].

### 2.7 Vigilance
A watcher or standing order that fires without a user prompt. Evidence: the
fired event on the ledger.

*Grounding:* Wooldridge & Jennings' pro-activeness property — agents take
initiative toward standing goals rather than only reacting [W2].

### 2.8 Learning
Self-improvement evidence: trained adapters, updated strategy memory, or
equivalent measurable change. Evidence: artifact + before/after.

*Grounding:* Legg & Hutter's measure weights performance improvement across
environments; a system that cannot get better is not exhibiting adaptive
intelligence [L2].

### 2.9 Audit
A hash-chained event log that a second implementation can verify
independently. Evidence: chain verification result + independent verifier run.

*Grounding:* Haber & Stornetta's linked time-stamping scheme — tamper-
evidence through hash commitments — applied to the organism's own record
[H1].

### 2.10 Sovereignty
No required network egress for core function; an owner-held stop path exists.
Evidence: socket/process inspection + kill-path demonstration.

*Grounding:* the corrigibility literature's off-switch requirement, directed
correctly: the interruption channel must terminate at the owner, not a
vendor [S1].

### 2.11 Generality
Goal achievement demonstrated across at least three distinct task domains
(e.g., filesystem action + information synthesis + scheduling), not a
single scripted capability. Generality is structural: it proves breadth of
operation, not benchmark scores. Evidence: per-domain task results.

*Grounding:* Legg & Hutter's "wide range of environments" clause — the G in
AGI is environmental breadth, not task depth [L2].

### 2.12 Planning
Multi-step task decomposition distinct from one-shot deliberation: a
recorded plan with ordered steps, executed, with evidence of replanning
when a step fails. Evidence: the plan record plus execution trace.

*Grounding:* STRIPS-style deliberative planning — an explicit operator
sequence whose execution can fail and be repaired — is the canonical
distinction between planning and deliberation [F1].

### 2.13 Reflection
The organism detects and corrects its own errors: a failed action followed
by a recorded adjustment, or an explicit self-correction artifact.
Evidence: ledger/replay record showing error → corrected approach.

*Grounding:* metareasoning — computation about one's own computation — is
the literature's term for the error-detection/correction loop that
separates adaptive systems from reactive ones [C1].

## 3. Attestation document

A run emits a signed JSON document:

```json
{
  "spec": "touchstone/0.2",
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

The signature covers the canonical serialization of everything except the
`signature` field.

### 3.1 Canonicalization

The signed document is canonicalized before hashing. Rules:

1. Remove the `signature` field entirely.
2. Serialize to JSON with **all object keys sorted lexicographically**, at
   every depth, recursively.
3. No whitespace: no spaces after `:` or `,`, no indentation, no trailing
   newline.
4. Strings are UTF-8, escaped per JSON spec (shortest form).
5. Numbers serialize in shortest round-trip form (`1` not `1.0`).
6. The hash committed to the signature is `SHA-256` of the canonical bytes.

Any implementation producing the same canonical bytes from the same document
is conformant; byte differences (e.g. key order) mean a different hash and a
broken signature check, which is the intended behavior.

## 4. Verification

A verifier MUST, in order:

1. Parse the document and reject malformed JSON.
2. Structurally validate: `spec` is `touchstone/<version>`, timestamp is
   RFC3339, check ids are unique, `signature` fields are well-formed hex.
3. **Recompute the verdict from the check list** and reject if it differs
   from the claimed `verdict` — a valid signature over a false verdict is
   still a failed attestation.
4. Recompute the canonical hash and verify the signature against the
   embedded pubkey.
5. Confirm every CONTROL check reports `control_ok`.

An independent verifier MUST NOT share code with the subject. This repo
ships a Rust verifier (`touchstone verify`) and an independent JavaScript
verifier (`verifiers/js/verify.mjs`) with zero shared code.

## 5. Adapter protocol

Adapters are executables the harness spawns once per run. The adapter prints
**newline-delimited JSON** on stdout, one check result per line:

```
{"check":"<id>","organ":"<organ>","status":"pass|fail|optional","evidence":{...}}
{"check":"planted_negative","organ":"audit","status":"fail","control":true,"evidence":{...}}
```

### 5.1 Request/response model

One process spawn = one full battery. The harness sends nothing on stdin;
the adapter emits all check results then exits. There is no per-check
request — the adapter is the source of truth about what it measured.

### 5.2 Output semantics

- Each line MUST be a complete JSON object (NDJSON — one value per line).
- Lines that are empty or do not start with `{` MUST be ignored (log
  chatter is permitted on stdout).
- Lines that fail JSON parsing MUST be ignored (treated as chatter).
- Lines with unknown `organ` or `status` values MUST be ignored.
- `evidence` is a free-form object — the harness stores it opaquely.
- stderr is inherited by the harness for diagnostics; it carries no
  protocol meaning.

### 5.3 Termination and failure

- The adapter MUST exit 0 when finished. Nonzero exit after emitting
  results is a harness error, not a check failure.
- The harness enforces a wall-clock **timeout** (default 600 s). An adapter
  exceeding it is killed; the run is recorded as a harness error.
- An adapter emitting zero parseable check results is a harness error
  (`NoResults`), distinct from a run of all-fail checks.

### 5.4 Duplicate and unknown checks

Duplicate check ids are protocol errors — the attestation validator rejects
them. Check ids outside the spec's named set are permitted: they appear on
the scoreboard but do not affect the verdict, letting subjects report extra
organs without breaking conformant scoring.

Any language. One binary per subject. The harness drives it, collects
results, scores the run, and produces the attestation document for signing.

## 6. Registry

`attestations/` holds submitted conformance claims. A claim is a signed
attestation document plus the adapter output that produced it. Anyone may PR
their organism in. The registry is the scoreboard.

## 7. References

The thirteen organs operationalize established agent and AGI literature;
each organ's grounding line cites its root.

- [R1] S. Russell, P. Norvig. *Artificial Intelligence: A Modern Approach.*
  Agent = perceive (sensors) + act (effectors). Organs: perception, action.
- [W2] M. Wooldridge, N. Jennings. "Intelligent Agents: Theory and
  Practice." *Knowledge Engineering Review* 10(2), 1995. Autonomy,
  pro-activeness, social ability, persistence. Organs: awake, identity,
  vigilance.
- [L2] S. Legg, M. Hutter. "Universal Intelligence: A Definition of Machine
  Intelligence." *Minds and Machines* 17(4), 2007. Goal achievement across a
  wide range of environments. Organs: generality, learning.
- [G1] A. Rao, M. Georgeff. "BDI Agents: From Theory to Practice." *ICMAS*
  1995. Explicit deliberation before committed intention. Organ:
  deliberation.
- [F1] R. Fikes, N. Nilsson. "STRIPS: A New Approach to the Application of
  Theorem Proving to Problem Solving." *Artificial Intelligence* 2(3-4),
  1971. Organ: planning.
- [L1] J. Laird. *The Soar Cognitive Architecture.* MIT Press, 2012. Organ:
  memory.
- [A1] J. Anderson et al. "An Integrated Theory of the Mind."
  *Psychological Review* 111(4), 2004 (ACT-R). Organ: memory.
- [C1] M. Cox, A. Raja (eds). *Metareasoning: Thinking about Thinking.*
  MIT Press, 2011. Organ: reflection.
- [H1] S. Haber, W. S. Stornetta. "How to Time-Stamp a Digital Document."
  *Journal of Cryptology* 3(2), 1991. Organs: audit, identity.
- [S1] N. Soares, B. Fallenstein, E. Yudkowsky, S. Armstrong.
  "Corrigibility." *AAAI Workshop on AI and Ethics*, 2015. Organ:
  sovereignty.

The spec deliberately does not adopt any literature claim that intelligence
*is* measurable cognition alone (e.g., IQ-style task benchmarks); this
spec's scope is organism structure, persistence, and accountability, and the
citations above define where each required organ comes from.
