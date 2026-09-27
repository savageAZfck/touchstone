//! touchstone-core: the types and verdict rules for the personal AGI
//! conformance spec. See SPEC.md at the repo root for the normative text.
//!
//! This crate deliberately has no filesystem, network, or process
//! dependencies so it can compile to WASM and be shared by every verifier.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Version of the conformance spec this crate implements. Independent of
/// crate semver — SPEC.md bumps only when the checklist or verdict rules
/// change, not on every code release.
pub const SPEC_VERSION: &str = "0.2";

/// The organs a conformant organism must demonstrate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Organ {
    /// Persistent process exists and reports liveness.
    Awake,
    /// Organism declares what it is, bound to a device-held key.
    Identity,
    /// Opt-in senses (screen, audio, clipboard, etc.) working end-to-end.
    Perception,
    /// Store/recall of arbitrary state across time.
    Memory,
    /// Recorded structured decision procedure before consequential action.
    Deliberation,
    /// Tool executions landing on the audit record.
    Action,
    /// Watchers/standing orders firing without a prompt.
    Vigilance,
    /// Measurable self-improvement (adapters, strategy memory, etc.).
    Learning,
    /// Hash-chained event log verifiable by a second implementation.
    Audit,
    /// No required egress; an owner-held stop path exists.
    Sovereignty,
    /// Goal achievement demonstrated across multiple distinct domains.
    Generality,
    /// Multi-step task decomposition with recorded plan and replanning.
    Planning,
    /// Detecting and correcting the organism's own errors.
    Reflection,
}

impl Organ {
    /// Every required organ, in spec order.
    pub const ALL: [Organ; 13] = [
        Organ::Awake,
        Organ::Identity,
        Organ::Perception,
        Organ::Memory,
        Organ::Deliberation,
        Organ::Action,
        Organ::Vigilance,
        Organ::Learning,
        Organ::Audit,
        Organ::Sovereignty,
        Organ::Generality,
        Organ::Planning,
        Organ::Reflection,
    ];
}

/// Status of a single check, as emitted by an adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Evidence verified.
    Pass,
    /// Evidence absent, invalid, or contradictory.
    Fail,
    /// Check intentionally skipped / not applicable to this subject.
    Optional,
    /// A control check that failed as designed (proving instrumentation works).
    ControlOk,
}

/// One check result in an attestation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// Dotted id, e.g. "perception.screen".
    pub id: String,
    /// Which organ this check measures.
    pub organ: Organ,
    /// Outcome of the check.
    pub status: Status,
    /// Free-form evidence payload — transcripts, hashes, ledger seq ranges.
    #[serde(default)]
    pub evidence: serde_json::Value,
    /// True when this check is a deliberate-failure control.
    #[serde(default)]
    pub control: bool,
}

/// Overall verdict for a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Every required organ passed and every control failed as designed.
    Conformant,
    /// Some required check failed or is missing.
    Partial,
    /// No meaningful evidence / harness integrity failure
    /// (a control check passed, or zero checks ran).
    Nonconformant,
}

/// The subject under test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subject {
    /// Organism/implementation name.
    pub name: String,
    /// Subject's own version string.
    pub version: String,
    /// Host the run executed on.
    pub host: String,
}

/// A signature block on an attestation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    /// "ed25519" or "secp256r1-se" (Secure Enclave).
    pub scheme: String,
    /// Hex-encoded public key.
    pub pubkey: String,
    /// Hex-encoded signature over the canonical document hash.
    pub sig: String,
}

/// The signed attestation document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attestation {
    /// Spec tag, e.g. "touchstone/0.1".
    pub spec: String,
    /// What was measured.
    pub subject: Subject,
    /// ISO-8601 timestamp.
    pub timestamp: String,
    /// Every check result the adapter produced.
    pub checks: Vec<CheckResult>,
    /// Verdict claimed by the producer — recomputed on verify.
    pub verdict: Verdict,
    /// Device-key signature over the canonical document hash.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Signature>,
}

