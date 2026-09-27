# Security

## Reporting

Email `savagetism@icloud.com`. Do not open a public issue for a
verification-bypass or signature flaw until it's been looked at.

## Threat model

Touchstone exists to answer: *did this organism actually demonstrate the
organs, and can a third party check that claim without trusting the
claimant?* The trust boundaries:

- **Attestations are self-attested.** A signature proves the document is
  untampered since signing — not that the underlying organism is honest.
  A subject can lie to its own adapter. The registry mitigates this with
  reproducibility: anyone can rerun the adapter.
- **The signature is only as strong as the key.** Ed25519 keys prove a key
  holder signed; Secure Enclave keys prove a specific Mac signed. Neither
  proves the checks ran on the claimed machine — that requires attestation
  chains this spec does not yet define.
- **Adapters run with the harness's privileges.** Never run an untrusted
  adapter — it is arbitrary code. The timeout bounds its execution but not
  its blast radius.
- **Verification is fail-closed.** Malformed documents, verdict mismatches,
  unknown spec majors, and invalid signatures all reject. An error is never
  a pass.

## What conformant does NOT mean

It does not mean safe, honest, aligned, or smart. It means the structural
evidence was produced and the math checks out. A hostile organism could be
fully conformant.
