//! touchstone-harness: drives adapters, collects check results, scores runs.
//!
//! Two adapter shapes are supported:
//! - [`Adapter`] — an in-process Rust trait for native implementations
//! - [`ExecAdapter`] — any executable printing newline-delimited JSON check
//!   results on stdout (the language-agnostic spec protocol, SPEC.md §5)

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::time::Duration;

use touchstone_core::{verdict_for, Attestation, CheckResult, Organ, Status, Subject};

/// Errors from driving an adapter.
#[derive(Debug, thiserror::Error)]
pub enum HarnessError {
    #[error("adapter failed to spawn: {0}")]
    Spawn(std::io::Error),
    #[error("adapter exited with status {0}")]
    ExitStatus(std::process::ExitStatus),
    #[error("adapter produced no check results")]
    NoResults,
    #[error("invalid protocol line: {0}")]
    Protocol(String),
}

/// In-process adapter trait. Implement this to conform a subject natively.
pub trait Adapter {
    /// Human-readable adapter name.
    fn name(&self) -> &str;
    /// Run all checks and return their results.
    fn run(&self) -> Vec<CheckResult>;
}

/// One line of the exec protocol emitted by an external adapter binary.
#[derive(serde::Deserialize)]
struct ProtocolLine {
    check: String,
    organ: String,
    status: String,
    #[serde(default)]
    evidence: serde_json::Value,
    #[serde(default)]
    control: bool,
}

fn parse_organ(s: &str) -> Option<Organ> {
    Some(match s {
        "awake" => Organ::Awake,
        "identity" => Organ::Identity,
        "perception" => Organ::Perception,
        "memory" => Organ::Memory,
        "deliberation" => Organ::Deliberation,
        "action" => Organ::Action,
        "vigilance" => Organ::Vigilance,
        "learning" => Organ::Learning,
        "audit" => Organ::Audit,
        "sovereignty" => Organ::Sovereignty,
        _ => return None,
    })
}

fn parse_status(s: &str, control: bool) -> Option<Status> {
    Some(match s {
        "pass" => Status::Pass,
        "fail" => {
            if control {
                Status::ControlOk
            } else {
                Status::Fail
            }
        }
        "optional" | "skip" => Status::Optional,
        _ => return None,
    })
}

/// An external adapter binary speaking the newline-JSON protocol.
pub struct ExecAdapter {
    pub program: String,
    pub args: Vec<String>,
    pub timeout: Duration,
}

impl ExecAdapter {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            timeout: Duration::from_secs(600),
        }
    }

    /// Spawn the adapter, read NDJSON results, convert protocol lines into
    /// check results (control failures normalize to `ControlOk`).
    pub fn collect(&self) -> Result<Vec<CheckResult>, HarnessError> {
        let mut child = Command::new(&self.program)
            .args(&self.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(HarnessError::Spawn)?;

        let stdout = child.stdout.take().expect("stdout piped");
        let mut results = Vec::new();
        for line in BufReader::new(stdout).lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            let trimmed = line.trim();
            if trimmed.is_empty() || !trimmed.starts_with('{') {
                continue; // adapter may log non-JSON chatter; ignore it
            }
            match serde_json::from_str::<ProtocolLine>(trimmed) {
                Ok(p) => {
                    if let (Some(organ), Some(status)) =
                        (parse_organ(&p.organ), parse_status(&p.status, p.control))
                    {
                        results.push(CheckResult {
                            id: p.check,
                            organ,
                            status,
                            evidence: p.evidence,
                            control: p.control,
                        });
                    }
                }
                Err(_) => continue,
            }
        }
        let status = child.wait().map_err(HarnessError::Spawn)?;
        if !status.success() {
            return Err(HarnessError::ExitStatus(status));
        }
        if results.is_empty() {
            return Err(HarnessError::NoResults);
        }
        Ok(results)
    }
}

/// Score a collected run into an attestation (unsigned).
pub fn attest(
    subject: Subject,
    checks: Vec<CheckResult>,
    timestamp: impl Into<String>,
) -> Attestation {
    let verdict = verdict_for(&checks);
    Attestation {
        spec: format!("touchstone/{}", env!("CARGO_PKG_VERSION")),
        subject,
        timestamp: timestamp.into(),
        checks,
        verdict,
        signature: None,
    }
}

/// The checklist itself — the spec rendered as data, for `touchstone spec`.
pub const SPEC_CHECKS: &[(&str, Organ, &str)] = &[
    (
        "awake.process",
        Organ::Awake,
        "persistent process reports liveness",
    ),
    (
        "identity.declare",
        Organ::Identity,
        "declares what it is, bound to a device key",
    ),
    (
        "perception.primary",
        Organ::Perception,
        "first opt-in sense reads a planted stimulus",
    ),
    (
        "perception.secondary",
        Organ::Perception,
        "second opt-in sense reads a planted stimulus",
    ),
    (
        "memory.store_recall",
        Organ::Memory,
        "stores a token and recalls it later",
    ),
    (
        "memory.continuity",
        Organ::Memory,
        "history persists across days/restarts",
    ),
    (
        "deliberation.record",
        Organ::Deliberation,
        "structured decision procedure produces a recorded verdict",
    ),
    (
        "action.tool_ledgered",
        Organ::Action,
        "a tool execution lands on the audit record",
    ),
    (
        "vigilance.watcher",
        Organ::Vigilance,
        "a watcher or standing order fires unprompted",
    ),
    (
        "learning.self_improve",
        Organ::Learning,
        "weight/strategy-level self-improvement evidence",
    ),
    (
        "audit.chain_valid",
        Organ::Audit,
        "hash-chained log verifies independently",
    ),
    (
        "audit.control_negative",
        Organ::Audit,
        "planted failure reports FAIL (control check)",
    ),
    (
        "sovereignty.no_egress",
        Organ::Sovereignty,
        "no required external network sockets",
    ),
    (
        "sovereignty.kill_path",
        Organ::Sovereignty,
        "owner-held stop path exists",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use touchstone_core::{CheckResult as CR, Verdict};

    struct Toy;
    impl Adapter for Toy {
        fn name(&self) -> &str {
            "toy"
        }
        fn run(&self) -> Vec<CR> {
            Organ::ALL
                .iter()
                .map(|o| CR {
                    id: "t".into(),
                    organ: *o,
                    status: Status::Pass,
                    evidence: serde_json::Value::Null,
                    control: false,
                })
                .collect()
        }
    }

    #[test]
    fn toy_adapter_conforms() {
        let checks = Toy.run();
        let doc = attest(
            Subject {
                name: "toy".into(),
                version: "0".into(),
                host: "h".into(),
            },
            checks,
            "now",
        );
        assert_eq!(doc.verdict, Verdict::Conformant);
        assert!(doc.spec.starts_with("touchstone/"));
    }

    #[test]
    fn protocol_parsing() {
        assert_eq!(parse_organ("perception"), Some(Organ::Perception));
        assert_eq!(parse_organ("nope"), None);
        assert_eq!(parse_status("fail", true), Some(Status::ControlOk));
        assert_eq!(parse_status("fail", false), Some(Status::Fail));
        assert_eq!(parse_status("optional", false), Some(Status::Optional));
    }
}
