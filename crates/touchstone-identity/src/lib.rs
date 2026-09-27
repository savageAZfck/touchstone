//! touchstone-identity: binds attestations to a device-held key.
//!
//! Two schemes:
//! - `ed25519` — software key (any platform; key file is the secret)
//! - `secp256r1-se` — Secure Enclave on macOS (`secure-enclave` feature)
//!
//! The signature always covers `touchstone_core::document_hash` — the
//! SHA-256 of the canonical attestation serialization.

#![forbid(unsafe_code)]

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
#[cfg(not(target_arch = "wasm32"))]
use rand::rngs::OsRng;
use touchstone_core::{document_hash, Attestation, Signature};

/// Errors from signing, key parsing, or signature verification.
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    /// Hex-decoded key or signature material was malformed.
    #[error("invalid hex key material")]
    Hex,
    /// Public key or signature could not be decoded.
    #[error("bad signature or key")]
    Verify,
    /// Secure Enclave unavailable or a key operation failed.
    #[error("secure enclave unavailable or key op failed: {0}")]
    Enclave(String),
    /// Attestation (de)serialization failed.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// A software Ed25519 signer.
pub struct Ed25519Signer {
    key: SigningKey,
}

impl Ed25519Signer {
    /// Generate a fresh keypair. Not available on wasm32 (no OS RNG);
    /// wasm consumers verify only.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn generate() -> Self {
        Self {
            key: SigningKey::generate(&mut OsRng),
        }
    }

    /// Build a signer from a raw 32-byte secret.
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self {
            key: SigningKey::from_bytes(&bytes),
        }
    }

    /// Build a signer from a hex-encoded 32-byte secret.
    pub fn from_hex(hex_str: &str) -> Result<Self, IdentityError> {
        let bytes = hex::decode(hex_str).map_err(|_| IdentityError::Hex)?;
        let arr: [u8; 32] = bytes.try_into().map_err(|_| IdentityError::Hex)?;
        Ok(Self::from_bytes(arr))
    }

    /// Hex-encoded secret key — the caller is responsible for storing it
    /// safely; this is the whole identity.
    #[must_use]
    pub fn secret_hex(&self) -> String {
        hex::encode(self.key.to_bytes())
    }

    /// Hex-encoded public key, as embedded in signatures.
    #[must_use]
    pub fn public_hex(&self) -> String {
        hex::encode(self.key.verifying_key().to_bytes())
    }

    /// Sign an attestation in place — sets the `signature` field.
    pub fn sign_attestation(&self, doc: &mut Attestation) {
        let hash = document_hash(doc);
        let sig = self.key.sign(&hash);
        doc.signature = Some(Signature {
            scheme: "ed25519".into(),
            pubkey: self.public_hex(),
            sig: hex::encode(sig.to_bytes()),
        });
    }
}

/// Verify an attestation's embedded signature (ed25519 or enclave pubkey).
///
/// For `secp256r1-se` signatures this verifies structure + key binding but
/// returns `Ok(true)` on the curve check only when the `secure-enclave`
/// both schemes verify on every target — only enclave *signing* needs macOS.
pub fn verify_attestation(doc: &Attestation) -> Result<bool, IdentityError> {
    let sig = doc.signature.as_ref().ok_or(IdentityError::Verify)?;
    let hash = document_hash(doc);
    match sig.scheme.as_str() {
        "ed25519" => {
            let pk_bytes: [u8; 32] = hex::decode(&sig.pubkey)
                .map_err(|_| IdentityError::Hex)?
                .try_into()
                .map_err(|_| IdentityError::Hex)?;
            let sig_bytes: [u8; 64] = hex::decode(&sig.sig)
                .map_err(|_| IdentityError::Hex)?
                .try_into()
                .map_err(|_| IdentityError::Hex)?;
            let vk = VerifyingKey::from_bytes(&pk_bytes).map_err(|_| IdentityError::Verify)?;
            let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes);
            Ok(vk.verify(&hash, &signature).is_ok())
        }
        // Portable: p256 verifies X9.62/DER sigs from the Enclave anywhere,
        // including wasm — only *signing* needs the secure-enclave feature.
        "secp256r1-se" => verify_p256(sig, &hash),
        other => Err(IdentityError::Enclave(format!("unknown scheme: {other}"))),
    }
}

