//! End-to-end: mock adapter -> run -> attest -> verify, plus tamper rejection.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_touchstone");

fn tmpdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("touchstone-it-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_mock_adapter(dir: &Path) -> PathBuf {
    // Minimal conformant subject: one pass per organ (vigilance+sovereignty
    // have >1 required check so we emit extras), one planted control fail.
    let script = r#"#!/bin/sh
cat <<'EOF'
{"check":"awake.status","organ":"awake","status":"pass","evidence":"heartbeat ok"}
{"check":"identity.self_model","organ":"identity","status":"pass","evidence":"named"}
{"check":"perception.screen","organ":"perception","status":"pass","evidence":"frame captured"}
{"check":"perception.ambient","organ":"perception","status":"optional","evidence":"ears off"}
{"check":"memory.store_recall","organ":"memory","status":"pass","evidence":"round-trip"}
{"check":"deliberation.council","organ":"deliberation","status":"pass","evidence":"council spoke"}
{"check":"action.ledgered","organ":"action","status":"pass","evidence":"call in ledger"}
{"check":"vigilance.watchers","organ":"vigilance","status":"pass","evidence":"2 watchers"}
{"check":"vigilance.sentinel","organ":"vigilance","status":"pass","evidence":"armed"}
{"check":"learning.dream","organ":"learning","status":"pass","evidence":"nightly dream"}
{"check":"audit.chain","organ":"audit","status":"pass","evidence":"1024 events valid"}
{"check":"audit.control","organ":"audit","status":"fail","evidence":"planted","control":true}
{"check":"sovereignty.egress","organ":"sovereignty","status":"pass","evidence":"no egress"}
{"check":"sovereignty.kill","organ":"sovereignty","status":"pass","evidence":"kill switch present"}
EOF
"#;
    let path = dir.join("mock-adapter.sh");
    fs::write(&path, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(BIN).args(args).output().unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn run_attest_verify_round_trip() {
    let dir = tmpdir("rt");
    let adapter = write_mock_adapter(&dir);
    let raw = dir.join("raw.json");
    let signed = dir.join("signed.json");
    // Deterministic test key — 32-byte hex secret, NOT a file path.
    let key = "0101010101010101010101010101010101010101010101010101010101010101";

    // spec
    let (ok, stdout, _) = run(&["spec"]);
    assert!(ok);
    assert!(stdout.contains("perception"));

    // run
    let (ok, stdout, stderr) = run(&[
        "run",
        "--adapter",
        adapter.to_str().unwrap(),
        "--subject",
        "mock",
        "--subject-version",
        "0.0.1",
        "--out",
        raw.to_str().unwrap(),
    ]);
    assert!(ok, "run failed: {stderr}");
    // verdict is reported on stderr; exit code 0 == Conformant
    assert!(stderr.contains("Conformant"), "stderr: {stderr}");
    let doc: serde_json::Value = serde_json::from_str(&fs::read_to_string(&raw).unwrap()).unwrap();
    assert_eq!(doc["verdict"], "conformant");
    assert_eq!(doc["checks"].as_array().unwrap().len(), 14);
    let _ = stdout;

    // attest
    let (ok, _, stderr) = run(&[
        "attest",
        raw.to_str().unwrap(),
        "--key",
        key,
        "--out",
        signed.to_str().unwrap(),
    ]);
    assert!(ok, "attest failed: {stderr}");

    // verify
    let (ok, stdout, _) = run(&["verify", signed.to_str().unwrap()]);
    assert!(ok);
    assert!(stdout.contains("signature: VALID"));

    // explore
    let (ok, stdout, _) = run(&["explore", signed.to_str().unwrap()]);
    assert!(ok);
    assert!(stdout.contains("mock"));

    // tamper: flip a check status, signature must fail
    let mut doc: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&signed).unwrap()).unwrap();
    doc["checks"][0]["status"] = serde_json::json!("fail");
    let tampered = dir.join("tampered.json");
    fs::write(&tampered, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
    let (ok, stdout, stderr) = run(&["verify", tampered.to_str().unwrap()]);
    assert!(!ok, "tampered doc verified");
    // dies in structural validation (verdict mismatch) or signature check
    assert!(
        stdout.contains("INVALID") || stderr.contains("invalid") || stdout.contains("failed"),
        "stdout: {stdout} stderr: {stderr}"
    );

    // verdict lie: claim Nonconformant on a conformant doc, signature intact
    // (signature covers the lie too, so recompute-check catches it)
    let mut doc2: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&signed).unwrap()).unwrap();
    doc2["verdict"] = serde_json::json!("nonconformant");
    // re-sign the lied doc so signature is valid but verdict is wrong
    let lied = dir.join("lied.json");
    fs::write(&lied, serde_json::to_string_pretty(&doc2).unwrap()).unwrap();
    // strip sig and re-attest
    let mut unsigned = doc2.clone();
    unsigned["signature"] = serde_json::Value::Null;
    fs::write(&lied, serde_json::to_string_pretty(&unsigned).unwrap()).unwrap();
    let resigned = dir.join("lied-signed.json");
    let (ok, _, _) = run(&[
        "attest",
        lied.to_str().unwrap(),
        "--key",
        key,
        "--out",
        resigned.to_str().unwrap(),
    ]);
    assert!(ok);
    let (ok, stdout, stderr) = run(&["verify", resigned.to_str().unwrap()]);
    assert!(!ok, "verdict lie passed verification: {stdout}");
    assert!(
        stdout.contains("recomputed") || stderr.contains("recomputed"),
        "stdout: {stdout} stderr: {stderr}"
    );
}
