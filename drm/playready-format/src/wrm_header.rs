/*!
    WRM (Windows Rights Management) Header XML format and PRH/PRO binary containers.
*/

use core::fmt;
use core::str::FromStr;

use quick_xml::events::Event;

use drm_core::{ParseError, Reader, eq_ignore_ascii_case, trim_ascii};

use crate::error::FormatError;

// ---------------------------------------------------------------------------
// Binary: PlayReady Header (PRH) and PlayReady Object (PRO)
// All fields are LITTLE-ENDIAN.
// ---------------------------------------------------------------------------

/**
    PlayReady Object record type: WRM Header XML (UTF-16 LE).
*/
pub const RECORD_TYPE_WRM_HEADER: u16 = 1;

/**
    PlayReady Header — wraps one or more PlayReady Object records.
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayReadyHeader {
    pub records: Vec<PlayReadyObject>,
}

/**
    A single PlayReady Object record.
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayReadyObject {
    pub record_type: u16,
    pub data: Vec<u8>,
}

impl PlayReadyHeader {
    /**
        Parse a PlayReady Header from raw bytes.

        Layout (little-endian):
        - length: u32le (total byte length of the header)
        - record_count: u16le
        - records: \[PlayReadyObject; record_count\]
    */
    pub fn from_bytes(data: &[u8]) -> Result<Self, FormatError> {
        let mut r = Reader::new(data);

        let _length = r.read_u32le()?;
        let record_count = r.read_u16le()? as usize;

        let mut records = Vec::with_capacity(record_count);
        for _ in 0..record_count {
            let record_type = r.read_u16le()?;
            let record_len = r.read_u16le()? as usize;
            let record_data = r.read_bytes(record_len)?.to_vec();
            records.push(PlayReadyObject {
                record_type,
                data: record_data,
            });
        }

        Ok(Self { records })
    }

    /**
        Get the first WRM Header XML string from type-1 records.
    */
    pub fn wrm_header_xml(&self) -> Option<Result<String, FormatError>> {
        self.records
            .iter()
            .find(|r| r.record_type == RECORD_TYPE_WRM_HEADER)
            .map(|r| r.as_utf16le_string())
    }
}

impl PlayReadyObject {
    /**
        Decode the record data as a UTF-16 LE string.
    */
    pub fn as_utf16le_string(&self) -> Result<String, FormatError> {
        if !self.data.len().is_multiple_of(2) {
            return Err(FormatError::InvalidUtf16("odd byte count".into()));
        }
        let u16s: Vec<u16> = self
            .data
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        // Strip trailing null if present
        let trimmed = if u16s.last() == Some(&0) {
            &u16s[..u16s.len() - 1]
        } else {
            &u16s
        };
        String::from_utf16(trimmed).map_err(|e| FormatError::InvalidUtf16(e.to_string()))
    }
}

// ---------------------------------------------------------------------------
// XML: WRM Header
// ---------------------------------------------------------------------------

/**
    Parsed WRM Header XML content.
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrmHeader {
    pub version: WrmHeaderVersion,
    pub kids: Vec<SignedKeyId>,
    pub la_url: Option<String>,
    pub lui_url: Option<String>,
    pub ds_id: Option<String>,
}

/**
    WRM Header version.
*/
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WrmHeaderVersion {
    V4_0_0_0,
    V4_1_0_0,
    V4_2_0_0,
    V4_3_0_0,
}

impl WrmHeaderVersion {
    pub const fn from_name(name: &[u8]) -> Option<Self> {
        let name = trim_ascii(name);
        match name.len() {
            7 if eq_ignore_ascii_case(name, b"4.0.0.0") => Some(Self::V4_0_0_0),
            7 if eq_ignore_ascii_case(name, b"4.1.0.0") => Some(Self::V4_1_0_0),
            7 if eq_ignore_ascii_case(name, b"4.2.0.0") => Some(Self::V4_2_0_0),
            7 if eq_ignore_ascii_case(name, b"4.3.0.0") => Some(Self::V4_3_0_0),
            _ => None,
        }
    }

    pub const fn to_name(self) -> &'static str {
        match self {
            Self::V4_0_0_0 => "4.0.0.0",
            Self::V4_1_0_0 => "4.1.0.0",
            Self::V4_2_0_0 => "4.2.0.0",
            Self::V4_3_0_0 => "4.3.0.0",
        }
    }
}

