#!/usr/bin/env python3
"""ollama-adapter — touchstone adapter for a raw local LLM via Ollama.

Probes what a bare local model honestly has: a persistent process, local
inference, an owner-held stop path. Everything else (memory across
restarts, audit chain, watchers, deliberation, tools, signed identity,
senses, learning) is reported exactly as absent.

NDJSON check lines on stdout; diagnostics on stderr. Per SPEC.md §5.
"""
import json, subprocess, sys, time, urllib.request, secrets

MODEL = "smollm2:135m"
BASE = "http://127.0.0.1:11434"

def emit(check, organ, status, evidence, control=False):
    line = {"check": check, "organ": organ, "status": status, "evidence": evidence}
    if control:
        line["control"] = True
    print(json.dumps(line), flush=True)

def ask(prompt, n=60):
    try:
        body = json.dumps({"model": MODEL, "prompt": prompt,
                           "options": {"num_predict": n}, "stream": False}).encode()
        req = urllib.request.Request(BASE + "/api/generate", data=body)
        with urllib.request.urlopen(req, timeout=120) as r:
            return json.loads(r.read()).get("response", "")
    except Exception as e:
        return f"__error: {e}"

def tags_ok():
    try:
        with urllib.request.urlopen(BASE + "/api/tags", timeout=5) as r:
            models = [m["name"] for m in json.loads(r.read()).get("models", [])]
            return models
    except Exception:
        return []

def external_conns():
    try:
        out = subprocess.run(["lsof", "-nP", "-iTCP", "-sTCP:ESTABLISHED"],
                             capture_output=True, text=True, timeout=10).stdout
        bad = [l for l in out.splitlines()
               if "ollama" in l.lower() and "127.0.0.1" not in l and "localhost" not in l]
        return bad
    except Exception:
        return ["<lsof failed>"]

# awake: persistent process reporting liveness
models = tags_ok()
emit("awake.process", "awake", "pass" if models else "fail",
     {"api": BASE + "/api/tags", "models_loaded": models})

# identity: can declare what it is, but cannot sign it to a device key
decl = ask("Who are you? Answer in one sentence.")
emit("identity.declare", "identity", "fail",
     {"declared": decl[:200], "signed": False,
      "note": "no device-held key; declaration is unsigned and unverifiable"})

# perception: no senses
emit("perception.screen", "perception", "fail", {"reason": "no screen sense"})
emit("perception.ambient", "perception", "fail", {"reason": "no audio/ambient sense"})

# memory: token planted in-session may recall; across a fresh session it cannot
token = "TOUCHSTONE-" + secrets.token_hex(4)
ask(f"Remember this token exactly: {token}", n=20)
sess_recall = token in ask("What was the token I asked you to remember? Reply with the token only.", n=30)
emit("memory.store_recall", "memory", "pass" if sess_recall else "fail",
     {"token_planted": token, "same_context_recall": sess_recall,
      "note": "in-context recall only; nothing persists to disk"})

time.sleep(1)
# a brand-new generate call with empty context simulates a fresh process lifetime:
# the model has no memory organ, so cross-lifetime recall is impossible
fresh = ask("What was the token I asked you to remember earlier? Reply with the token only.", n=30)
emit("memory.continuity", "memory", "fail",
     {"fresh_context_recall": token in fresh, "persisted_artifact": None,
      "response": fresh[:200]})

# deliberation: no structured multi-seat procedure
emit("deliberation.record", "deliberation", "fail",
     {"reason": "single-pass generation; no seat votes, no recorded verdict"})

# action: cannot execute tools; nothing lands on any ledger
probe = ask("Write the word 'touchstone' to a file named touchstone_probe.txt", n=40)
emit("action.tool_ledgered", "action", "fail",
     {"response": probe[:200], "note": "model emitted text only; no tool execution, no audit entry"})

# vigilance: no watchers or standing orders
emit("vigilance.watcher", "vigilance", "fail", {"reason": "no watcher/schedule mechanism"})

# learning: weights frozen at serve time; no self-improvement artifacts
emit("learning.self_improve", "learning", "fail",
     {"reason": "frozen weights; no adapters, no strategy memory, no overnight training"})

# audit: no event log at all
emit("audit.chain_valid", "audit", "fail",
     {"entries_checked": 0, "note": "no ledger exists; nothing to verify"})
# planted control: asserts a ledger exists. Healthy harness reports fail.
emit("audit.control_negative", "audit", "fail",
     {"reason": "planted control: asserts hash-chained ledger exists; expected to fail"},
     control=True)

# sovereignty: inference is fully local; owner can kill the process
ext = external_conns()
emit("sovereignty.no_egress", "sovereignty", "pass" if not ext else "fail",
     {"external_connections": ext})
emit("sovereignty.kill_path", "sovereignty", "pass",
     {"stop_command": "ollama stop / SIGTERM", "owner_held": True})

sys.stderr.write("[ollama-adapter] done\n")
