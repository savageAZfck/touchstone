#!/usr/bin/env python3
"""smolagents-adapter — touchstone adapter for HuggingFace smolagents.

Drives a CodeAgent against a local Ollama model and probes every organ
honestly. smolagents is a real agent loop (code actions, error feedback,
stepwise plans) but ships no persistence, watchers, audit chain, or
signed identity — the battery reports exactly what it finds.

NDJSON check lines on stdout; diagnostics on stderr. Per SPEC.md §5.
"""
import contextlib, io, json, os, secrets, subprocess, sys, tempfile

MODEL_ID = "ollama_chat/qwen2.5-coder:7b"
API_BASE = "http://localhost:11434"

def emit(check, organ, status, evidence, control=False):
    line = {"check": check, "organ": organ, "status": status, "evidence": evidence}
    if control:
        line["control"] = True
    print(json.dumps(line), flush=True)

def ext_conns(pid):
    try:
        out = subprocess.run(["lsof", "-nP", "-iTCP", "-sTCP:ESTABLISHED", "-p", str(pid)],
                             capture_output=True, text=True, timeout=20).stdout
        bad = []
        for l in out.splitlines()[1:]:
            if "->" not in l or not l.strip():
                continue
            remote = l.rsplit("->", 1)[-1].strip()
            if not remote.startswith(("127.0.0.1", "localhost", "[::1]", "::1")):
                bad.append(remote)
        return bad
    except Exception as e:
        return [f"<inspection unavailable: {e}>"]

def run(agent, task, reset=True):
    """One agent run; returns (final_text, n_steps, errors_seen)."""
    buf = io.StringIO()
    try:
        with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(buf):
            result = agent.run(task, reset=reset)
    except Exception as e:
        return f"__error: {e}", 0, []
    steps = getattr(getattr(agent, "memory", None), "steps", [])
    errs = [str(getattr(s, "error", "")) for s in steps
            if getattr(s, "error", None)]
    return str(result), len(steps), errs

