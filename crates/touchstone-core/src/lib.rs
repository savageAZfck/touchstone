//! touchstone-core: the types and verdict rules for the personal AGI
//! conformance spec. See SPEC.md at the repo root for the normative text.
//!
//! This crate deliberately has no filesystem, network, or process
//! dependencies so it can compile to WASM and be shared by every verifier.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The organs a conformant organism must demonstrate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Organ {
    Awake,
    Identity,
    Perception,
    Memory,
    Deliberation,
    Action,
    Vigilance,
    Learning,
    Audit,
    Sovereignty,
}

impl Organ {
    /// Every required organ, in spec order.
    pub const ALL: [Organ; 10] = [
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
    ];
}

/// Status of a single check, as emitted by an adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Pass,
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
    pub organ: Organ,
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
    pub name: String,
    pub version: String,
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
    pub subject: Subject,
    /// ISO-8601 timestamp.
    pub timestamp: String,
    pub checks: Vec<CheckResult>,
    pub verdict: Verdict,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Signature>,
}

/// Compute the verdict for a set of check results.
///
/// - `Conformant`: every organ has >=1 PASS, no required check FAILed,
///   every control check reported ControlOk (i.e. it failed as designed).
/// - `Nonconformant`: a control check passed (instrumentation can't be
///   trusted) or the run produced no checks at all.
/// - `Partial`: anything else.
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
    let mut organs_passed = [false; 10];
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
pub fn canonical_bytes(doc: &Attestation) -> Vec<u8> {
    let mut v = serde_json::to_value(doc).expect("attestation serializes");
    if let Some(obj) = v.as_object_mut() {
        obj.remove("signature");
    }
    canonical_json(&v).into_bytes()
}

/// SHA-256 of the canonical document — what signatures cover.
pub fn document_hash(doc: &Attestation) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(canonical_bytes(doc));
    h.finalize().into()
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
