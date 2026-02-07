/*!
    BCert (Binary Certificate) chain format parsing.
*/

use crate::error::FormatError;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const CHAIN_MAGIC: &[u8; 4] = b"CHAI";
pub const CERT_MAGIC: &[u8; 4] = b"CERT";

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/**
    BCert attribute tag.
*/
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttributeTag {
    Basic = 0x0001,
    Domain = 0x0002,
    Pc = 0x0003,
    Device = 0x0004,
    Feature = 0x0005,
    Key = 0x0006,
    Manufacturer = 0x0007,
    Signature = 0x0008,
    Silverlight = 0x0009,
    Metering = 0x000A,
    ExtDataSignKey = 0x000B,
    ExtDataContainer = 0x000C,
    ExtDataSignature = 0x000D,
    ExtDataHwid = 0x000E,
    Server = 0x000F,
    SecurityVersion = 0x0010,
    SecurityVersion2 = 0x0011,
}

impl TryFrom<u16> for AttributeTag {
    type Error = FormatError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0x0001 => Ok(Self::Basic),
            0x0002 => Ok(Self::Domain),
            0x0003 => Ok(Self::Pc),
            0x0004 => Ok(Self::Device),
            0x0005 => Ok(Self::Feature),
            0x0006 => Ok(Self::Key),
            0x0007 => Ok(Self::Manufacturer),
            0x0008 => Ok(Self::Signature),
            0x0009 => Ok(Self::Silverlight),
            0x000A => Ok(Self::Metering),
            0x000B => Ok(Self::ExtDataSignKey),
            0x000C => Ok(Self::ExtDataContainer),
            0x000D => Ok(Self::ExtDataSignature),
            0x000E => Ok(Self::ExtDataHwid),
            0x000F => Ok(Self::Server),
            0x0010 => Ok(Self::SecurityVersion),
            0x0011 => Ok(Self::SecurityVersion2),
            _ => Err(FormatError::InvalidEnumValue {
                kind: "AttributeTag",
                value,
            }),
        }
    }
}

/**
    Certificate type from BasicInfo.
*/
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CertType {
    Unknown = 0,
    Pc = 1,
    Device = 2,
    Domain = 3,
    Issuer = 4,
    CrlSigner = 5,
    Service = 6,
    Silverlight = 7,
    Application = 8,
    Metering = 9,
    KeyFileSigner = 10,
    Server = 11,
    LicenseSigner = 12,
    SecureTimeServer = 13,
    RprovModelAuth = 14,
}

impl TryFrom<u32> for CertType {
    type Error = FormatError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Unknown),
            1 => Ok(Self::Pc),
            2 => Ok(Self::Device),
            3 => Ok(Self::Domain),
            4 => Ok(Self::Issuer),
            5 => Ok(Self::CrlSigner),
            6 => Ok(Self::Service),
            7 => Ok(Self::Silverlight),
            8 => Ok(Self::Application),
            9 => Ok(Self::Metering),
            10 => Ok(Self::KeyFileSigner),
            11 => Ok(Self::Server),
            12 => Ok(Self::LicenseSigner),
            13 => Ok(Self::SecureTimeServer),
            14 => Ok(Self::RprovModelAuth),
            _ => Err(FormatError::InvalidEnumValue {
                kind: "CertType",
                value: value as u16,
            }),
        }
    }
}