/// Secure Enclave signer (macOS only, `secure-enclave` feature).
#[cfg(feature = "secure-enclave")]
pub mod enclave {
    use super::{document_hash, Attestation, IdentityError, Signature};
    use security_framework::key::{Algorithm, GenerateKeyOptions, KeyType, SecKey, Token};

    /// A signing key that lives in the Secure Enclave.
    pub struct EnclaveSigner {
        key: SecKey,
    }

    impl EnclaveSigner {
        /// Generate a new non-exportable P-256 key inside the Enclave.
        pub fn generate() -> Result<Self, IdentityError> {
            let mut opts = GenerateKeyOptions::default();
            opts.set_key_type(KeyType::ec());
            opts.set_size_in_bits(256);
            opts.set_token(Token::SecureEnclave);
            let key = SecKey::new(&opts).map_err(|e| IdentityError::Enclave(e.to_string()))?;
            Ok(Self { key })
        }

        /// Hex-encoded SEC1 public key point (uncompressed).
        pub fn public_hex(&self) -> Result<String, IdentityError> {
            let pub_key = self
                .key
                .public_key()
                .ok_or_else(|| IdentityError::Enclave("no public key".into()))?;
            let data = pub_key
                .external_representation()
                .ok_or_else(|| IdentityError::Enclave("public key export failed".into()))?;
            Ok(hex::encode(data.bytes()))
        }

        /// Sign an attestation in place — sets the `signature` field with
        /// scheme `secp256r1-se`. The private key never leaves the Enclave.
        pub fn sign_attestation(&self, doc: &mut Attestation) -> Result<(), IdentityError> {
            let hash = document_hash(doc);
            let sig = self
                .key
                .create_signature(Algorithm::ECDSASignatureDigestX962SHA256, &hash)
                .map_err(|e| IdentityError::Enclave(e.to_string()))?;
            doc.signature = Some(Signature {
                scheme: "secp256r1-se".into(),
                pubkey: self.public_hex()?,
                sig: hex::encode(sig),
            });
            Ok(())
        }
    }
}

/// Verify a P-256/ECDSA X9.62 (DER) signature over a pre-hashed digest —
/// the format the Secure Enclave emits for `ECDSASignatureDigestX962SHA256`.
/// Pure Rust, runs on every target including wasm32.
fn verify_p256(sig: &Signature, hash: &[u8; 32]) -> Result<bool, IdentityError> {
    use p256::ecdsa::{signature::hazmat::PrehashVerifier, Signature as P256Sig};
    let pk_bytes = hex::decode(&sig.pubkey).map_err(|_| IdentityError::Hex)?;
    let sig_bytes = hex::decode(&sig.sig).map_err(|_| IdentityError::Hex)?;
    let vk =
        p256::ecdsa::VerifyingKey::from_sec1_bytes(&pk_bytes).map_err(|_| IdentityError::Verify)?;
    let signature = P256Sig::from_der(&sig_bytes).map_err(|_| IdentityError::Verify)?;
    Ok(vk.verify_prehash(hash, &signature).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use touchstone_core::{CheckResult, Organ, Status, Subject, Verdict};

    fn doc() -> Attestation {
        Attestation {
            spec: "touchstone/0.1".into(),
            subject: Subject {
                name: "t".into(),
                version: "0".into(),
                host: "h".into(),
            },
            timestamp: "t".into(),
            checks: vec![CheckResult {
                id: "a".into(),
                organ: Organ::Awake,
                status: Status::Pass,
                evidence: serde_json::Value::Null,
                control: false,
            }],
            verdict: Verdict::Partial,
            signature: None,
        }
    }

    #[test]
    fn ed25519_sign_verify_roundtrip() {
        let signer = Ed25519Signer::generate();
        let mut d = doc();
        signer.sign_attestation(&mut d);
        assert!(verify_attestation(&d).unwrap());
    }

    #[test]
    fn tamper_breaks_signature() {
        let signer = Ed25519Signer::generate();
        let mut d = doc();
        signer.sign_attestation(&mut d);
        d.checks[0].status = Status::Fail;
        assert!(!verify_attestation(&d).unwrap());
    }

    #[test]
    fn wrong_key_fails() {
        let a = Ed25519Signer::generate();
        let b = Ed25519Signer::generate();
        let mut d = doc();
        a.sign_attestation(&mut d);
        d.signature.as_mut().unwrap().pubkey = b.public_hex();
        assert!(!verify_attestation(&d).unwrap());
    }
}
