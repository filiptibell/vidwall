use drm_core::Reader;
use drm_playready_format::bcert::BCertChain;

use crate::error::{CdmError, CdmResult};

const MAGIC: &[u8] = b"PRD";

/// An ECC P-256 keypair (32-byte private scalar + 64-byte uncompressed public point).
#[derive(Debug, Clone)]
pub(crate) struct EccKeyPair {
    pub private_key: [u8; 32],
    pub public_key: [u8; 64],
}

/**
    Represents a PlayReady device loaded from a `.prd` file.

    The device holds three ECC P-256 keypairs and a group certificate chain.
    The encryption key is used for ElGamal decryption of content keys,
    the signing key for ECDSA-SHA256 challenge signing, and the group key
    (v3 only) for provisioning.
*/
#[derive(Debug, Clone)]
pub struct Device {
    /// Security level extracted from the leaf BCert's BasicInfo.
    pub security_level: u32,
    /// Group key — signs new leaf BCerts during provisioning. Only present in PRD v3.
    pub(crate) group_key: Option<EccKeyPair>,
    /// Encryption key — ElGamal decryption of content keys in license responses.
    pub(crate) encryption_key: EccKeyPair,
    /// Signing key — ECDSA-SHA256 signing of license challenge XML.
    pub(crate) signing_key: EccKeyPair,
    /// Raw group certificate chain bytes.
    pub(crate) group_certificate: Vec<u8>,
}

impl Device {
    /**
        Parse a PRD file from raw bytes.

        Supports PRD v2 (no group key) and v3 (with group key).
    */
    pub fn from_bytes(data: impl AsRef<[u8]>) -> CdmResult<Self> {
        let data = data.as_ref();
        let mut r = Reader::new(data);

        // Check magic bytes
        let magic = r.read_bytes(3).map_err(|_| CdmError::PrdTruncated)?;
        if magic != MAGIC {
            return Err(CdmError::PrdBadMagic);
        }

        // Read version
        let version = r.read_array::<1>().map_err(|_| CdmError::PrdTruncated)?[0];

        match version {
            2 => Self::parse_v2(&mut r),
            3 => Self::parse_v3(&mut r),
            _ => Err(CdmError::PrdUnsupportedVersion(version)),
        }
    }

    /**
        Parse a base64-encoded PRD file.
    */
    pub fn from_base64(prd: impl AsRef<[u8]>) -> CdmResult<Self> {
        let bytes = data_encoding::BASE64
            .decode(prd.as_ref())
            .map_err(|e| CdmError::InvalidBase64(format!("PRD: {e}")))?;
        Self::from_bytes(&bytes)
    }

    /**
        Serialize to PRD v3 format bytes.

        Always writes v3 format regardless of the version originally loaded.
        If the device was loaded from v2 (no group key), the group key
        field is written as all zeros.
    */
    pub fn to_bytes(&self) -> Vec<u8> {
        let cert_len = self.group_certificate.len() as u32;
        let total = 3 + 1 + 96 + 96 + 96 + 4 + self.group_certificate.len();
        let mut buf = Vec::with_capacity(total);

        // Header
        buf.extend(MAGIC);
        buf.push(3u8);

        // Group key (96 bytes) — zeros if absent
        match &self.group_key {
            Some(kp) => {
                buf.extend(&kp.private_key);
                buf.extend(&kp.public_key);
            }
            None => buf.extend(&[0u8; 96]),
        }

        // Encryption key (96 bytes)
        buf.extend(&self.encryption_key.private_key);
        buf.extend(&self.encryption_key.public_key);

        // Signing key (96 bytes)
        buf.extend(&self.signing_key.private_key);
        buf.extend(&self.signing_key.public_key);

        // Certificate chain
        buf.extend(&cert_len.to_be_bytes());
        buf.extend(&self.group_certificate);

        buf
    }

    /**
        Serialize to a base64-encoded PRD string.
    */
    pub fn to_base64(&self) -> String {
        data_encoding::BASE64.encode(&self.to_bytes())
    }

    /**
        Parse the group certificate chain from the stored raw bytes.
    */
    pub fn group_certificate_chain(&self) -> CdmResult<BCertChain> {
        BCertChain::from_bytes(&self.group_certificate).map_err(CdmError::from)
    }

    /**
        Returns the encryption public key (64 bytes, X || Y).
    */
    pub fn encryption_public_key(&self) -> &[u8; 64] {
        &self.encryption_key.public_key
    }