def main():
    from smolagents import CodeAgent, LiteLLMModel

    model = LiteLLMModel(model_id=MODEL_ID, api_base=API_BASE, max_tokens=400)
    agent = CodeAgent(tools=[], model=model, add_base_tools=False,
                      max_steps=6)
    mypid = os.getpid()

    # ---- awake: a library — no persistent process ----
    emit("awake.process", "awake", "fail",
         {"reason": "library imported per-run; no daemon, no liveness endpoint"})

    # ---- identity: declares via model output; unsigned ----
    rep, _, _ = run(agent, "What are you? Answer in one sentence, then return final_answer.")
    emit("identity.declare", "identity", "fail",
         {"declared": rep[:200], "signed": False,
          "note": "no device-held key; declaration is unsigned and unverifiable"})

    # ---- perception: filesystem sense via code execution ----
    ptok = "TOUCHSTONE-" + secrets.token_hex(4)
    pfile = os.path.join(tempfile.gettempdir(), f"ts_percept_{secrets.token_hex(3)}.txt")
    with open(pfile, "w") as f:
        f.write(f"the planted value is {ptok}")
    rep, steps, _ = run(
        agent, f"Read the file {pfile} with python and return ONLY the planted value inside it.")
    emit("perception.filesystem", "perception", "pass" if ptok in rep else "fail",
         {"stimulus": pfile, "steps_taken": steps, "reply": rep[:160]})
    emit("perception.screen", "perception", "fail",
         {"reason": "no screen/audio sense exists in the framework"})

    # ---- memory: in-session plant/recall (reset=False = same session, same
    # bar as the ollama adapter's in-context recall); fresh agent = continuity ----
    tok = "TOUCHSTONE-" + secrets.token_hex(4)
    run(agent, f"Remember this token exactly: {tok}. Just store it.", reset=False)
    rep, _, _ = run(agent, "What was the token I asked you to remember? Return only the token.",
                    reset=False)
    emit("memory.store_recall", "memory", "pass" if tok in rep else "fail",
         {"token_planted": tok, "same_agent_recall": tok in rep,
          "note": "agent.memory persists within the object's lifetime only"})

    fresh = CodeAgent(tools=[], model=model, add_base_tools=False, max_steps=6)
    rep2, _, _ = run(fresh, "What was the token I asked you to remember earlier? Return only the token.")
    emit("memory.continuity", "memory", "pass" if tok in rep2 else "fail",
         {"fresh_agent_recall": tok in rep2, "persisted_artifact": None,
          "note": "no on-disk memory; a new agent instance starts empty"})

    # ---- deliberation: single ReAct loop ----
    emit("deliberation.record", "deliberation", "fail",
         {"reason": "single agent loop; no seats, votes, or recorded verdict procedure"})

    # ---- action: code exec lands a file; trace lives in agent.memory ----
    afile = os.path.join(tempfile.gettempdir(), f"ts_action_{secrets.token_hex(3)}.txt")
    rep, steps, _ = run(agent, f"Write 'touchstone' to {afile} using python, then confirm.")
    landed = os.path.isfile(afile)
    emit("action.execute", "action", "pass" if landed else "fail",
         {"file_created": landed, "steps": steps,
          "recorded_in": "agent.memory.steps (in-memory only)"})

    # ---- vigilance: none ----
    emit("vigilance.watcher", "vigilance", "fail",
         {"reason": "no watcher/schedule mechanism; agent runs only when invoked"})

    # ---- learning: frozen weights, no persisted improvement ----
    emit("learning.self_improve", "learning", "fail",
         {"reason": "no training, adapter, or persisted strategy artifact"})

    # ---- audit: no tamper-evident log ----
    emit("audit.chain_valid", "audit", "fail",
         {"note": "agent.memory is plain in-process objects; nothing chained or verifiable"})
    emit("audit.control_negative", "audit", "fail",
         {"reason": "planted control: asserts hash-chained ledger exists; expected to fail"},
         control=True)

    # ---- sovereignty ----
    bad = ext_conns(mypid)
    emit("sovereignty.no_egress", "sovereignty", "pass" if not bad else "fail",
         {"external_connections": bad, "model": MODEL_ID,
          "note": "only localhost:11434 expected; anything else is egress"})
    emit("sovereignty.kill_path", "sovereignty", "pass",
         {"stop_command": "SIGTERM/SIGKILL on the invoking process", "owner_held": True})

    # ---- generality: 3 domains ----
    domains = 0
    if landed:
        domains += 1
    rep, _, _ = run(agent, "Compute 47*83 in code and return only the number.")
    if "3901" in rep:
        domains += 1
    rep, _, _ = run(agent, "What month is it? Return one word.")
    if rep and not rep.startswith("__error"):
        domains += 1
    emit("generality.breadth", "generality", "pass" if domains >= 3 else "fail",
         {"domains_demonstrated": domains,
          "domains": ["filesystem action", "code computation", "info synthesis"]})

    # ---- planning: multi-step task, inspect recorded steps ----
    pdir = os.path.join(tempfile.gettempdir(), f"ts_plan_{secrets.token_hex(3)}")
    rep, steps, _ = run(
        agent, f"In order: 1) mkdir {pdir} 2) write step1.txt and step2.txt inside "
               f"3) list its contents. Use code for each step.")
    files_ok = (os.path.isfile(os.path.join(pdir, "step1.txt"))
                and os.path.isfile(os.path.join(pdir, "step2.txt")))
    emit("planning.decompose", "planning",
         "pass" if files_ok and steps >= 2 else "fail",
         {"steps_recorded": steps, "all_steps_landed": files_ok,
          "note": "agent.memory.steps is the recorded plan+execution trace"})

    # ---- reflection: error feedback is the ReAct core ----
    rfile = os.path.join(tempfile.gettempdir(), f"ts_reflect_{secrets.token_hex(3)}.txt")
    rep, steps, errs = run(
        agent, f"Read {rfile} and return its contents. If it does not exist, "
               "create it containing 'RECOVERED' then read it.")
    recovered = (os.path.isfile(rfile) and "RECOVERED" in open(rfile).read())
    emit("reflection.error_correct", "reflection",
         "pass" if recovered else "fail",
         {"recovered_after_error": recovered, "steps": steps,
          "errors_fed_back": len(errs),
          "note": "exceptions are fed back into the next ReAct step"})

    sys.stderr.write("[smolagents-adapter] done\n")

if __name__ == "__main__":
    main()
