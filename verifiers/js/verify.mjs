// Independent touchstone verifier — pure JS, zero shared code with the
// Rust implementation. Spec: SPEC.md §3–4.
//
// Full §4: structural validation (spec tag, unique ids, timestamp, sig
// shape), verdict recomputation, canonical hash, signature verification.
// Ed25519 via WebCrypto. A second opinion that shares nothing with the
// subject.
//
// CLI: node verify.mjs <attestation.json>  — prints verdict, exit 0/1.

const ORGANS = [
  "awake", "identity", "perception", "memory", "deliberation",
  "action", "vigilance", "learning", "audit", "sovereignty",
  "generality", "planning", "reflection",
];

// canonical JSON: recursively sort object keys, compact separators
function canon(v) {
  if (v === null || typeof v !== "object") return JSON.stringify(v);
  if (Array.isArray(v)) return "[" + v.map(canon).join(",") + "]";
  const keys = Object.keys(v).sort();
  return "{" + keys.map(k => JSON.stringify(k) + ":" + canon(v[k])).join(",") + "}";
}

async function sha256Hex(str) {
  const buf = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(str));
  return [...new Uint8Array(buf)].map(b => b.toString(16).padStart(2, "0")).join("");
}

function hexToBytes(hex) {
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  return out;
}

const isHex = (s, n) => typeof s === "string" && s.length === n && /^[0-9a-f]+$/i.test(s);

// Spec §4 step 2: structural validation, independent reimplementation.
function structuralErrors(doc) {
  const errs = [];
  if (typeof doc.spec !== "string" || !doc.spec.startsWith("touchstone/") || doc.spec.length <= "touchstone/".length)
    errs.push("spec field missing or not touchstone/<version>");
  if (!Array.isArray(doc.checks) || doc.checks.length === 0)
    errs.push("attestation has no checks");
  const seen = new Set();
  for (const c of doc.checks || []) {
    if (seen.has(c.id)) errs.push("duplicate check id: " + c.id);
    seen.add(c.id);
  }
  if (typeof doc.timestamp !== "string" || isNaN(Date.parse(doc.timestamp)))
    errs.push("timestamp missing or not RFC3339-parseable");
  if (doc.signature != null) {
    if (!isHex(doc.signature.pubkey, 64) || !isHex(doc.signature.sig, 128))
      errs.push("signature fields malformed");
  }
  return errs;
}

// Spec §1.1: verdict is a pure function of check results.
function verdictFor(checks) {
  if (!checks.length) return "nonconformant";
  for (const c of checks) {
    if (c.control && c.status === "pass") return "nonconformant";
  }
  const passed = new Set();
  let anyFail = false;
  for (const c of checks) {
    if (c.status === "pass") passed.add(c.organ);
    if (c.status === "fail" && !c.control) anyFail = true;
  }
  const allOrgans = ORGANS.every(o => passed.has(o));
  return allOrgans && !anyFail ? "conformant" : "partial";
}

export async function verifyAttestation(jsonText) {
  let doc;
  try { doc = JSON.parse(jsonText); }
  catch (e) { return { ok: false, error: "parse: " + e.message }; }

  // §4.2 structural validation — fail closed before touching signatures
  const errs = structuralErrors(doc);
  if (errs.length) return { ok: false, error: "invalid: " + errs.join("; "), invalid: errs };

  // §4.3 verdict recomputation — a valid sig over a false verdict still fails
  const recomputed = verdictFor(doc.checks);
  const verdictConsistent = recomputed === doc.verdict;

  // §4.4 canonical hash + signature
  const sig = doc.signature;
  delete doc.signature;
  const canonical = canon(doc);
  const hash = await sha256Hex(canonical);

  let sigValid = false;
  if (sig && sig.scheme === "ed25519") {
    try {
      const key = await crypto.subtle.importKey(
        "raw", hexToBytes(sig.pubkey), { name: "Ed25519" }, false, ["verify"]
      );
      sigValid = await crypto.subtle.verify(
        { name: "Ed25519" }, key, hexToBytes(sig.sig), hexToBytes(hash)
      );
    } catch { sigValid = false; }
  }
  // secp256r1-se: WebCrypto can verify ECDSA P-256 — SEC1 raw point needs
  // wrapping; left as structure-checked until a caller needs it.

  const ok = sigValid && verdictConsistent;
  return {
    ok,
    signature_valid: sigValid,
    verdict: doc.verdict,
    verdict_consistent: verdictConsistent,
    recomputed_verdict: recomputed,
    checks: doc.checks.length,
    doc_hash: hash,
    subject: doc.subject?.name,
    spec: doc.spec,
  };
}

// CLI mode: node verify.mjs <file>
const invokedDirectly = process.argv[1] &&
  import.meta.url === (await import("node:url")).pathToFileURL(process.argv[1]).href;
if (invokedDirectly) {
  const fs = await import("node:fs");
  const file = process.argv[2];
  if (!file) {
    console.error("usage: node verify.mjs <attestation.json>");
    process.exit(2);
  }
  const r = await verifyAttestation(fs.readFileSync(file, "utf8"));
  if (r.error) console.error(r.error);
  console.log(`signature: ${r.signature_valid ? "VALID" : "INVALID"}`);
  if (r.verdict !== undefined) {
    console.log(`verdict:   ${r.verdict} (recomputed ${r.recomputed_verdict}, consistent=${r.verdict_consistent})`);
    console.log(`doc hash:  ${r.doc_hash}`);
  }
  process.exit(r.ok ? 0 : 1);
}