    /**
        Returns the signing public key (64 bytes, X || Y).
    */
    pub fn signing_public_key(&self) -> &[u8; 64] {
        &self.signing_key.public_key
    }

    /// PRD v2: cert_len(4) + cert + enc_key(96) + sign_key(96)
    fn parse_v2(r: &mut Reader<'_>) -> CdmResult<Self> {
        let cert_len = r.read_u32be().map_err(|_| CdmError::PrdTruncated)? as usize;
        let cert_bytes = r.read_bytes(cert_len).map_err(|_| CdmError::PrdTruncated)?;
        let encryption_key = read_ecc_keypair(r)?;
        let signing_key = read_ecc_keypair(r)?;

        let security_level = extract_security_level(cert_bytes)?;

        Ok(Self {
            security_level,
            group_key: None,
            encryption_key,
            signing_key,
            group_certificate: cert_bytes.to_vec(),
        })
    }

    /// PRD v3: group_key(96) + enc_key(96) + sign_key(96) + cert_len(4) + cert
    fn parse_v3(r: &mut Reader<'_>) -> CdmResult<Self> {
        let group_key = read_ecc_keypair(r)?;
        let encryption_key = read_ecc_keypair(r)?;
        let signing_key = read_ecc_keypair(r)?;

        let cert_len = r.read_u32be().map_err(|_| CdmError::PrdTruncated)? as usize;
        let cert_bytes = r.read_bytes(cert_len).map_err(|_| CdmError::PrdTruncated)?;

        let security_level = extract_security_level(cert_bytes)?;

        // Check if group key is all zeros (absent)
        let has_group_key = group_key.private_key != [0u8; 32];

        Ok(Self {
            security_level,
            group_key: if has_group_key { Some(group_key) } else { None },
            encryption_key,
            signing_key,
            group_certificate: cert_bytes.to_vec(),
        })
    }
}

/// Read a 96-byte ECC keypair (32 private + 64 public) from the reader.
fn read_ecc_keypair(r: &mut Reader<'_>) -> CdmResult<EccKeyPair> {
    let private_key = r.read_array::<32>().map_err(|_| CdmError::PrdTruncated)?;
    let public_key = r.read_array::<64>().map_err(|_| CdmError::PrdTruncated)?;
    Ok(EccKeyPair {
        private_key,
        public_key,
    })
}

/// Extract the security level from a raw BCertChain by parsing and reading the leaf cert.
fn extract_security_level(cert_bytes: &[u8]) -> CdmResult<u32> {
    let chain = BCertChain::from_bytes(cert_bytes).map_err(CdmError::from)?;
    let leaf = chain
        .leaf()
        .ok_or_else(|| CdmError::Format("BCert chain has no certificates".into()))?;
    let basic_info = leaf
        .basic_info()
        .ok_or_else(|| CdmError::Format("leaf BCert has no BasicInfo attribute".into()))?;
    Ok(basic_info.security_level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_magic() {
        let err = Device::from_bytes(b"XYZ\x03").unwrap_err();
        assert!(matches!(err, CdmError::PrdBadMagic));
    }

    #[test]
    fn unsupported_version() {
        let err = Device::from_bytes(b"PRD\x01").unwrap_err();
        assert!(matches!(err, CdmError::PrdUnsupportedVersion(1)));
    }

    #[test]
    fn truncated_input() {
        let err = Device::from_bytes(b"PR").unwrap_err();
        assert!(matches!(err, CdmError::PrdTruncated));
    }

    #[test]
    fn empty_input() {
        let err = Device::from_bytes(b"").unwrap_err();
        assert!(matches!(err, CdmError::PrdTruncated));
    }

    #[test]
    fn v2_truncated_cert_len() {
        // Magic + version 2 + only 2 bytes of cert_len
        let err = Device::from_bytes(b"PRD\x02\x00\x00").unwrap_err();
        assert!(matches!(err, CdmError::PrdTruncated));
    }

    #[test]
    fn v3_truncated_keys() {
        // Magic + version 3 + only a few bytes (not enough for 3 keypairs)
        let mut data = b"PRD\x03".to_vec();
        data.extend(&[0u8; 50]); // not enough for 288 bytes of keys
        let err = Device::from_bytes(&data).unwrap_err();
        assert!(matches!(err, CdmError::PrdTruncated));
    }
}
