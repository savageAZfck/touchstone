//! touchstone — the personal AGI conformance battery.
//!
//! `touchstone spec`     prints the checklist
//! `touchstone run`      drives an adapter and emits an unsigned attestation
//! `touchstone attest`   signs an attestation with a device key
//! `touchstone verify`   verifies signature + verdict rules on an attestation

#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use touchstone_core::{document_hash, verdict_for, Attestation, Organ, Status};
use touchstone_harness::{attest, ExecAdapter, SPEC_CHECKS};
use touchstone_identity::{verify_attestation, Ed25519Signer};

#[derive(Parser)]
#[command(
    name = "touchstone",
    version,
    about = "Personal AGI conformance battery — verify, don't trust"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Print the conformance checklist.
    Spec,
    /// Run the battery against an adapter.
    Run {
        /// Adapter name (resolves to `adapters/<name>/...` or PATH) or a
        /// path to an adapter executable speaking the NDJSON protocol.
        #[arg(long)]
        adapter: String,
        /// Subject name recorded in the attestation.
        #[arg(long, default_value = "unknown")]
        subject: String,
        /// Subject version recorded in the attestation.
        #[arg(long, default_value = "0.0.0")]
        subject_version: String,
        /// Extra args passed through to the adapter.
        #[arg(long, trailing_var_arg = true, num_args = 0..)]
        adapter_args: Vec<String>,
        /// Write the attestation here instead of stdout.
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// Adapter wall-clock timeout in seconds (default 600).
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// Sign an unsigned attestation JSON file with a fresh or given key.
    Attest {
        /// The attestation JSON to sign.
        file: PathBuf,
        /// Hex-encoded Ed25519 secret; generates an ephemeral key if unset.
        #[arg(long, env = "TOUCHSTONE_KEY")]
        key: Option<String>,
        /// Write signed attestation here.
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Verify a signed attestation: signature + verdict rules.
    Verify {
        /// The signed attestation JSON.
        file: PathBuf,
    },
    /// Pretty-print an attestation's organ scoreboard.
    Explore {
        /// The attestation JSON (signed or unsigned).
        file: PathBuf,
    },
}

fn resolve_adapter(name_or_path: &str) -> Result<String, String> {
    let p = PathBuf::from(name_or_path);
    if p.is_file() {
        return Ok(p.display().to_string());
    }
    // workspace adapter binary convention: target/{release,debug}/<name>-adapter
    for profile in ["release", "debug"] {
        let candidate = PathBuf::from(format!("target/{profile}/{name_or_path}-adapter"));
        if candidate.is_file() {
            return Ok(candidate.display().to_string());
        }
    }
    // fall back to PATH lookup — the binary may be installed
    if let Ok(path) = which_in_path(name_or_path) {
        return Ok(path);
    }
    Err(format!(
        "adapter '{name_or_path}' not found (looked for file, target/*/{name_or_path}-adapter, and on PATH)"
    ))
}

fn which_in_path(name: &str) -> Result<String, ()> {
    let path_var = std::env::var("PATH").map_err(|_| ())?;
    for dir in path_var.split(':') {
        let candidate = PathBuf::from(dir).join(name);
        if candidate.is_file() {
            return Ok(candidate.display().to_string());
        }
    }
    Err(())
}

fn cmd_spec() {
    println!(
        "touchstone conformance checklist (spec v{})",
        touchstone_core::SPEC_VERSION
    );
    println!();
    let mut by_organ: Vec<(Organ, Vec<(&str, &str)>)> = Vec::new();
    for (id, organ, desc) in SPEC_CHECKS {
        if let Some((_, v)) = by_organ.iter_mut().find(|(o, _)| o == organ) {
            v.push((id, desc));
        } else {
            by_organ.push((*organ, vec![(id, desc)]));
        }
    }
    for (organ, checks) in by_organ {
        println!("  {organ:?}");
        for (id, desc) in checks {
            println!("    {id:28} {desc}");
        }
    }
    println!();
    println!("verdicts: conformant = all organs PASS + every control FAILs as designed");
}

fn cmd_run(
    adapter: &str,
    subject: String,
    subject_version: String,
    adapter_args: Vec<String>,
    out: Option<PathBuf>,
    timeout: Option<u64>,
) -> ExitCode {
    let program = match resolve_adapter(adapter) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!("[touchstone] driving adapter: {program}");
    let mut exec = ExecAdapter::new(program);
    exec.args = adapter_args;
    if let Some(t) = timeout {
        exec.timeout = std::time::Duration::from_secs(t);
    }
    let checks = match exec.collect() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let host = std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".into());
    let doc = attest(
        touchstone_core::Subject {
            name: subject,
            version: subject_version,
            host,
        },
        checks,
        chrono::Utc::now().to_rfc3339(),
    );
    let json = serde_json::to_string_pretty(&doc).unwrap();
    match out {
        Some(path) => {
            if let Err(e) = fs::write(&path, &json) {
                eprintln!("error writing {}: {e}", path.display());
                return ExitCode::FAILURE;
            }
            eprintln!(
                "[touchstone] verdict: {:?} → {}",
                doc.verdict,
                path.display()
            );
        }
        None => println!("{json}"),
    }
    match doc.verdict {
        touchstone_core::Verdict::Conformant => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}

fn cmd_attest(file: &Path, key: Option<&str>, out: Option<PathBuf>) -> ExitCode {
    let text = match fs::read_to_string(file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error reading {}: {e}", file.display());
            return ExitCode::FAILURE;
        }
    };
    let mut doc: Attestation = match serde_json::from_str(&text) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error parsing attestation: {e}");
            return ExitCode::FAILURE;
        }
    };
    if doc.signature.is_some() {
        eprintln!("warning: attestation already signed — replacing signature");
    }
    if doc.checks.is_empty() {
        eprintln!("error: attestation has no checks — nothing to attest");
        return ExitCode::FAILURE;
    }
    let signer = if let Some(k) = key {
        if let Ok(s) = Ed25519Signer::from_hex(k) {
            s
        } else {
            eprintln!("error: --key must be 32-byte hex");
            return ExitCode::FAILURE;
        }
    } else {
        let s = Ed25519Signer::generate();
        eprintln!("[touchstone] ephemeral key generated: {}", s.public_hex());
        eprintln!("[touchstone] save the secret if you want stable identity:");
        eprintln!("  {}", s.secret_hex());
        s
    };
    signer.sign_attestation(&mut doc);
    let json = serde_json::to_string_pretty(&doc).unwrap();
    let path = out.unwrap_or_else(|| file.with_extension("signed.json"));
    if let Err(e) = fs::write(&path, &json) {
        eprintln!("error writing {}: {e}", path.display());
        return ExitCode::FAILURE;
    }
    eprintln!("[touchstone] signed → {}", path.display());
    ExitCode::SUCCESS
}