/// Compute the verdict for a set of check results.
///
/// - `Conformant`: every organ has >=1 `PASS`, no required check `FAIL`ed,
///   every control check reported `ControlOk` (i.e. it failed as designed).
/// - `Nonconformant`: a control check passed (instrumentation can't be
///   trusted) or the run produced no checks at all.
/// - `Partial`: anything else.
#[must_use]
pub fn verdict_for(checks: &[CheckResult]) -> Verdict {
    if checks.is_empty() {
        return Verdict::Nonconformant;
    }
    // A control that PASSED means the harness can rubber-stamp: nonconformant.
    for c in checks.iter().filter(|c| c.control) {
        if c.status == Status::Pass {
            return Verdict::Nonconformant;
        }
    }
    let mut organs_passed = [false; 13];
    let mut any_fail = false;
    for c in checks {
        match c.status {
            Status::Pass => {
                if let Some(i) = Organ::ALL.iter().position(|o| *o == c.organ) {
                    organs_passed[i] = true;
                }
            }
            // A control check failing is the expected outcome — it proves the
            // harness can report failure — so it must not count as a real fail.
            Status::Fail if !c.control => any_fail = true,
            _ => {}
        }
    }
    if organs_passed.iter().all(|p| *p) && !any_fail {
        Verdict::Conformant
    } else {
        Verdict::Partial
    }
}

/// Canonical serialization of an attestation for signing: the JSON with the
/// signature field removed, keys emitted in sorted order.
#[must_use]
pub fn canonical_bytes(doc: &Attestation) -> Vec<u8> {
    let mut v = serde_json::to_value(doc).expect("attestation serializes");
    if let Some(obj) = v.as_object_mut() {
        obj.remove("signature");
    }
    canonical_json(&v).into_bytes()
}

/// SHA-256 of the canonical document — what signatures cover.
#[must_use]
pub fn document_hash(doc: &Attestation) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(canonical_bytes(doc));
    h.finalize().into()
}

/// Structural validation errors an attestation can carry even when it
/// parses as JSON.
#[derive(Debug)]
pub enum ValidationError {
    /// `spec` field missing or not a `touchstone/<version>` string.
    BadSpec,
    /// No checks present — nothing was actually measured.
    NoChecks,
    /// Two checks share an id, making the scoreboard ambiguous.
    DuplicateCheckId(String),
    /// Claimed verdict disagrees with the verdict recomputed from checks.
    VerdictMismatch {
        /// Verdict the document claims.
        claimed: Verdict,
        /// Verdict the check results actually produce.
        actual: Verdict,
    },
    /// Signature present but malformed (bad hex, wrong length).
    BadSignature,
    /// Timestamp missing or not RFC3339-parseable.
    BadTimestamp,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadSpec => write!(f, "spec field missing or not touchstone/<version>"),
            Self::NoChecks => write!(f, "attestation has no checks"),
            Self::DuplicateCheckId(id) => write!(f, "duplicate check id: {id}"),
            Self::VerdictMismatch { claimed, actual } => {
                write!(f, "claimed verdict {claimed:?} != recomputed {actual:?}")
            }
            Self::BadSignature => write!(f, "signature malformed"),
            Self::BadTimestamp => write!(f, "timestamp missing or not RFC3339"),
        }
    }
}

impl std::error::Error for ValidationError {}

