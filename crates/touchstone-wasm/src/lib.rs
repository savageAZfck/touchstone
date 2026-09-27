//! touchstone-wasm: verify attestations in the browser.

#![forbid(unsafe_code)]

use touchstone_core::{document_hash, verdict_for, Attestation};
use touchstone_identity::verify_attestation;
use wasm_bindgen::prelude::*;

/// Verify a signed attestation JSON string. Returns a JSON verdict object:
/// { ok, signature_valid, verdict, checks, doc_hash, error? }
#[wasm_bindgen]
pub fn verify_attestation_json(json: &str) -> String {
    let doc: Attestation = match serde_json::from_str(json) {
        Ok(d) => d,
        Err(e) => {
            return serde_json::json!({"ok": false, "error": format!("parse: {e}")}).to_string();
        }
    };
    let sig_ok = verify_attestation(&doc).unwrap_or(false);
    let recomputed = verdict_for(&doc.checks);
    let verdict_consistent = recomputed == doc.verdict;
    serde_json::json!({
        "ok": sig_ok && verdict_consistent,
        "signature_valid": sig_ok,
        "verdict": doc.verdict,
        "verdict_consistent": verdict_consistent,
        "recomputed_verdict": recomputed,
        "checks": doc.checks.len(),
        "doc_hash": hex::encode(document_hash(&doc)),
        "subject": doc.subject.name,
        "spec": doc.spec,
    })
    .to_string()
}