/**
    Key usage values.
*/
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyUsage {
    Unknown = 0,
    Sign = 1,
    EncryptKey = 2,
    SignCrl = 3,
    IssuerAll = 4,
    IssuerIndiv = 5,
    IssuerDevice = 6,
    IssuerLink = 7,
    IssuerDomain = 8,
    IssuerSilverlight = 9,
    IssuerApplication = 10,
    IssuerCrl = 11,
    IssuerMetering = 12,
    IssuerSignKeyfile = 13,
    SignKeyfile = 14,
    IssuerServer = 15,
    EncryptKeySampleProtectionRc4 = 16,
    Reserved2 = 17,
    IssuerSignLicense = 18,
    SignLicense = 19,
    SignResponse = 20,
    PrndEncryptKeyDeprecated = 21,
    EncryptKeySampleProtectionAes128Ctr = 22,
    IssuerSecureTimeServer = 23,
    IssuerRprovModelAuth = 24,
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

/**
    Parsed BCert certificate chain.
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BCertChain {
    pub version: u32,
    pub flags: u32,
    pub certificates: Vec<BCert>,
}

/**
    A single BCert certificate.
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BCert {
    pub version: u32,
    pub total_length: u32,
    pub certificate_length: u32,
    pub attributes: Vec<BCertAttribute>,
    /// Raw bytes of this certificate (for signature verification).
    raw: Vec<u8>,
}

/**
    A BCert attribute (TLV).
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BCertAttribute {
    pub flags: u16,
    pub tag: u16,
    pub data: AttributeData,
}

/**
    Parsed attribute data variants.
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributeData {
    Basic(BasicInfo),
    Domain(DomainInfo),
    Pc(PcInfo),
    Device(DeviceInfo),
    Feature(FeatureInfo),
    Key(KeyInfo),
    Manufacturer(ManufacturerInfo),
    Signature(SignatureInfo),
    Silverlight(SilverlightInfo),
    Metering(MeteringInfo),
    ExtDataSignKey(ExtDataSignKeyInfo),
    Server(ServerInfo),
    SecurityVersion(SecurityVersionInfo),
    Unknown(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicInfo {
    pub cert_id: [u8; 16],
    pub security_level: u32,
    pub flags: u32,
    pub cert_type: u32,
    pub public_key_digest: [u8; 32],
    pub expiration_date: u32,
    pub client_id: [u8; 16],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainInfo {
    pub service_id: [u8; 16],
    pub account_id: [u8; 16],
    pub revision_timestamp: u32,
    pub domain_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcInfo {
    pub security_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub max_license: u32,
    pub max_header: u32,
    pub max_chain_depth: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureInfo {
    pub features: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInfo {
    pub keys: Vec<CertKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertKey {
    pub key_type: u16,
    /// Raw public key bytes (X || Y for ECC-256, 64 bytes).
    pub key: Vec<u8>,
    pub flags: u32,
    pub usages: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManufacturerInfo {
    pub flags: u32,
    pub name: String,
    pub model_name: String,
    pub model_number: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureInfo {
    pub signature_type: u16,
    pub signature: Vec<u8>,
    /// Issuer's public key that signed this certificate.
    pub signing_key: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SilverlightInfo {
    pub security_version: u32,
    pub platform_identifier: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeteringInfo {
    pub metering_id: [u8; 16],
    pub metering_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtDataSignKeyInfo {
    pub key_type: u16,
    pub flags: u32,
    pub key: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerInfo {
    pub warning_days: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityVersionInfo {
    pub security_version: u32,
    pub platform_identifier: u32,
}

// ---------------------------------------------------------------------------
// Binary reader helpers
// ---------------------------------------------------------------------------

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn ensure(&self, n: usize) -> Result<(), FormatError> {
        if self.remaining() < n {
            Err(FormatError::UnexpectedEof {
                needed: self.pos + n,
                have: self.data.len(),
            })
        } else {
            Ok(())
        }
    }

    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], FormatError> {
        self.ensure(n)?;
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    fn read_u16be(&mut self) -> Result<u16, FormatError> {
        let b = self.read_bytes(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    fn read_u32be(&mut self) -> Result<u32, FormatError> {
        let b = self.read_bytes(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], FormatError> {
        let b = self.read_bytes(N)?;
        let mut arr = [0u8; N];
        arr.copy_from_slice(b);
        Ok(arr)
    }

    /**
        Read a null-terminated, 4-byte-aligned string field.
        `raw_len` is the declared byte length (before alignment padding).
    */
    fn read_padded_string(&mut self, raw_len: usize) -> Result<String, FormatError> {
        let aligned = (raw_len + 3) & !3;
        let bytes = self.read_bytes(aligned)?;
        // Strip trailing nulls
        let end = bytes
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(raw_len.min(aligned));
        Ok(String::from_utf8_lossy(&bytes[..end]).into_owned())
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

impl BCertChain {
    /**
        Parse a BCert chain from raw bytes.
    */
    pub fn from_bytes(data: &[u8]) -> Result<Self, FormatError> {
        let mut r = Reader::new(data);

        // CHAI magic
        let magic = r.read_bytes(4)?;
        if magic != CHAIN_MAGIC {
            return Err(FormatError::InvalidMagic {
                expected: "CHAI",
                got: String::from_utf8_lossy(magic).into_owned(),
            });
        }

        let version = r.read_u32be()?;
        let _total_length = r.read_u32be()?;
        let flags = r.read_u32be()?;
        let cert_count = r.read_u32be()? as usize;

        let mut certificates = Vec::with_capacity(cert_count);
        for _ in 0..cert_count {
            let cert = BCert::parse(&mut r)?;
            certificates.push(cert);
        }

        Ok(Self {
            version,
            flags,
            certificates,
        })
    }

    /**
        First certificate (leaf / device certificate).
    */
    pub fn leaf(&self) -> Option<&BCert> {
        self.certificates.first()
    }

    /**
        Last certificate (root / issuer certificate).
    */
    pub fn root(&self) -> Option<&BCert> {
        self.certificates.last()
    }
}

impl BCert {
    fn parse(r: &mut Reader<'_>) -> Result<Self, FormatError> {
        let cert_start = r.pos;

        let magic = r.read_bytes(4)?;
        if magic != CERT_MAGIC {
            return Err(FormatError::InvalidMagic {
                expected: "CERT",
                got: String::from_utf8_lossy(magic).into_owned(),
            });
        }

        let version = r.read_u32be()?;
        let total_length = r.read_u32be()?;
        let certificate_length = r.read_u32be()?;

        // Parse attributes within total_length bytes (from start of CERT)
        let cert_end = cert_start + total_length as usize;
        let mut attributes = Vec::new();

        while r.pos < cert_end && r.remaining() >= 8 {
            let attr = BCertAttribute::parse(r)?;
            attributes.push(attr);
        }

        // Capture raw bytes for signature verification
        let raw_end = cert_start + total_length as usize;
        let raw = if raw_end <= r.data.len() {
            r.data[cert_start..raw_end].to_vec()
        } else {
            r.data[cert_start..].to_vec()
        };

        // Advance past any remaining bytes
        r.pos = cert_end.min(r.data.len());

        Ok(Self {
            version,
            total_length,
            certificate_length,
            attributes,
            raw,
        })
    }

    /**
        Get the BasicInfo attribute if present.
    */
    pub fn basic_info(&self) -> Option<&BasicInfo> {
        self.attributes.iter().find_map(|a| match &a.data {
            AttributeData::Basic(info) => Some(info),
            _ => None,
        })
    }

    /**
        Get the KeyInfo attribute if present.
    */
    pub fn key_info(&self) -> Option<&KeyInfo> {
        self.attributes.iter().find_map(|a| match &a.data {
            AttributeData::Key(info) => Some(info),
            _ => None,
        })
    }

    /**
        Get the SignatureInfo attribute if present.
    */
    pub fn signature_info(&self) -> Option<&SignatureInfo> {
        self.attributes.iter().find_map(|a| match &a.data {
            AttributeData::Signature(info) => Some(info),
            _ => None,
        })
    }

    /**
        Get the first key with `Sign` (1) usage.
    */
    pub fn signing_key(&self) -> Option<&[u8]> {
        self.key_info().and_then(|ki| {
            ki.keys
                .iter()
                .find(|k| k.usages.contains(&(KeyUsage::Sign as u32)))
                .map(|k| k.key.as_slice())
        })
    }

    /**
        Get the first key with `EncryptKey` (2) usage.
    */
    pub fn encryption_key(&self) -> Option<&[u8]> {
        self.key_info().and_then(|ki| {
            ki.keys
                .iter()
                .find(|k| k.usages.contains(&(KeyUsage::EncryptKey as u32)))
                .map(|k| k.key.as_slice())
        })
    }

    /**
        Raw bytes covered by the signature: `[0..certificate_length]` from the cert start.
    */
    pub fn signed_bytes(&self) -> &[u8] {
        let end = (self.certificate_length as usize).min(self.raw.len());
        &self.raw[..end]
    }
}

impl BCertAttribute {
    fn parse(r: &mut Reader<'_>) -> Result<Self, FormatError> {
        let flags = r.read_u16be()?;
        let tag = r.read_u16be()?;
        let length = r.read_u32be()? as usize; // includes 8-byte header

        let data_len = length.saturating_sub(8);
        let data_bytes = r.read_bytes(data_len)?;

        let data = match AttributeTag::try_from(tag) {
            Ok(AttributeTag::Basic) => parse_basic(data_bytes)?,
            Ok(AttributeTag::Domain) => parse_domain(data_bytes)?,
            Ok(AttributeTag::Pc) => parse_pc(data_bytes)?,
            Ok(AttributeTag::Device) => parse_device(data_bytes)?,
            Ok(AttributeTag::Feature) => parse_feature(data_bytes)?,
            Ok(AttributeTag::Key) => parse_key(data_bytes)?,
            Ok(AttributeTag::Manufacturer) => parse_manufacturer(data_bytes)?,
            Ok(AttributeTag::Signature) => parse_signature(data_bytes)?,
            Ok(AttributeTag::Silverlight) => parse_silverlight(data_bytes)?,
            Ok(AttributeTag::Metering) => parse_metering(data_bytes)?,
            Ok(AttributeTag::ExtDataSignKey) => parse_ext_data_sign_key(data_bytes)?,
            Ok(AttributeTag::Server) => parse_server(data_bytes)?,
            Ok(AttributeTag::SecurityVersion | AttributeTag::SecurityVersion2) => {
                parse_security_version(data_bytes)?
            }
            // Unknown or container tags — store raw bytes
            _ => AttributeData::Unknown(data_bytes.to_vec()),
        };

        Ok(Self { flags, tag, data })
    }
}

// ---------------------------------------------------------------------------
// Attribute data parsers
// ---------------------------------------------------------------------------

fn parse_basic(data: &[u8]) -> Result<AttributeData, FormatError> {
    let mut r = Reader::new(data);
    let cert_id = r.read_array::<16>()?;
    let security_level = r.read_u32be()?;
    let flags = r.read_u32be()?;
    let cert_type = r.read_u32be()?;
    let public_key_digest = r.read_array::<32>()?;
    let expiration_date = r.read_u32be()?;
    let client_id = r.read_array::<16>()?;
    Ok(AttributeData::Basic(BasicInfo {
        cert_id,
        security_level,
        flags,
        cert_type,
        public_key_digest,
        expiration_date,
        client_id,
    }))
}

fn parse_domain(data: &[u8]) -> Result<AttributeData, FormatError> {
    let mut r = Reader::new(data);
    let service_id = r.read_array::<16>()?;
    let account_id = r.read_array::<16>()?;
    let revision_timestamp = r.read_u32be()?;
    let url_len = r.read_u32be()? as usize;
    let domain_url = r.read_padded_string(url_len)?;
    Ok(AttributeData::Domain(DomainInfo {
        service_id,
        account_id,
        revision_timestamp,
        domain_url,
    }))
}

fn parse_pc(data: &[u8]) -> Result<AttributeData, FormatError> {
    let mut r = Reader::new(data);
    let security_version = r.read_u32be()?;
    Ok(AttributeData::Pc(PcInfo { security_version }))
}

fn parse_device(data: &[u8]) -> Result<AttributeData, FormatError> {
    let mut r = Reader::new(data);
    let max_license = r.read_u32be()?;
    let max_header = r.read_u32be()?;
    let max_chain_depth = r.read_u32be()?;
    Ok(AttributeData::Device(DeviceInfo {
        max_license,
        max_header,
        max_chain_depth,
    }))
}

fn parse_feature(data: &[u8]) -> Result<AttributeData, FormatError> {
    let mut r = Reader::new(data);
    let count = r.read_u32be()? as usize;
    let mut features = Vec::with_capacity(count.min(32));
    for _ in 0..count.min(32) {
        features.push(r.read_u32be()?);
    }
    Ok(AttributeData::Feature(FeatureInfo { features }))
}

fn parse_key(data: &[u8]) -> Result<AttributeData, FormatError> {
    let mut r = Reader::new(data);
    let key_count = r.read_u32be()? as usize;
    let mut keys = Vec::with_capacity(key_count);
    for _ in 0..key_count {
        let key_type = r.read_u16be()?;
        let key_length_bits = r.read_u16be()? as usize;
        let key_length_bytes = key_length_bits / 8;
        let flags = r.read_u32be()?;
        let key = r.read_bytes(key_length_bytes)?.to_vec();
        let usages_count = r.read_u32be()? as usize;
        let mut usages = Vec::with_capacity(usages_count);
        for _ in 0..usages_count {
            usages.push(r.read_u32be()?);
        }
        keys.push(CertKey {
            key_type,
            key,
            flags,
            usages,
        });
    }
    Ok(AttributeData::Key(KeyInfo { keys }))
}

fn parse_manufacturer(data: &[u8]) -> Result<AttributeData, FormatError> {
    let mut r = Reader::new(data);
    let flags = r.read_u32be()?;
    let name_len = r.read_u32be()? as usize;
    let name = r.read_padded_string(name_len)?;
    let model_name_len = r.read_u32be()? as usize;
    let model_name = r.read_padded_string(model_name_len)?;
    let model_number_len = r.read_u32be()? as usize;
    let model_number = r.read_padded_string(model_number_len)?;
    Ok(AttributeData::Manufacturer(ManufacturerInfo {
        flags,
        name,
        model_name,
        model_number,
    }))
}

fn parse_signature(data: &[u8]) -> Result<AttributeData, FormatError> {
    let mut r = Reader::new(data);
    let signature_type = r.read_u16be()?;
    let signature_size = r.read_u16be()? as usize;
    let signature = r.read_bytes(signature_size)?.to_vec();
    let signing_key_size_bits = r.read_u32be()? as usize;
    let signing_key_size_bytes = signing_key_size_bits / 8;
    let signing_key = r.read_bytes(signing_key_size_bytes)?.to_vec();
    Ok(AttributeData::Signature(SignatureInfo {
        signature_type,
        signature,
        signing_key,
    }))
}

fn parse_silverlight(data: &[u8]) -> Result<AttributeData, FormatError> {
    let mut r = Reader::new(data);
    let security_version = r.read_u32be()?;
    let platform_identifier = r.read_u32be()?;
    Ok(AttributeData::Silverlight(SilverlightInfo {
        security_version,
        platform_identifier,
    }))
}

fn parse_metering(data: &[u8]) -> Result<AttributeData, FormatError> {
    let mut r = Reader::new(data);
    let metering_id = r.read_array::<16>()?;
    let url_len = r.read_u32be()? as usize;
    let metering_url = r.read_padded_string(url_len)?;
    Ok(AttributeData::Metering(MeteringInfo {
        metering_id,
        metering_url,
    }))
}

fn parse_ext_data_sign_key(data: &[u8]) -> Result<AttributeData, FormatError> {
    let mut r = Reader::new(data);
    let key_type = r.read_u16be()?;
    let key_length_bits = r.read_u16be()? as usize;
    let flags = r.read_u32be()?;
    let key = r.read_bytes(key_length_bits / 8)?.to_vec();
    Ok(AttributeData::ExtDataSignKey(ExtDataSignKeyInfo {
        key_type,
        flags,
        key,
    }))
}

fn parse_server(data: &[u8]) -> Result<AttributeData, FormatError> {
    let mut r = Reader::new(data);
    let warning_days = r.read_u32be()?;
    Ok(AttributeData::Server(ServerInfo { warning_days }))
}

fn parse_security_version(data: &[u8]) -> Result<AttributeData, FormatError> {
    let mut r = Reader::new(data);
    let security_version = r.read_u32be()?;
    let platform_identifier = r.read_u32be()?;
    Ok(AttributeData::SecurityVersion(SecurityVersionInfo {
        security_version,
        platform_identifier,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal BCert chain for testing.
    fn build_test_chain() -> Vec<u8> {
        // Build a minimal cert with just a BasicInfo attribute
        let mut cert_body = Vec::new();

        // BasicInfo attribute: flags(2) + tag(2) + length(4) + data(84)
        let basic_data_len = 16 + 4 + 4 + 4 + 32 + 4 + 16; // = 80
        let attr_total_len = 8 + basic_data_len;
        cert_body.extend_from_slice(&0x0001u16.to_be_bytes()); // flags: MUST_UNDERSTAND
        cert_body.extend_from_slice(&0x0001u16.to_be_bytes()); // tag: Basic
        cert_body.extend_from_slice(&(attr_total_len as u32).to_be_bytes());
        cert_body.extend_from_slice(&[0x01; 16]); // cert_id
        cert_body.extend_from_slice(&3000u32.to_be_bytes()); // security_level
        cert_body.extend_from_slice(&0u32.to_be_bytes()); // flags
        cert_body.extend_from_slice(&2u32.to_be_bytes()); // cert_type = Device
        cert_body.extend_from_slice(&[0x02; 32]); // public_key_digest
        cert_body.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes()); // expiration (never)
        cert_body.extend_from_slice(&[0x03; 16]); // client_id

        // Build CERT wrapper
        let cert_length = cert_body.len() as u32;
        let total_length = 16 + cert_length; // CERT(4) + version(4) + total_length(4) + cert_length(4) + body
        let mut cert = Vec::new();
        cert.extend_from_slice(CERT_MAGIC);
        cert.extend_from_slice(&1u32.to_be_bytes()); // version
        cert.extend_from_slice(&total_length.to_be_bytes());
        cert.extend_from_slice(&(cert_body.len() as u32).to_be_bytes()); // certificate_length
        cert.extend_from_slice(&cert_body);

        // Build CHAI wrapper
        let chain_total = 20 + cert.len(); // CHAI(4) + version(4) + total(4) + flags(4) + count(4) + certs
        let mut chain = Vec::new();
        chain.extend_from_slice(CHAIN_MAGIC);
        chain.extend_from_slice(&1u32.to_be_bytes()); // version
        chain.extend_from_slice(&(chain_total as u32).to_be_bytes());
        chain.extend_from_slice(&0u32.to_be_bytes()); // flags
        chain.extend_from_slice(&1u32.to_be_bytes()); // cert_count
        chain.extend_from_slice(&cert);

        chain
    }

    #[test]
    fn parse_basic_chain() {
        let data = build_test_chain();
        let chain = BCertChain::from_bytes(&data).unwrap();

        assert_eq!(chain.version, 1);
        assert_eq!(chain.certificates.len(), 1);

        let cert = &chain.certificates[0];
        let basic = cert.basic_info().unwrap();
        assert_eq!(basic.cert_id, [0x01; 16]);
        assert_eq!(basic.security_level, 3000);
        assert_eq!(basic.cert_type, CertType::Device as u32);
        assert_eq!(basic.public_key_digest, [0x02; 32]);
        assert_eq!(basic.expiration_date, 0xFFFFFFFF);
        assert_eq!(basic.client_id, [0x03; 16]);
    }

    #[test]
    fn leaf_and_root() {
        let data = build_test_chain();
        let chain = BCertChain::from_bytes(&data).unwrap();

        // With one cert, leaf and root are the same
        assert!(chain.leaf().is_some());
        assert!(chain.root().is_some());
        assert_eq!(
            chain.leaf().unwrap().basic_info().unwrap().cert_id,
            chain.root().unwrap().basic_info().unwrap().cert_id,
        );
    }

    #[test]
    fn bad_chain_magic() {
        let data = b"XXXX\x00\x00\x00\x01";
        let err = BCertChain::from_bytes(data).unwrap_err();
        assert!(matches!(err, FormatError::InvalidMagic { .. }));
    }

    #[test]
    fn unknown_attribute_tag() {
        // Build a cert with an unknown tag
        let mut cert_body = Vec::new();
        cert_body.extend_from_slice(&0x0000u16.to_be_bytes()); // flags
        cert_body.extend_from_slice(&0xFFFDu16.to_be_bytes()); // tag: unknown
        cert_body.extend_from_slice(&12u32.to_be_bytes()); // length (8 header + 4 data)
        cert_body.extend_from_slice(&[0xAA; 4]); // data

        let total_length = 16 + cert_body.len() as u32;
        let mut cert = Vec::new();
        cert.extend_from_slice(CERT_MAGIC);
        cert.extend_from_slice(&1u32.to_be_bytes());
        cert.extend_from_slice(&total_length.to_be_bytes());
        cert.extend_from_slice(&(cert_body.len() as u32).to_be_bytes());
        cert.extend_from_slice(&cert_body);

        let chain_total = 20 + cert.len();
        let mut chain = Vec::new();
        chain.extend_from_slice(CHAIN_MAGIC);
        chain.extend_from_slice(&1u32.to_be_bytes());
        chain.extend_from_slice(&(chain_total as u32).to_be_bytes());
        chain.extend_from_slice(&0u32.to_be_bytes());
        chain.extend_from_slice(&1u32.to_be_bytes());
        chain.extend_from_slice(&cert);

        let parsed = BCertChain::from_bytes(&chain).unwrap();
        assert_eq!(parsed.certificates[0].attributes.len(), 1);
        assert!(matches!(
            parsed.certificates[0].attributes[0].data,
            AttributeData::Unknown(_)
        ));
    }
}
