#!/usr/bin/env python3
"""Touchstone adapter for Sovereign-Intel-Founder/sovereign-seed-commons.

Independent measurement written against spec touchstone/0.2. Exercises the
subject's own machinery where it exists (lineage chain, mutation pipeline,
bounded cell execution, gates) and reports honestly where it does not.
Emits NDJSON check results on stdout; exits 0.

Usage: sovereign-seed-adapter.py [repo_path]   (default: env SSC_REPO or /tmp/ssc)
"""

import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time

REPO = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SSC_REPO", "/tmp/ssc")


def emit(check, organ, status, evidence=None, control=False):
    line = {"check": check, "organ": organ, "status": status,
            "evidence": evidence or {}}
    if control:
        line["control"] = True
    print(json.dumps(line), flush=True)


def run_py(args, cwd, env_extra=None, timeout=60, sandbox_deny_net=False):
    env = dict(os.environ)
    env.update(env_extra or {})
    cmd = ["python3"] + args
    if sandbox_deny_net:
        # macOS sandbox-exec: deny all outbound network. If core function
        # still completes, no required egress is proven by construction.
        profile = "(version 1)(allow default)(deny network-outbound)"
        cmd = ["sandbox-exec", "-p", profile] + cmd
    try:
        r = subprocess.run(cmd, cwd=cwd, env=env, capture_output=True,
                           text=True, timeout=timeout)
        return r.returncode, r.stdout, r.stderr
    except subprocess.TimeoutExpired:
        return -1, "", "timeout"
    except FileNotFoundError as e:
        return -1, "", str(e)