impl fmt::Display for WrmHeaderVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_name())
    }
}

impl FromStr for WrmHeaderVersion {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_name(s.as_bytes()).ok_or_else(|| ParseError {
            kind: "WRM header version",
            value: s.to_owned(),
        })
    }
}

/**
    A KID entry from a WRM Header, with optional algorithm and checksum.
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedKeyId {
    /**
        Key ID as a standard big-endian 16-byte UUID
        (already swapped from PlayReady's GUID bytes_le format).
    */
    pub key_id: [u8; 16],
    pub alg_id: Option<AlgId>,
    pub checksum: Option<Vec<u8>>,
}

/**
    Content encryption algorithm identifier from WRM Header XML.
*/
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AlgId {
    AesCtr,
    AesCbc,
    Cocktail,
}

impl AlgId {
    pub const fn from_name(name: &[u8]) -> Option<Self> {
        let name = trim_ascii(name);
        match name.len() {
            6 if eq_ignore_ascii_case(name, b"AESCTR") => Some(Self::AesCtr),
            6 if eq_ignore_ascii_case(name, b"AESCBC") => Some(Self::AesCbc),
            8 if eq_ignore_ascii_case(name, b"COCKTAIL") => Some(Self::Cocktail),
            _ => None,
        }
    }

    pub const fn to_name(self) -> &'static str {
        match self {
            Self::AesCtr => "AESCTR",
            Self::AesCbc => "AESCBC",
            Self::Cocktail => "COCKTAIL",
        }
    }
}

impl fmt::Display for AlgId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_name())
    }
}

impl FromStr for AlgId {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_name(s.as_bytes()).ok_or_else(|| ParseError {
            kind: "algorithm ID",
            value: s.to_owned(),
        })
    }
}

// ---------------------------------------------------------------------------
// GUID byte-swap helpers
// ---------------------------------------------------------------------------

/**
    Convert a PlayReady KID (GUID mixed-endian / bytes_le) to standard big-endian UUID bytes.

    PlayReady encodes the first three GUID groups in little-endian:
    - bytes 0-3: reversed
    - bytes 4-5: reversed
    - bytes 6-7: reversed
    - bytes 8-15: unchanged
*/
pub fn kid_to_uuid(kid_bytes: &[u8; 16]) -> [u8; 16] {
    let mut uuid = *kid_bytes;
    uuid[0..4].reverse();
    uuid[4..6].reverse();
    uuid[6..8].reverse();
    uuid
}

/**
    Convert a standard big-endian UUID to PlayReady KID (GUID mixed-endian / bytes_le).
*/
pub fn uuid_to_kid(uuid_bytes: &[u8; 16]) -> [u8; 16] {
    // Same operation — reversing is self-inverse
    kid_to_uuid(uuid_bytes)
}

// ---------------------------------------------------------------------------
// WRM Header XML parsing
// ---------------------------------------------------------------------------

/**
    Decode a base64 KID value (from WRM XML) into a 16-byte UUID.

    The base64-decoded bytes are in GUID mixed-endian format, so we swap to big-endian.
*/
fn decode_kid_base64(b64: &str) -> Result<[u8; 16], FormatError> {
    use data_encoding::BASE64;
    let bytes = BASE64
        .decode(b64.as_bytes())
        .map_err(|e| FormatError::Malformed(format!("invalid base64 KID: {e}")))?;
    if bytes.len() != 16 {
        return Err(FormatError::Malformed(format!(
            "KID decoded to {} bytes, expected 16",
            bytes.len()
        )));
    }
    let mut kid = [0u8; 16];
    kid.copy_from_slice(&bytes);
    Ok(kid_to_uuid(&kid))
}

/**
    Decode a base64 checksum string.
*/
fn decode_checksum_base64(s: &str) -> Result<Vec<u8>, FormatError> {
    use data_encoding::BASE64;
    BASE64
        .decode(s.as_bytes())
        .map_err(|e| FormatError::Malformed(format!("invalid base64 checksum: {e}")))
}

