//! touchstone-identity: binds attestations to a device-held key.
//!
//! Two schemes:
//! - `ed25519` — software key (any platform; key file is the secret)
//! - `secp256r1-se` — Secure Enclave on macOS (`secure-enclave` feature)
//!
//! The signature always covers `touchstone_core::document_hash` — the
//! SHA-256 of the canonical attestation serialization.

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use touchstone_core::{document_hash, Attestation, Signature};

#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("invalid hex key material")]
    Hex,
    #[error("bad signature or key")]
    Verify,
    #[error("secure enclave unavailable or key op failed: {0}")]
    Enclave(String),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// A software Ed25519 signer.
pub struct Ed25519Signer {
    key: SigningKey,
}

impl Ed25519Signer {
    pub fn generate() -> Self {
        Self {
            key: SigningKey::generate(&mut OsRng),
        }
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self {
            key: SigningKey::from_bytes(&bytes),
        }
    }

    pub fn from_hex(hex_str: &str) -> Result<Self, IdentityError> {
        let bytes = hex::decode(hex_str).map_err(|_| IdentityError::Hex)?;
        let arr: [u8; 32] = bytes.try_into().map_err(|_| IdentityError::Hex)?;
        Ok(Self::from_bytes(arr))
    }

    pub fn secret_hex(&self) -> String {
        hex::encode(self.key.to_bytes())
    }

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
/// feature is compiled in; otherwise it returns `Err` rather than lie.
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
        "secp256r1-se" => {
            #[cfg(feature = "secure-enclave")]
            {
                verify_p256(sig, &hash)
            }
            #[cfg(not(feature = "secure-enclave"))]
            {
                Err(IdentityError::Enclave(
                    "secp256r1-se verification requires the secure-enclave feature".into(),
                ))
            }
        }
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

        pub fn public_hex(&self) -> Result<String, IdentityError> {
            let pub_key = self
                .key
                .public_key()
                .ok_or_else(|| IdentityError::Enclave("no public key".into()))?;
            let data = pub_key
                .external_representation()
                .map_err(|e| IdentityError::Enclave(e.to_string()))?;
            Ok(hex::encode(data))
        }

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

#[cfg(feature = "secure-enclave")]
fn verify_p256(sig: &Signature, hash: &[u8; 32]) -> Result<bool, IdentityError> {
    use security_framework::key::{Algorithm, KeyType, SecKey};
    let pk_bytes = hex::decode(&sig.pubkey).map_err(|_| IdentityError::Hex)?;
    let sig_bytes = hex::decode(&sig.sig).map_err(|_| IdentityError::Hex)?;
    let key = SecKey::from_data(KeyType::ec(), &pk_bytes)
        .map_err(|e| IdentityError::Enclave(e.to_string()))?;
    key.verify_signature(Algorithm::ECDSASignatureDigestX962SHA256, hash, &sig_bytes)
        .map_err(|e| IdentityError::Enclave(e.to_string()))
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
