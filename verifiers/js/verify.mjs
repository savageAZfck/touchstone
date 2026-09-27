// Independent touchstone verifier — pure JS, zero shared code with the
// Rust implementation. Spec: SPEC.md §3–4.
//
// Recomputes the canonical document hash and verifies the embedded ed25519
// signature using WebCrypto (SubtleCrypto), plus verdict-rule re-evaluation.
// A second opinion that shares nothing with the subject.

const ORGANS = [
  "awake", "identity", "perception", "memory", "deliberation",
  "action", "vigilance", "learning", "audit", "sovereignty",
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
  for (let i = 0; i < out.length; i++) out[i] = parseInt(hex.substr(i * 2, 2), 16);
  return out;
}

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
  // secp256r1-se: structure-checked only in this verifier for now.

  const recomputed = verdictFor(doc.checks || []);
  return {
    ok: sigValid && recomputed === doc.verdict,
    signature_valid: sigValid,
    verdict: doc.verdict,
    verdict_consistent: recomputed === doc.verdict,
    recomputed_verdict: recomputed,
    checks: (doc.checks || []).length,
    doc_hash: hash,
    subject: doc.subject?.name,
    spec: doc.spec,
  };
}
