/*!
    PRD (PlayReady Device) file format parsing and serialization.
*/

use crate::error::FormatError;

/**
    PRD file magic bytes.
*/
pub const PRD_MAGIC: &[u8; 3] = b"PRD";

/**
    Raw ECC P-256 key as stored in PRD files (96 bytes).

    Layout: private scalar (32B big-endian) || public X (32B) || public Y (32B).
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEccKey {
    pub private_key: [u8; 32],
    pub public_key: [u8; 64],
}

impl RawEccKey {
    /**
        Parse a 96-byte ECC key from a byte slice.
    */
    pub fn from_bytes(data: &[u8]) -> Result<Self, FormatError> {
        if data.len() < 96 {
            return Err(FormatError::UnexpectedEof {
                needed: 96,
                have: data.len(),
            });
        }
        let mut private_key = [0u8; 32];
        let mut public_key = [0u8; 64];
        private_key.copy_from_slice(&data[..32]);
        public_key.copy_from_slice(&data[32..96]);
        Ok(Self {
            private_key,
            public_key,
        })
    }

    /**
        Serialize to 96 bytes.
    */
    pub fn to_bytes(&self) -> [u8; 96] {
        let mut buf = [0u8; 96];
        buf[..32].copy_from_slice(&self.private_key);
        buf[32..96].copy_from_slice(&self.public_key);
        buf
    }
}

/**
    Parsed PRD device file.
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrdFile {
    pub version: u8,
    /**
        Group key (ECC P-256 keypair). Present in v1 and v3, absent in v2.
    */
    pub group_key: Option<RawEccKey>,
    /**
        Encryption key (ECC P-256 keypair). Used for ElGamal key exchange.
        Absent in v1.
    */
    pub encryption_key: Option<RawEccKey>,
    /**
        Signing key (ECC P-256 keypair). Used for ECDSA challenge signing.
        Absent in v1.
    */
    pub signing_key: Option<RawEccKey>,
    /**
        Raw BCert certificate chain bytes.
    */
    pub group_certificate: Vec<u8>,
}

