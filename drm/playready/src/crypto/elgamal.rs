use p256::{
    AffinePoint, FieldBytes, ProjectivePoint, Scalar,
    elliptic_curve::{
        Field, PrimeField,
        rand_core::OsRng,
        sec1::{FromEncodedPoint, ToEncodedPoint},
    },
};

use crate::error::{CdmError, CdmResult};

/**
    ElGamal encrypt a message point to a public key on P-256.

    Steps:
      k = random scalar
      point1 = G * k
      point2 = message_point + public_key * k
      return point1.x || point1.y || point2.x || point2.y (128 bytes)

    Used for: encrypting the session key point to the WMRM server public key.
*/
pub fn ecc256_encrypt(public_key: &[u8; 64], message_point: &[u8; 64]) -> CdmResult<[u8; 128]> {
    let pk = parse_point(public_key)?;
    let msg = parse_point(message_point)?;

    let k = Scalar::random(&mut OsRng);

    let point1 = (ProjectivePoint::GENERATOR * k).to_affine();
    let point2 = (ProjectivePoint::from(msg) + ProjectivePoint::from(pk) * k).to_affine();

    let mut output = [0u8; 128];
    output[..64].copy_from_slice(&serialize_point(&point1));
    output[64..].copy_from_slice(&serialize_point(&point2));
    Ok(output)
}

/**
    ElGamal decrypt a ciphertext using a private key on P-256.

    Steps:
      point1 = parse(ciphertext[0..64])
      point2 = parse(ciphertext[64..128])
      shared = private_key * point1
      decrypted = point2 - shared
      return decrypted.x (32 bytes)

    Used for: decrypting content keys from XMR license blobs.
    The ciphertext must be at least 128 bytes; only the first 128 are used.
*/
pub fn ecc256_decrypt(private_key: &[u8; 32], ciphertext: &[u8]) -> CdmResult<[u8; 32]> {
    if ciphertext.len() < 128 {
        return Err(CdmError::ElGamalDecryptFailed(format!(
            "ciphertext too short: {} bytes, need at least 128",
            ciphertext.len()
        )));
    }

    let p1_bytes: &[u8; 64] = ciphertext[..64].try_into().unwrap();
    let p2_bytes: &[u8; 64] = ciphertext[64..128].try_into().unwrap();

    let point1 = parse_point(p1_bytes)?;
    let point2 = parse_point(p2_bytes)?;

    let ct_scalar = Scalar::from_repr(*FieldBytes::from_slice(private_key));
    let scalar: Scalar = Option::<Scalar>::from(ct_scalar)
        .ok_or_else(|| CdmError::EccKeyParse("invalid private key scalar".into()))?;

    let shared = (ProjectivePoint::from(point1) * scalar).to_affine();
    let decrypted = (ProjectivePoint::from(point2) - ProjectivePoint::from(shared)).to_affine();

    let encoded = decrypted.to_encoded_point(false);
    let x = encoded
        .x()
        .ok_or_else(|| CdmError::ElGamalDecryptFailed("decrypted to identity point".into()))?;

    let mut result = [0u8; 32];
    result.copy_from_slice(x.as_slice());
    Ok(result)
}

/**
    Parse 64 bytes (X || Y) as an uncompressed P-256 affine point.
*/
fn parse_point(xy: &[u8; 64]) -> CdmResult<AffinePoint> {
    // Build SEC1 uncompressed encoding: 0x04 || X || Y
    let mut sec1 = [0u8; 65];
    sec1[0] = 0x04;
    sec1[1..].copy_from_slice(xy);

    let encoded = p256::EncodedPoint::from_bytes(sec1)
        .map_err(|e| CdmError::EccKeyParse(format!("invalid point encoding: {e}")))?;

    Option::from(AffinePoint::from_encoded_point(&encoded))
        .ok_or_else(|| CdmError::EccKeyParse("point not on curve".into()))
}

/**
    Serialize an P-256 affine point to 64 bytes (X || Y).
*/
fn serialize_point(point: &AffinePoint) -> [u8; 64] {
    let encoded = point.to_encoded_point(false);
    let mut out = [0u8; 64];
    // Uncompressed point is 0x04 || X(32) || Y(32), skip the tag byte
    out.copy_from_slice(&encoded.as_bytes()[1..65]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_round_trip() {
        // Generate a random keypair for the "recipient"
        let sk = Scalar::random(&mut OsRng);
        let pk = (ProjectivePoint::GENERATOR * sk).to_affine();
        let pk_bytes = serialize_point(&pk);

        let mut private_key = [0u8; 32];
        private_key.copy_from_slice(&sk.to_bytes());

        // Generate a random message point
        let msg_scalar = Scalar::random(&mut OsRng);
        let msg_point = (ProjectivePoint::GENERATOR * msg_scalar).to_affine();
        let msg_bytes = serialize_point(&msg_point);

        // Encrypt and decrypt
        let ciphertext = ecc256_encrypt(&pk_bytes, &msg_bytes).unwrap();
        assert_eq!(ciphertext.len(), 128);

        let decrypted_x = ecc256_decrypt(&private_key, &ciphertext).unwrap();

        // The decrypted x-coordinate should match the original message point's x
        let original_encoded = msg_point.to_encoded_point(false);
        let original_x = original_encoded.x().unwrap();
        assert_eq!(decrypted_x, original_x.as_slice());
    }

    #[test]
    fn decrypt_wrong_key_gives_wrong_result() {
        let sk1 = Scalar::random(&mut OsRng);
        let pk1 = (ProjectivePoint::GENERATOR * sk1).to_affine();

        let sk2 = Scalar::random(&mut OsRng);

        let msg_scalar = Scalar::random(&mut OsRng);
        let msg_point = (ProjectivePoint::GENERATOR * msg_scalar).to_affine();

        let ciphertext =
            ecc256_encrypt(&serialize_point(&pk1), &serialize_point(&msg_point)).unwrap();

        let mut wrong_key = [0u8; 32];
        wrong_key.copy_from_slice(&sk2.to_bytes());

        let decrypted_x = ecc256_decrypt(&wrong_key, &ciphertext).unwrap();

        let original_encoded = msg_point.to_encoded_point(false);
        let original_x = original_encoded.x().unwrap();
        assert_ne!(decrypted_x, original_x.as_slice());
    }

    #[test]
    fn decrypt_too_short_ciphertext() {
        let key = [0x01u8; 32];
        assert!(ecc256_decrypt(&key, &[0u8; 127]).is_err());
    }

    #[test]
    fn parse_serialize_round_trip() {
        let scalar = Scalar::random(&mut OsRng);
        let point = (ProjectivePoint::GENERATOR * scalar).to_affine();
        let bytes = serialize_point(&point);
        let parsed = parse_point(&bytes).unwrap();
        assert_eq!(parsed, point);
    }
}