fn cmd_verify(file: &Path) -> ExitCode {
    let text = match fs::read_to_string(file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error reading {}: {e}", file.display());
            return ExitCode::FAILURE;
        }
    };
    let doc: Attestation = match serde_json::from_str(&text) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error parsing attestation: {e}");
            return ExitCode::FAILURE;
        }
    };
    // Structural validation first — malformed docs die here regardless of
    // whether a valid signature happens to cover them.
    if let Err(errs) = doc.validate() {
        for e in &errs {
            eprintln!("invalid: {e}");
        }
        return ExitCode::FAILURE;
    }
    let recomputed = verdict_for(&doc.checks);
    match verify_attestation(&doc) {
        Ok(true) => {
            println!("signature: VALID");
            println!("doc hash:  {}", hex::encode(document_hash(&doc)));
            println!("verdict:   {:?}", doc.verdict);
            if recomputed != doc.verdict {
                println!(
                    "warning:   claimed verdict {:?} != recomputed {:?}",
                    doc.verdict, recomputed
                );
                return ExitCode::FAILURE;
            }
            let (pass, fail, opt, ctrl) =
                doc.checks
                    .iter()
                    .fold((0u32, 0u32, 0u32, 0u32), |(p, f, o, c), ch| {
                        match ch.status {
                            touchstone_core::Status::Pass => (p + 1, f, o, c),
                            touchstone_core::Status::Fail => (p, f + 1, o, c),
                            touchstone_core::Status::Optional => (p, f, o + 1, c),
                            touchstone_core::Status::ControlOk => (p, f, o, c + 1),
                        }
                    });
            println!("checks:    {pass} pass / {fail} fail / {opt} optional / {ctrl} controls ok");
            ExitCode::SUCCESS
        }
        Ok(false) => {
            println!("signature: INVALID");
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("verify error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Spec => {
            cmd_spec();
            ExitCode::SUCCESS
        }
        Cmd::Run {
            adapter,
            subject,
            subject_version,
            adapter_args,
            out,
            timeout,
        } => cmd_run(&adapter, subject, subject_version, adapter_args, out, timeout),
        Cmd::Attest { file, key, out } => cmd_attest(&file, key.as_deref(), out),
        Cmd::Verify { file } => cmd_verify(&file),
        Cmd::Explore { file } => cmd_explore(&file),
    }
}

fn cmd_explore(file: &Path) -> ExitCode {
    let text = match fs::read_to_string(file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error reading {}: {e}", file.display());
            return ExitCode::FAILURE;
        }
    };
    let doc: Attestation = match serde_json::from_str(&text) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error parsing attestation: {e}");
            return ExitCode::FAILURE;
        }
    };
    println!(
        "{} v{} @ {} — spec {}",
        doc.subject.name, doc.subject.version, doc.subject.host, doc.spec
    );
    println!("timestamp: {}", doc.timestamp);
    println!("verdict:   {:?}", doc.verdict);
    println!();
    for organ in Organ::ALL {
        let organ_checks: Vec<_> = doc.checks.iter().filter(|c| c.organ == organ).collect();
        if organ_checks.is_empty() {
            continue;
        }
        println!("  {organ:?}");
        for c in organ_checks {
            let mark = match c.status {
                Status::Pass => "PASS",
                Status::Fail => "FAIL",
                Status::Optional => "OPT ",
                Status::ControlOk => "CTRL",
            };
            println!("    [{mark}] {}", c.id);
        }
    }
    println!();
    match &doc.signature {
        Some(s) => println!(
            "signature: {} (pub {}...)",
            s.scheme,
            &s.pubkey[..s.pubkey.len().min(16)]
        ),
        None => println!("signature: unsigned"),
    }
    ExitCode::SUCCESS
}
