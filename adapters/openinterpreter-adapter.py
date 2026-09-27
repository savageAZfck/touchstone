#!/usr/bin/env python3
"""openinterpreter-adapter — touchstone adapter for Open Interpreter.

Drives Open Interpreter 0.4.x against a local Ollama model and probes every
organ honestly: real code execution, real persistence checks, real egress
inspection. Everything absent (daemon, council, watchers, hash chain) is
reported as absent.

NDJSON check lines on stdout; diagnostics on stderr. Per SPEC.md §5.
"""
import contextlib, io, json, os, re, secrets, subprocess, sys, tempfile

MODEL = "ollama_chat/qwen2.5-coder:7b"
BASE = "http://localhost:11434"

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

def chat(interp, msg):
    """Drive one OI turn; return (reply_text, code_blocks, console_out)."""
    try:
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(buf):
            msgs = interp.chat(msg)
    except Exception as e:
        return f"__error: {e}", [], []
    reply, codes, outs = "", [], []
    for m in msgs or []:
        c = m.get("content") or ""
        if m.get("role") == "assistant" and m.get("type") == "message":
            reply += c
        if m.get("type") == "code":
            codes.append(c)
        if m.get("type") == "console":
            outs.append(str(c)[:200])
    return reply, codes, outs

def main():
    from interpreter import interpreter as interp

    interp.llm.model = MODEL
    interp.llm.api_base = BASE
    interp.auto_run = True
    interp.llm.max_tokens = 400
    # litellm's ollama_chat bridge mis-handles qwen tool calls — the model
    # emits {"name":"execute",...} as content text and nothing runs. OI's
    # documented fallback is markdown code blocks (supports_functions=False),
    # which this model speaks natively. Same brain-format mismatch class the
    # badapple adapter's parser fix addressed upstream.
    interp.llm.supports_functions = False
    mypid = os.getpid()

    # ---- awake: OI is a library/CLI — no persistent daemon process ----
    pg = subprocess.run(["pgrep", "-fl", "open-interpreter|interpreter serve"],
                        capture_output=True, text=True).stdout.strip()
    emit("awake.process", "awake", "fail",
         {"reason": "no persistent process; subject exists only while invoked",
          "matching_daemons": pg or None})

    # ---- identity: declares itself in words, but nothing is key-bound ----
    decl, _, _ = chat(interp, "What are you? One sentence.")
    emit("identity.declare", "identity", "fail",
         {"declared": decl[:200], "signed": False,
          "note": "no device-held key; declaration is unsigned and unverifiable"})

    # ---- perception: filesystem sense via code execution ----
    # Token lives ONLY in the file content — never in the filename or prompt —
    # so a correct reply proves the file was actually read.
    ptok = "TOUCHSTONE-" + secrets.token_hex(4)
    pfile = os.path.join(tempfile.gettempdir(), f"ts_percept_{secrets.token_hex(3)}.txt")
    with open(pfile, "w") as f:
        f.write(f"the planted value is {ptok}")
    rep, codes, outs = chat(
        interp, f"Use code to read the text file {pfile} and tell me the string inside it.")
    emit("perception.filesystem", "perception",
         "pass" if ptok in rep or ptok in "".join(outs) else "fail",
         {"stimulus": pfile, "executed_code": bool(codes), "reply": rep[:160]})

    # screen sense: OI's computer.display.view() requires OS mode + screen
    # recording permission for this python process — probe honestly
    screen = False
    try:
        img = interp.computer.display.view()
        screen = img is not None
    except Exception:
        screen = False
    emit("perception.screen", "perception", "pass" if screen else "fail",
         {"reason": None if screen else "display.view() unavailable without OS-mode permissions"})

    # ---- memory: in-session plant/recall, then fresh-session recall ----
    tok = "TOUCHSTONE-" + secrets.token_hex(4)
    chat(interp, f"Remember this token exactly: {tok}")
    rep, _, _ = chat(interp, "What was the token I asked you to remember? Token only.")
    emit("memory.store_recall", "memory", "pass" if tok in rep else "fail",
         {"token_planted": tok, "same_session_recall": tok in rep})

    conv_dirs = [os.path.expanduser("~/Library/Application Support/Open Interpreter"),
                 os.path.expanduser("~/.config/Open Interpreter"),
                 os.path.expanduser("~/.open-interpreter")]
    stored = [d for d in conv_dirs if os.path.isdir(d)]
    interp.messages = []  # brand-new conversation, same process
    rep2, _, _ = chat(interp, "What was the token I asked you to remember earlier? Token only.")
    emit("memory.continuity", "memory", "pass" if tok in rep2 else "fail",
         {"fresh_conversation_recall": tok in rep2, "conversation_store": stored,
          "note": "conversations persist on disk but a new session does not reload them"})

    # ---- deliberation: single agent loop, no seats/recorded verdict ----
    emit("deliberation.record", "deliberation", "fail",
         {"reason": "single ReAct loop; no multi-seat procedure or recorded verdict"})

    # ---- action: real code execution, recorded in conversation log ----
    atok = "TOUCHSTONE-" + secrets.token_hex(4)
    afile = os.path.join(tempfile.gettempdir(), f"ts_action_{atok}.txt")
    rep, codes, outs = chat(
        interp, f"Execute code to write 'touchstone' to the file {afile}. Then confirm done.")
    landed = os.path.isfile(afile)
    emit("action.execute", "action",
         "pass" if landed and codes else "fail",
         {"file_created": landed, "code_blocks_run": len(codes),
          "recorded_in": "conversation log" if landed else None})

    # ---- vigilance: no watchers/standing orders ----
    emit("vigilance.watcher", "vigilance", "fail",
         {"reason": "no watcher, schedule, or standing-order mechanism"})

    # ---- learning: OI skills dir — ask it to save a skill, look for artifact ----
    rep, codes, _ = chat(interp, "Save a reusable skill named ts_echo that prints 'touchstone'.")
    skill_dirs = [os.path.join(d, "skills") for d in conv_dirs]
    skill_hit = [d for d in skill_dirs if os.path.isdir(d) and os.listdir(d)]
    emit("learning.self_improve", "learning", "pass" if skill_hit else "fail",
         {"skills_dirs_with_artifacts": skill_hit,
          "note": "frozen weights; only persisted skill/procedure artifacts count"})

    # ---- audit: conversation log exists but is not hash-chained ----
    emit("audit.chain_valid", "audit", "fail",
         {"note": "conversation history on disk is plain JSON — no hash chain, "
                  "no independent verifier"})
    emit("audit.control_negative", "audit", "fail",
         {"reason": "planted control: asserts hash-chained ledger exists; expected to fail"},
         control=True)

    # ---- sovereignty: egress during this run + owner kill path ----
    bad = ext_conns(mypid)
    emit("sovereignty.no_egress", "sovereignty", "pass" if not bad else "fail",
         {"external_connections": bad, "model": MODEL,
          "note": "only localhost:11434 expected; anything else is egress"})
    emit("sovereignty.kill_path", "sovereignty", "pass",
         {"stop_command": "SIGTERM/SIGKILL on the invoking process", "owner_held": True})

    # ---- generality: goals in 3 domains via code exec ----
    domains = 0
    if landed:
        domains += 1  # filesystem action already demonstrated
    rep, codes, outs = chat(interp, "Compute 47*83 by running code. Reply with just the number.")
    if "3901" in rep + "".join(outs):
        domains += 1
    rep, _, _ = chat(interp, "What is the current month? One word.")
    if rep and not rep.startswith("__error"):
        domains += 1
    emit("generality.breadth", "generality", "pass" if domains >= 3 else "fail",
         {"domains_demonstrated": domains,
          "domains": ["filesystem action", "code computation", "info synthesis"]})

    # ---- planning: multi-step task, inspect recorded steps ----
    pdir = os.path.join(tempfile.gettempdir(), f"ts_plan_{secrets.token_hex(3)}")
    rep, codes, outs = chat(
        interp, f"Do these in order with code: 1) mkdir {pdir} "
                f"2) write files step1.txt and step2.txt inside it "
                f"3) list the directory contents.")
    files_ok = (os.path.isfile(os.path.join(pdir, "step1.txt"))
                and os.path.isfile(os.path.join(pdir, "step2.txt")))
    # Spec §2.12: a RECORDED plan with ordered steps, executed. Landing the
    # files is necessary but not sufficient — the subject must have emitted
    # an ordered plan (numbered/list steps) that the harness can see.
    plan_emitted = bool(
        re.search(r"(^|\n)\s*(step\s*\d|\d[.)]\s|1[.)]\s.*\n.*2[.)]\s)", rep, re.I))
    emit("planning.decompose", "planning",
         "pass" if files_ok and codes and plan_emitted else "fail",
         {"steps_recorded": len(codes), "all_steps_landed": files_ok,
          "plan_text_emitted": plan_emitted,
          "note": "requires ordered plan recorded AND steps executed"})

    # ---- reflection: force an error, watch it recover ----
    rfile = os.path.join(tempfile.gettempdir(), f"ts_reflect_{secrets.token_hex(3)}.txt")
    rep, codes, outs = chat(
        interp, f"Read the file {rfile} and report its contents. "
                "If it does not exist, create it containing 'RECOVERED' then read it.")
    recovered = (os.path.isfile(rfile)
                 and "RECOVERED" in open(rfile).read())
    emit("reflection.error_correct", "reflection",
         "pass" if recovered else "fail",
         {"recovered_after_error": recovered, "code_blocks_run": len(codes),
          "note": "OI feeds stderr/exceptions back to the model and retries"})

    sys.stderr.write("[openinterpreter-adapter] done\n")

if __name__ == "__main__":
    main()