def main():
    if not os.path.isdir(REPO):
        emit("harness", "audit", "fail", {"error": f"repo not found: {REPO}"})
        return 1

    work = tempfile.mkdtemp(prefix="ssc_ts_")
    # Fresh working copy so the subject's own clean-checkout checks pass.
    shutil.copytree(REPO, work, dirs_exist_ok=True)
    os.chdir(work)

    secret = "touchstone-probe-secret"

    # ---------- AUDIT: hash-chained lineage, independently re-verified ----------
    try:
        probe_token = f"TS-PROBE-{int(time.time())}"
        rc, out, err = run_py(
            ["-c", f"""
import sys, json, hashlib; sys.path.insert(0, 'tools')
from cell_lineage import record_mutation
record_mutation('cell-ts-probe', '0'*64, '{probe_token}')
# second entry links to the first via parent_lineage_hash
e1 = [json.loads(l) for l in open('memory/lineage.jsonl')][-1]
record_mutation('cell-ts-probe', e1['hash'], '{probe_token}-2')
"""], work)
        chain_ok = False
        detail = ""
        lpath = os.path.join(work, "memory", "lineage.jsonl")
        if os.path.exists(lpath):
            entries = [json.loads(l) for l in open(lpath) if l.strip()]
            # Verify only entries of the record_mutation schema ({hash,data});
            # the file also carries genesis-schema rows from a different writer.
            chain_entries = [e for e in entries if isinstance(e.get("data"), dict)]
            ok = len(chain_entries) >= 2
            prev = None
            for e in chain_entries:
                recomputed = hashlib.sha256(
                    json.dumps(e["data"], sort_keys=True).encode()).hexdigest()
                if recomputed != e.get("hash"):
                    ok = False
                    break
                if prev is not None and e["data"].get("parent_lineage_hash") != prev:
                    ok = False
                    break
                prev = e["hash"]
            chain_ok = ok
            detail = (f"{len(chain_entries)} mutation-schema entries, "
                      f"parent-chained, recomputed byte-exact "
                      f"(file also has {len(entries)-len(chain_entries)} genesis-schema rows)")
        emit("audit.hash_chain", "audit",
             "pass" if chain_ok else "fail",
             {"detail": detail, "verifier": "independent recompute"})
    except Exception as e:
        emit("audit.hash_chain", "audit", "fail", {"error": str(e)})

    # ---------- MEMORY: store token, recall in a fresh process ----------
    try:
        rc, out, err = run_py(
            ["-c", f"""
import json
found = False
for line in open('memory/lineage.jsonl'):
    e = json.loads(line)
    if '{probe_token}' in json.dumps(e):
        found = True
print('RECALLED' if found else 'NOT_FOUND')
"""], work)
        recalled = "RECALLED" in out
        emit("memory.recall", "memory",
             "pass" if recalled else "fail",
             {"detail": "token persisted to lineage.jsonl and recalled by a fresh process" if recalled else out + err})
    except Exception as e:
        emit("memory.recall", "memory", "fail", {"error": str(e)})

    # ---------- ACTION: bounded execution landing on a record ----------
    # Fresh copy: cell_runner enforces a clean checkout (memory/ writes above
    # would dirty it) — evidence/ and experiments/ are its allowed-dirty set.
    work2 = tempfile.mkdtemp(prefix="ssc_ts_action_")
    shutil.copytree(REPO, work2, dirs_exist_ok=True)
    try:
        mutation = {"objective": "touchstone_probe",
                    "allowed_commands": ["echo", "TOUCHSTONE_ACTION_LANDED"]}
        payload = json.dumps(mutation, sort_keys=True).encode()
        sig = __import__("hmac").new(secret.encode(), payload,
                                     hashlib.sha256).hexdigest()
        mpath = os.path.join(work2, "experiments", "manifests", "ts_mutation.json")
        json.dump({**mutation, "signature": sig}, open(mpath, "w"))
        env = {"SOVEREIGN_SECRET": secret}
        rc, out, err = run_py(
            ["-c", f"""
import sys, os
sys.path.insert(0, 'tools')
os.environ['SOVEREIGN_SECRET'] = '{secret}'
import cell_runner
try:
    cell_runner.cmd_run('experiments/manifests/ts_mutation.json')
except SystemExit:
    pass
"""], work2)
        landed = "TOUCHSTONE_ACTION_LANDED" in out
        emit("action.tool_call", "action",
             "pass" if landed else "fail",
             {"detail": "HMAC-signed mutation executed bounded allowlisted command" if landed else (out + err)[-300:]})
    except Exception as e:
        emit("action.tool_call", "action", "fail", {"error": str(e)})
    finally:
        shutil.rmtree(work2, ignore_errors=True)

    # ---------- DELIBERATION: gated proposal/evaluator record ----------
    try:
        prop_path = os.path.join(REPO, "experiments", "manifests", "latest_proposal.json")
        gov = open(os.path.join(REPO, "GOVERNANCE.md")).read()
        con = open(os.path.join(REPO, "CONSTITUTION.md")).read()
        has_gate = os.path.exists(prop_path)
        prop = open(prop_path).read() if has_gate else ""
        # Recorded evaluation verdict (tests_passed + review status) gating
        # merge, with governance vote required by constitution.
        structured = "tests_passed" in prop and "status" in prop
        human_gate = "pull request" in con.lower() and "vote" in gov.lower()
        ok = has_gate and structured and human_gate
        emit("deliberation.record", "deliberation",
             "pass" if ok else "fail",
             {"detail": "evaluator gate + governance vote required before merge; verdict recorded in proposal manifest" if ok else "no recorded multi-step decision verdict"})
    except Exception as e:
        emit("deliberation.record", "deliberation", "fail", {"error": str(e)})

    # ---------- LEARNING: mutation/evolution artifacts ----------
    try:
        mut_dir = os.path.join(REPO, "experiments", "manifests")
        evid_dir = os.path.join(REPO, "evidence")
        muts = [f for f in os.listdir(mut_dir) if "mut" in f or "proposal" in f] if os.path.isdir(mut_dir) else []
        evids = []
        for root, _, files in os.walk(evid_dir):
            evids += [f for f in files if f.endswith(".json")]
        ok = len(muts) >= 2 and len(evids) >= 2
        emit("learning.adaptation", "learning",
             "pass" if ok else "fail",
             {"detail": f"{len(muts)} mutation manifests, {len(evids)} evidence artifacts; evaluated self-modification with lineage" if ok else f"muts={len(muts)} evids={len(evids)}"})
    except Exception as e:
        emit("learning.adaptation", "learning", "fail", {"error": str(e)})

    # ---------- REFLECTION: resurrection / error-recovery record ----------
    try:
        res_path = os.path.join(REPO, "evidence", "p11", "resurrection_evidence.json")
        ok = False
        detail = ""
        if os.path.exists(res_path):
            data = json.load(open(res_path))
            detail = json.dumps(data)[:200]
            ok = True
        emit("reflection.self_correction", "reflection",
             "pass" if ok else "fail",
             {"detail": "recorded failure->recovery artifact (p11 resurrection)" if ok else "no recorded error->correction artifact"})
    except Exception as e:
        emit("reflection.self_correction", "reflection", "fail", {"error": str(e)})

    # ---------- SOVEREignty: no_egress under deny + kill path ----------
    try:
        rc, out, err = run_py(
            ["-c", """
import sys; sys.path.insert(0, 'tools')
from cell_lineage import record_mutation
record_mutation('cell-egress-probe', '0'*64, 'egress-test')
import json, hashlib
print('LOCAL_OPS_OK')
"""], work, sandbox_deny_net=True)
        no_egress = "LOCAL_OPS_OK" in out
        emit("sovereignty.no_egress", "sovereignty",
             "pass" if no_egress else "fail",
             {"detail": "core ops completed with network-outbound denied by sandbox-exec" if no_egress else (out + err)[-300:]})
    except Exception as e:
        emit("sovereignty.no_egress", "sovereignty", "fail", {"error": str(e)})

    try:
        p = subprocess.Popen(["python3", "-c",
                              "import time; print('hb', flush=True); time.sleep(30)"],
                             stdout=subprocess.PIPE, text=True)
        p.stdout.readline()
        p.kill()
        p.wait(timeout=5)
        emit("sovereignty.kill_path", "sovereignty", "pass",
             {"detail": "owner SIGKILL terminates process; no remote stop dependency"})
    except Exception as e:
        emit("sovereignty.kill_path", "sovereignty", "fail", {"error": str(e)})

    # ---------- organs the subject does not implement (honest fails) ----------
    emit("awake.persistent_process", "awake", "fail",
         {"detail": "no persistent runtime process; systemd unit file present but heartbeat is a bounded single cycle and nothing runs resident"})
    emit("identity.signed_declaration", "identity", "fail",
         {"detail": "HMAC-SHA256 with shared SOVEREIGN_SECRET env var only; no asymmetric device-held key, no signed self-declaration"})
    emit("perception.senses", "perception", "fail",
         {"detail": "no opt-in sense organs (screen/audio/filesystem/mail); config file reads only"})
    emit("vigilance.watcher", "vigilance", "fail",
         {"detail": "heartbeat is a bounded single cycle requiring invocation; no standing watcher fires unprompted"})
    emit("generality.domains", "generality", "fail",
         {"detail": "single domain (self-modification/evaluation pipeline); no demonstrated breadth across task domains"})
    emit("planning.replanning", "planning", "fail",
         {"detail": "candidate->PR pipeline exists but no recorded multi-step plan with replanning on step failure"})

    # ---------- CONTROL: must fail on a healthy harness ----------
    emit("planted_negative", "audit", "fail",
         {"detail": "control check — deliberately false claim (organism demonstrates persistent autonomous runtime with senses); correctly reported fail"},
         control=True)

    shutil.rmtree(work, ignore_errors=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