/**
    Extract KID attributes from a `<KID>` element (v4.1+).

    Looks for `VALUE`, `ALGID`, and `CHECKSUM` attributes and returns
    a `SignedKeyId` if `VALUE` is present.
*/
fn parse_kid_element<'a>(
    attrs: impl Iterator<Item = quick_xml::events::attributes::Attribute<'a>>,
) -> Result<Option<SignedKeyId>, FormatError> {
    let mut kid_value = None;
    let mut kid_alg = None;
    let mut kid_checksum = None;

    for attr in attrs {
        match attr.key.as_ref() {
            b"VALUE" => {
                kid_value = Some(String::from_utf8_lossy(&attr.value).into_owned());
            }
            b"ALGID" => {
                kid_alg = Some(String::from_utf8_lossy(&attr.value).into_owned());
            }
            b"CHECKSUM" => {
                kid_checksum = Some(String::from_utf8_lossy(&attr.value).into_owned());
            }
            _ => {}
        }
    }

    let Some(b64) = kid_value else {
        return Ok(None);
    };

    let key_id = decode_kid_base64(&b64)?;
    let alg_id = kid_alg.as_deref().and_then(|s| s.parse().ok());
    let checksum = kid_checksum
        .as_deref()
        .map(decode_checksum_base64)
        .transpose()?;

    Ok(Some(SignedKeyId {
        key_id,
        alg_id,
        checksum,
    }))
}

impl WrmHeader {
    /**
        Parse a WRM Header from XML string.

        Supports versions 4.0 through 4.3.
    */
    pub fn from_xml(xml: &str) -> Result<Self, FormatError> {
        let mut reader = quick_xml::Reader::from_str(xml);

        let mut version = None;
        let mut kids = Vec::new();
        let mut la_url = None;
        let mut lui_url = None;
        let mut ds_id = None;

        // Track element path for context
        let mut path: Vec<String> = Vec::new();

        // v4.0 temporaries: KID text is collected first, ALGID comes later,
        // so we defer creating the SignedKeyId until the end.
        let mut v40_kid_texts: Vec<String> = Vec::new();
        let mut v40_algid_text: Option<String> = None;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();

                    if name == "WRMHEADER" {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"version" {
                                let v = String::from_utf8_lossy(&attr.value).into_owned();
                                version = v.parse().ok();
                            }
                        }
                    }

                    // v4.1+: KID as element with attributes
                    if name == "KID"
                        && path_contains(&path, "PROTECTINFO")
                        && let Some(kid) = parse_kid_element(e.attributes().flatten())?
                    {
                        kids.push(kid);
                    }

                    path.push(name);
                }
                Ok(Event::End(_)) => {
                    path.pop();
                }
                Ok(Event::Text(ref e)) => {
                    let text = e.unescape().unwrap_or_default().into_owned();
                    if let Some(current) = path.last() {
                        match current.as_str() {
                            // v4.0: KID as text child of DATA (not inside PROTECTINFO)
                            "KID" if !path_contains(&path, "PROTECTINFO") => {
                                v40_kid_texts.push(text);
                            }
                            // v4.0: ALGID as text child of PROTECTINFO
                            "ALGID" => {
                                v40_algid_text = Some(text);
                            }
                            "LA_URL" => la_url = Some(text),
                            "LUI_URL" => lui_url = Some(text),
                            "DS_ID" => ds_id = Some(text),
                            _ => {}
                        }
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();

                    // v4.1+: <KID VALUE="..." ALGID="..." CHECKSUM="..." />
                    if name == "KID"
                        && path_contains(&path, "PROTECTINFO")
                        && let Some(kid) = parse_kid_element(e.attributes().flatten())?
                    {
                        kids.push(kid);
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(FormatError::InvalidXml(e.to_string())),
                _ => {}
            }
        }

        let version = version
            .ok_or_else(|| FormatError::InvalidXml("missing WRMHEADER version attribute".into()))?;

        // v4.0: create SignedKeyIds now that we've seen both KID text and ALGID text
        let v40_alg = v40_algid_text.as_deref().and_then(|s| s.parse().ok());
        for b64 in &v40_kid_texts {
            let key_id = decode_kid_base64(b64.trim())?;
            kids.push(SignedKeyId {
                key_id,
                alg_id: v40_alg,
                checksum: None,
            });
        }

        Ok(Self {
            version,
            kids,
            la_url,
            lui_url,
            ds_id,
        })
    }
}