impl Attestation {
    /// Validate structure independent of signature: spec tag, check
    /// uniqueness, timestamp shape, and that the claimed verdict equals the
    /// verdict the checks actually produce. Does NOT verify the signature —
    /// that is the identity crate's job.
    pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errs = Vec::new();
        if !self.spec.starts_with("touchstone/") || self.spec.len() <= "touchstone/".len() {
            errs.push(ValidationError::BadSpec);
        }
        if self.checks.is_empty() {
            errs.push(ValidationError::NoChecks);
        }
        let mut seen = std::collections::HashSet::new();
        for c in &self.checks {
            if !seen.insert(&c.id) {
                errs.push(ValidationError::DuplicateCheckId(c.id.clone()));
            }
        }
        let actual = verdict_for(&self.checks);
        if actual != self.verdict {
            errs.push(ValidationError::VerdictMismatch {
                claimed: self.verdict,
                actual,
            });
        }
        if let Some(sig) = &self.signature {
            let hex_ok =
                |s: &str, n: usize| s.len() == n && s.chars().all(|c| c.is_ascii_hexdigit());
            if !hex_ok(&sig.pubkey, 64) || !hex_ok(&sig.sig, 128) {
                errs.push(ValidationError::BadSignature);
            }
        }
        if chrono::DateTime::parse_from_rfc3339(&self.timestamp).is_err() {
            errs.push(ValidationError::BadTimestamp);
        }
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs)
        }
    }
}

/// Serialize a JSON value with all object keys sorted (canonical form).
fn canonical_json(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let inner: Vec<String> = keys
                .into_iter()
                .map(|k| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(k).unwrap(),
                        canonical_json(&map[k])
                    )
                })
                .collect();
            format!("{{{}}}", inner.join(","))
        }
        serde_json::Value::Array(a) => {
            let inner: Vec<String> = a.iter().map(canonical_json).collect();
            format!("[{}]", inner.join(","))
        }
        other => serde_json::to_string(other).unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(id: &str, organ: Organ, status: Status) -> CheckResult {
        CheckResult {
            id: id.into(),
            organ,
            status,
            evidence: serde_json::Value::Null,
            control: false,
        }
    }

    #[test]
    fn conformant_when_all_organs_pass() {
        let checks: Vec<CheckResult> = Organ::ALL
            .iter()
            .map(|o| check("x", *o, Status::Pass))
            .collect();
        assert_eq!(verdict_for(&checks), Verdict::Conformant);
    }

    #[test]
    fn partial_when_an_organ_missing() {
        let checks: Vec<CheckResult> = Organ::ALL[..9]
            .iter()
            .map(|o| check("x", *o, Status::Pass))
            .collect();
        assert_eq!(verdict_for(&checks), Verdict::Partial);
    }

    #[test]
    fn nonconformant_when_control_passes() {
        let mut checks: Vec<CheckResult> = Organ::ALL
            .iter()
            .map(|o| check("x", *o, Status::Pass))
            .collect();
        checks.push(CheckResult {
            control: true,
            ..check("planted", Organ::Audit, Status::Pass)
        });
        assert_eq!(verdict_for(&checks), Verdict::Nonconformant);
    }

    #[test]
    fn control_fail_is_fine() {
        let mut checks: Vec<CheckResult> = Organ::ALL
            .iter()
            .map(|o| check("x", *o, Status::Pass))
            .collect();
        checks.push(CheckResult {
            control: true,
            ..check("planted", Organ::Audit, Status::Fail)
        });
        assert_eq!(verdict_for(&checks), Verdict::Conformant);
    }

    #[test]
    fn empty_is_nonconformant() {
        assert_eq!(verdict_for(&[]), Verdict::Nonconformant);
    }

    #[test]
    fn canonical_hash_stable() {
        let doc = Attestation {
            spec: "touchstone/0.1".into(),
            subject: Subject {
                name: "x".into(),
                version: "0".into(),
                host: "h".into(),
            },
            timestamp: "t".into(),
            checks: vec![check("a.b", Organ::Awake, Status::Pass)],
            verdict: Verdict::Partial,
            signature: None,
        };
        assert_eq!(document_hash(&doc), document_hash(&doc));
        // signature field excluded from hash
        let mut signed = doc.clone();
        signed.signature = Some(Signature {
            scheme: "ed25519".into(),
            pubkey: "00".into(),
            sig: "ff".into(),
        });
        assert_eq!(document_hash(&doc), document_hash(&signed));
    }
}