/**
    Helper to read a big-endian u32 from a byte slice at an offset.
*/
fn read_u32be(data: &[u8], offset: usize) -> Result<u32, FormatError> {
    if offset + 4 > data.len() {
        return Err(FormatError::UnexpectedEof {
            needed: offset + 4,
            have: data.len(),
        });
    }
    Ok(u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

/**
    Helper to ensure enough bytes remain.
*/
fn ensure_len(data: &[u8], offset: usize, needed: usize) -> Result<(), FormatError> {
    if offset + needed > data.len() {
        return Err(FormatError::UnexpectedEof {
            needed: offset + needed,
            have: data.len(),
        });
    }
    Ok(())
}

impl PrdFile {
    /**
        Parse a PRD device file from raw bytes.
    */
    pub fn from_bytes(data: &[u8]) -> Result<Self, FormatError> {
        if data.len() < 4 {
            return Err(FormatError::UnexpectedEof {
                needed: 4,
                have: data.len(),
            });
        }

        if &data[..3] != PRD_MAGIC {
            return Err(FormatError::InvalidMagic {
                expected: "PRD",
                got: String::from_utf8_lossy(&data[..3]).into_owned(),
            });
        }

        let version = data[3];
        match version {
            1 => Self::parse_v1(data),
            2 => Self::parse_v2(data),
            3 => Self::parse_v3(data),
            _ => Err(FormatError::UnsupportedVersion(version)),
        }
    }

    fn parse_v1(data: &[u8]) -> Result<Self, FormatError> {
        let mut offset = 4;

        // group_key_length (u32be) + group_key
        let gk_len = read_u32be(data, offset)? as usize;
        offset += 4;
        ensure_len(data, offset, gk_len)?;
        let group_key = if gk_len >= 96 {
            Some(RawEccKey::from_bytes(&data[offset..offset + 96])?)
        } else if gk_len >= 32 {
            // Private key only — pad public with zeros (will be derived later)
            let mut private_key = [0u8; 32];
            private_key.copy_from_slice(&data[offset..offset + 32]);
            Some(RawEccKey {
                private_key,
                public_key: [0u8; 64],
            })
        } else {
            None
        };
        offset += gk_len;

        // group_certificate_length (u32be) + group_certificate
        let cert_len = read_u32be(data, offset)? as usize;
        offset += 4;
        ensure_len(data, offset, cert_len)?;
        let group_certificate = data[offset..offset + cert_len].to_vec();

        Ok(Self {
            version: 1,
            group_key,
            encryption_key: None,
            signing_key: None,
            group_certificate,
        })
    }

    fn parse_v2(data: &[u8]) -> Result<Self, FormatError> {
        let mut offset = 4;

        // group_certificate_length (u32be) + group_certificate
        let cert_len = read_u32be(data, offset)? as usize;
        offset += 4;
        ensure_len(data, offset, cert_len)?;
        let group_certificate = data[offset..offset + cert_len].to_vec();
        offset += cert_len;

        // encryption_key (96 bytes)
        ensure_len(data, offset, 96)?;
        let encryption_key = RawEccKey::from_bytes(&data[offset..])?;
        offset += 96;

        // signing_key (96 bytes)
        ensure_len(data, offset, 96)?;
        let signing_key = RawEccKey::from_bytes(&data[offset..])?;

        Ok(Self {
            version: 2,
            group_key: None,
            encryption_key: Some(encryption_key),
            signing_key: Some(signing_key),
            group_certificate,
        })
    }

    fn parse_v3(data: &[u8]) -> Result<Self, FormatError> {
        let mut offset = 4;

        // group_key (96 bytes)
        ensure_len(data, offset, 96)?;
        let group_key = RawEccKey::from_bytes(&data[offset..])?;
        offset += 96;

        // encryption_key (96 bytes)
        ensure_len(data, offset, 96)?;
        let encryption_key = RawEccKey::from_bytes(&data[offset..])?;
        offset += 96;

        // signing_key (96 bytes)
        ensure_len(data, offset, 96)?;
        let signing_key = RawEccKey::from_bytes(&data[offset..])?;
        offset += 96;

        // group_certificate_length (u32be) + group_certificate
        let cert_len = read_u32be(data, offset)? as usize;
        offset += 4;
        ensure_len(data, offset, cert_len)?;
        let group_certificate = data[offset..offset + cert_len].to_vec();

        Ok(Self {
            version: 3,
            group_key: Some(group_key),
            encryption_key: Some(encryption_key),
            signing_key: Some(signing_key),
            group_certificate,
        })
    }

    /**
        Serialize as PRD v3 format.
    */
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(PRD_MAGIC);
        buf.push(3); // always write as v3

        // group_key (96 bytes) — use zeros if absent
        match &self.group_key {
            Some(k) => buf.extend_from_slice(&k.to_bytes()),
            None => buf.extend_from_slice(&[0u8; 96]),
        }

        // encryption_key (96 bytes) — use zeros if absent
        match &self.encryption_key {
            Some(k) => buf.extend_from_slice(&k.to_bytes()),
            None => buf.extend_from_slice(&[0u8; 96]),
        }

        // signing_key (96 bytes) — use zeros if absent
        match &self.signing_key {
            Some(k) => buf.extend_from_slice(&k.to_bytes()),
            None => buf.extend_from_slice(&[0u8; 96]),
        }

        // group_certificate_length (u32be) + group_certificate
        buf.extend_from_slice(&(self.group_certificate.len() as u32).to_be_bytes());
        buf.extend_from_slice(&self.group_certificate);

        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_v3_bytes(cert: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"PRD");
        buf.push(3);
        buf.extend_from_slice(&[0xAA; 96]); // group_key
        buf.extend_from_slice(&[0xBB; 96]); // encryption_key
        buf.extend_from_slice(&[0xCC; 96]); // signing_key
        buf.extend_from_slice(&(cert.len() as u32).to_be_bytes());
        buf.extend_from_slice(cert);
        buf
    }

    #[test]
    fn parse_v3_round_trip() {
        let cert = b"test-certificate-data";
        let data = make_v3_bytes(cert);
        let prd = PrdFile::from_bytes(&data).unwrap();

        assert_eq!(prd.version, 3);
        assert!(prd.group_key.is_some());
        assert!(prd.encryption_key.is_some());
        assert!(prd.signing_key.is_some());
        assert_eq!(prd.group_certificate, cert);
        assert_eq!(prd.group_key.as_ref().unwrap().private_key, [0xAA; 32]);
        assert_eq!(prd.encryption_key.as_ref().unwrap().private_key, [0xBB; 32]);
        assert_eq!(prd.signing_key.as_ref().unwrap().private_key, [0xCC; 32]);

        // Round-trip
        let rewritten = prd.to_bytes();
        let prd2 = PrdFile::from_bytes(&rewritten).unwrap();
        assert_eq!(prd, prd2);
    }

    fn make_v2_bytes(cert: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"PRD");
        buf.push(2);
        buf.extend_from_slice(&(cert.len() as u32).to_be_bytes());
        buf.extend_from_slice(cert);
        buf.extend_from_slice(&[0xBB; 96]); // encryption_key
        buf.extend_from_slice(&[0xCC; 96]); // signing_key
        buf
    }

    #[test]
    fn parse_v2() {
        let cert = b"v2-cert";
        let data = make_v2_bytes(cert);
        let prd = PrdFile::from_bytes(&data).unwrap();

        assert_eq!(prd.version, 2);
        assert!(prd.group_key.is_none());
        assert!(prd.encryption_key.is_some());
        assert!(prd.signing_key.is_some());
        assert_eq!(prd.group_certificate, cert);
    }

    #[test]
    fn bad_magic() {
        let data = b"XXX\x03rest-of-data";
        let err = PrdFile::from_bytes(data).unwrap_err();
        assert!(matches!(err, FormatError::InvalidMagic { .. }));
    }

    #[test]
    fn bad_version() {
        let mut data = Vec::new();
        data.extend_from_slice(b"PRD");
        data.push(99);
        let err = PrdFile::from_bytes(&data).unwrap_err();
        assert!(matches!(err, FormatError::UnsupportedVersion(99)));
    }

    #[test]
    fn truncated() {
        let data = b"PRD\x03";
        let err = PrdFile::from_bytes(data).unwrap_err();
        assert!(matches!(err, FormatError::UnexpectedEof { .. }));
    }

    #[test]
    fn raw_ecc_key_round_trip() {
        let mut data = [0u8; 96];
        data[0] = 0x01;
        data[31] = 0xFF;
        data[32] = 0x02;
        data[95] = 0xFE;
        let key = RawEccKey::from_bytes(&data).unwrap();
        assert_eq!(key.private_key[0], 0x01);
        assert_eq!(key.private_key[31], 0xFF);
        assert_eq!(key.public_key[0], 0x02);
        assert_eq!(key.public_key[63], 0xFE);
        assert_eq!(key.to_bytes(), data);
    }
}
