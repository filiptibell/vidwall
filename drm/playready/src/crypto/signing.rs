use p256::{
    AffinePoint,
    ecdsa::{
        Signature, SigningKey, VerifyingKey,
        signature::{Signer, Verifier},
    },
    elliptic_curve::sec1::FromEncodedPoint,
};

use crate::error::{CdmError, CdmResult};

/**
    ECDSA-SHA256 sign a message using a P-256 private key.

    Input: raw message bytes (NOT pre-hashed; hashing is done internally).
    Key: 32-byte P-256 private key scalar.
    Output: 64 bytes (32-byte R || 32-byte S), big-endian.

    Uses RFC 6979 deterministic nonce generation.
*/
pub fn ecdsa_sha256_sign(private_key: &[u8; 32], message: &[u8]) -> CdmResult<[u8; 64]> {
    let signing_key = SigningKey::from_bytes(private_key.into())
        .map_err(|e| CdmError::EccKeyParse(format!("invalid signing key: {e}")))?;

    let signature: Signature = signing_key.sign(message);

    let mut out = [0u8; 64];
    out.copy_from_slice(&signature.to_bytes());
    Ok(out)
}

/**
    ECDSA-SHA256 verify a signature over a message using a P-256 public key.

    Input: raw message bytes (NOT pre-hashed).
    Key: 64-byte P-256 public key (X || Y).
    Signature: raw R||S bytes. Accepts both 64-byte fixed-size and
    DER-encoded signatures (BCert signatures may be DER-encoded).
*/
pub fn ecdsa_sha256_verify(
    public_key: &[u8; 64],
    message: &[u8],
    signature: &[u8],
) -> CdmResult<()> {
    let verifying_key = parse_verifying_key(public_key)?;

    let sig = if signature.len() == 64 {
        // Raw R||S format
        Signature::from_bytes(signature.into()).map_err(|_| CdmError::EcdsaSignatureMismatch)?
    } else {
        // Try DER-encoded
        Signature::from_der(signature).map_err(|_| CdmError::EcdsaSignatureMismatch)?
    };

    verifying_key
        .verify(message, &sig)
        .map_err(|_| CdmError::EcdsaSignatureMismatch)
}

/**
    Parse a 64-byte public key (X || Y) into a VerifyingKey.
*/
fn parse_verifying_key(xy: &[u8; 64]) -> CdmResult<VerifyingKey> {
    // Build SEC1 uncompressed: 0x04 || X || Y
    let mut sec1 = [0u8; 65];
    sec1[0] = 0x04;
    sec1[1..].copy_from_slice(xy);

    let encoded = p256::EncodedPoint::from_bytes(sec1)
        .map_err(|e| CdmError::EccKeyParse(format!("invalid point encoding: {e}")))?;

    let point = Option::from(AffinePoint::from_encoded_point(&encoded))
        .ok_or_else(|| CdmError::EccKeyParse("point not on curve".into()))?;

    VerifyingKey::from_affine(point)
        .map_err(|e| CdmError::EccKeyParse(format!("invalid verifying key: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::{
        ProjectivePoint, Scalar,
        elliptic_curve::{Field, rand_core::OsRng, sec1::ToEncodedPoint},
    };

    fn generate_keypair() -> ([u8; 32], [u8; 64]) {
        let sk = Scalar::random(&mut OsRng);
        let pk = (ProjectivePoint::GENERATOR * sk).to_affine();
        let encoded = pk.to_encoded_point(false);

        let mut private_key = [0u8; 32];
        private_key.copy_from_slice(&sk.to_bytes());

        let mut public_key = [0u8; 64];
        public_key.copy_from_slice(&encoded.as_bytes()[1..65]);

        (private_key, public_key)
    }

    #[test]
    fn sign_verify_round_trip() {
        let (sk, pk) = generate_keypair();
        let message = b"test message for ECDSA signing";

        let sig = ecdsa_sha256_sign(&sk, message).unwrap();
        assert_eq!(sig.len(), 64);

        ecdsa_sha256_verify(&pk, message, &sig).unwrap();
    }

    #[test]
    fn verify_rejects_tampered_message() {
        let (sk, pk) = generate_keypair();
        let message = b"original message";

        let sig = ecdsa_sha256_sign(&sk, message).unwrap();
        let result = ecdsa_sha256_verify(&pk, b"tampered message", &sig);
        assert!(result.is_err());
    }

    #[test]
    fn verify_rejects_wrong_key() {
        let (sk1, _pk1) = generate_keypair();
        let (_sk2, pk2) = generate_keypair();
        let message = b"test";

        let sig = ecdsa_sha256_sign(&sk1, message).unwrap();
        let result = ecdsa_sha256_verify(&pk2, message, &sig);
        assert!(result.is_err());
    }

    #[test]
    fn deterministic_signatures() {
        let (sk, _pk) = generate_keypair();
        let message = b"deterministic test";

        let sig1 = ecdsa_sha256_sign(&sk, message).unwrap();
        let sig2 = ecdsa_sha256_sign(&sk, message).unwrap();
        // RFC 6979: same key + same message = same signature
        assert_eq!(sig1, sig2);
    }
}