fn path_contains(path: &[String], name: &str) -> bool {
    path.iter().any(|s| s == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kid_to_uuid_swap() {
        let kid: [u8; 16] = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D,
            0x0E, 0x0F,
        ];
        let uuid = kid_to_uuid(&kid);
        assert_eq!(
            uuid,
            [
                0x03, 0x02, 0x01, 0x00, 0x05, 0x04, 0x07, 0x06, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D,
                0x0E, 0x0F
            ]
        );
    }

    #[test]
    fn kid_uuid_round_trip() {
        let original: [u8; 16] = [
            0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
            0x99, 0x00,
        ];
        assert_eq!(uuid_to_kid(&kid_to_uuid(&original)), original);
    }

    #[test]
    fn parse_prh_binary() {
        let xml_str = "<WRMHEADER version=\"4.3.0.0\"><DATA></DATA></WRMHEADER>";
        let xml_utf16: Vec<u8> = xml_str
            .encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .collect();

        let mut prh = Vec::new();
        let record_len = xml_utf16.len() as u16;
        let total_len = (6 + 4 + xml_utf16.len()) as u32;
        prh.extend_from_slice(&total_len.to_le_bytes());
        prh.extend_from_slice(&1u16.to_le_bytes());
        prh.extend_from_slice(&1u16.to_le_bytes());
        prh.extend_from_slice(&record_len.to_le_bytes());
        prh.extend_from_slice(&xml_utf16);

        let header = PlayReadyHeader::from_bytes(&prh).unwrap();
        assert_eq!(header.records.len(), 1);
        assert_eq!(header.records[0].record_type, RECORD_TYPE_WRM_HEADER);

        let xml = header.wrm_header_xml().unwrap().unwrap();
        assert!(xml.contains("WRMHEADER"));
    }

    #[test]
    fn parse_wrm_v43_kids() {
        let xml = r#"<WRMHEADER version="4.3.0.0">
            <DATA>
                <PROTECTINFO>
                    <KIDS>
                        <KID VALUE="EBQ0VneJd0KQoLMBm3mUiw==" ALGID="AESCTR" CHECKSUM="abc=" />
                    </KIDS>
                </PROTECTINFO>
                <LA_URL>https://example.com/license</LA_URL>
            </DATA>
        </WRMHEADER>"#;

        let wrm = WrmHeader::from_xml(xml).unwrap();
        assert_eq!(wrm.version, WrmHeaderVersion::V4_3_0_0);
        assert_eq!(wrm.kids.len(), 1);
        assert_eq!(wrm.kids[0].alg_id, Some(AlgId::AesCtr));
        assert!(wrm.kids[0].checksum.is_some());
        assert_eq!(wrm.la_url.as_deref(), Some("https://example.com/license"));
    }

    #[test]
    fn parse_wrm_v40_kid() {
        let xml = r#"<WRMHEADER version="4.0.0.0">
            <DATA>
                <KID>EBQ0VneJd0KQoLMBm3mUiw==</KID>
                <PROTECTINFO>
                    <ALGID>AESCTR</ALGID>
                </PROTECTINFO>
                <LA_URL>https://example.com</LA_URL>
            </DATA>
        </WRMHEADER>"#;

        let wrm = WrmHeader::from_xml(xml).unwrap();
        assert_eq!(wrm.version, WrmHeaderVersion::V4_0_0_0);
        assert_eq!(wrm.kids.len(), 1);
        assert_eq!(wrm.kids[0].alg_id, Some(AlgId::AesCtr));
        assert!(wrm.kids[0].checksum.is_none());
    }

    #[test]
    fn wrm_header_version_display() {
        assert_eq!(WrmHeaderVersion::V4_0_0_0.to_string(), "4.0.0.0");
        assert_eq!(WrmHeaderVersion::V4_3_0_0.to_string(), "4.3.0.0");
    }

    #[test]
    fn wrm_header_version_from_str() {
        assert_eq!(
            "4.0.0.0".parse::<WrmHeaderVersion>().unwrap(),
            WrmHeaderVersion::V4_0_0_0
        );
        assert!("5.0.0.0".parse::<WrmHeaderVersion>().is_err());
    }

    #[test]
    fn alg_id_round_trip() {
        for alg in [AlgId::AesCtr, AlgId::AesCbc, AlgId::Cocktail] {
            let name = alg.to_name();
            let parsed: AlgId = name.parse().unwrap();
            assert_eq!(parsed, alg);
        }
        assert!("UNKNOWN".parse::<AlgId>().is_err());
    }
}
