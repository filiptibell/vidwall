/// Device type as encoded in WVD file byte offset 4.
/// Values: Chrome=1, Android=2. These are defined by the WVD file format specification,
/// not by Google's license_protocol.proto (the closest proto enum,
/// ClientIdentification.TokenType, has unrelated values).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeviceType {
    Chrome = 1,
    Android = 2,
}

impl DeviceType {
    pub const fn from_u8(u: u8) -> Option<Self> {
        match u {
            1 => Some(Self::Chrome),
            2 => Some(Self::Android),
            _ => None,
        }
    }

    pub const fn to_u8(self) -> u8 {
        self as u8
    }
}

/// Widevine security level.
/// Ref: license_protocol.proto.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SecurityLevel {
    L1 = 1,
    L2 = 2,
    L3 = 3,
}

impl SecurityLevel {
    pub const fn from_u8(u: u8) -> Option<Self> {
        match u {
            1 => Some(Self::L1),
            2 => Some(Self::L2),
            3 => Some(Self::L3),
            _ => None,
        }
    }

    pub const fn to_u8(self) -> u8 {
        self as u8
    }
}

/// Key type enumeration from License.KeyContainer.KeyType.
/// Ref: license_protocol.proto, License.KeyContainer.KeyType enum.
///
/// Note: Protobuf default value 0 has no named variant in the proto definition.
/// If a KeyContainer has key_type == 0, it should be treated as an unknown type
/// and processed (decrypted, stored) but not included in the CONTENT key output.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeyType {
    Signing = 1,
    Content = 2,
    KeyControl = 3,
    OperatorSession = 4,
    Entitlement = 5,
    OemContent = 6,
}

impl std::fmt::Display for KeyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Signing => write!(f, "SIGNING"),
            Self::Content => write!(f, "CONTENT"),
            Self::KeyControl => write!(f, "KEY_CONTROL"),
            Self::OperatorSession => write!(f, "OPERATOR_SESSION"),
            Self::Entitlement => write!(f, "ENTITLEMENT"),
            Self::OemContent => write!(f, "OEM_CONTENT"),
        }
    }
}

impl KeyType {
    pub const fn from_u8(u: u8) -> Option<Self> {
        match u {
            1 => Some(Self::Signing),
            2 => Some(Self::Content),
            3 => Some(Self::KeyControl),
            4 => Some(Self::OperatorSession),
            5 => Some(Self::Entitlement),
            6 => Some(Self::OemContent),
            _ => None,
        }
    }

    pub const fn to_u8(self) -> u8 {
        self as u8
    }
}

/// Proto KeyType (nested inside License.KeyContainer).
type ProtoKeyType = wdv3_proto::license::key_container::KeyType;

impl From<ProtoKeyType> for KeyType {
    fn from(proto: ProtoKeyType) -> Self {
        match proto {
            ProtoKeyType::Signing => Self::Signing,
            ProtoKeyType::Content => Self::Content,
            ProtoKeyType::KeyControl => Self::KeyControl,
            ProtoKeyType::OperatorSession => Self::OperatorSession,
            ProtoKeyType::Entitlement => Self::Entitlement,
            ProtoKeyType::OemContent => Self::OemContent,
        }
    }
}

impl From<KeyType> for ProtoKeyType {
    fn from(kt: KeyType) -> Self {
        match kt {
            KeyType::Signing => Self::Signing,
            KeyType::Content => Self::Content,
            KeyType::KeyControl => Self::KeyControl,
            KeyType::OperatorSession => Self::OperatorSession,
            KeyType::Entitlement => Self::Entitlement,
            KeyType::OemContent => Self::OemContent,
        }
    }
}

/// Widevine license type.
/// Ref: license_protocol.proto, LicenseType enum.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LicenseType {
    /// Normal one-time-use license for streaming content.
    #[default]
    Streaming,
    /// Offline-use license, usually for downloaded content.
    Offline,
    /// License type decision is left to the provider.
    Automatic,
}

type ProtoLicenseType = wdv3_proto::LicenseType;

impl From<LicenseType> for ProtoLicenseType {
    fn from(lt: LicenseType) -> Self {
        match lt {
            LicenseType::Streaming => Self::Streaming,
            LicenseType::Offline => Self::Offline,
            LicenseType::Automatic => Self::Automatic,
        }
    }
}

impl From<ProtoLicenseType> for LicenseType {
    fn from(proto: ProtoLicenseType) -> Self {
        match proto {
            ProtoLicenseType::Streaming => Self::Streaming,
            ProtoLicenseType::Offline => Self::Offline,
            ProtoLicenseType::Automatic => Self::Automatic,
        }
    }
}

/// A content decryption key extracted from a license response.
///
/// `Display` prints `kid_hex:key_hex` (e.g. `00000000000000000000000000000001:abcdef0123456789`).
/// `Debug` prints `[CONTENT] kid_hex:key_hex` (prefixed with the key type).
#[derive(Clone)]
pub struct ContentKey {
    /// Key ID: 16 bytes, from KeyContainer.id (proto field 1),
    /// normalized via kid_to_uuid conversion (see parse_license_response step 8c).
    pub kid: [u8; 16],
    /// Decrypted content key from KeyContainer.key (proto field 3)
    /// after AES-CBC decryption with enc_key and KeyContainer.iv (proto field 2),
    /// then PKCS#7 unpadding. Typically 16 bytes for AES-128 content, but the
    /// protocol does not constrain key length — Vec<u8> is used intentionally.
    pub key: Vec<u8>,
    /// Key type from KeyContainer.type (proto field 4).
    /// All types are decrypted and stored; consumers typically filter to CONTENT for output.
    pub key_type: KeyType,
}

impl ContentKey {
    /// Key ID as a lowercase hex string.
    pub fn kid_hex(&self) -> String {
        hex::encode(self.kid)
    }

    /// Decrypted key as a lowercase hex string.
    pub fn key_hex(&self) -> String {
        hex::encode(&self.key)
    }
}

impl std::fmt::Display for ContentKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", hex::encode(self.kid), hex::encode(&self.key))
    }
}

impl std::fmt::Debug for ContentKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {}:{}",
            self.key_type,
            hex::encode(self.kid),
            hex::encode(&self.key),
        )
    }
}

/// The three derived keys from a session key.
pub(crate) struct DerivedKeys {
    /// 16 bytes. AES-CMAC(session_key, 0x01 || enc_context).
    /// Used to decrypt KeyContainer.key fields.
    pub(crate) enc_key: [u8; 16],
    /// 32 bytes. CMAC(session_key, 0x01 || mac_context) || CMAC(session_key, 0x02 || mac_context).
    /// Used to verify license response signature via HMAC-SHA256.
    pub(crate) mac_key_server: [u8; 32],
    /// 32 bytes. CMAC(session_key, 0x03 || mac_context) || CMAC(session_key, 0x04 || mac_context).
    /// Used for license renewal requests.
    #[allow(dead_code)]
    pub(crate) mac_key_client: [u8; 32],
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    fn sample_key() -> ContentKey {
        ContentKey {
            kid: hex!("00000000000000000000000000000001"),
            key: vec![0xab, 0xcd, 0xef, 0x01],
            key_type: KeyType::Content,
        }
    }

    #[test]
    fn content_key_display() {
        let key = sample_key();
        let s = format!("{key}");
        assert_eq!(s, "00000000000000000000000000000001:abcdef01");
    }

    #[test]
    fn content_key_debug() {
        let key = sample_key();
        let s = format!("{key:?}");
        assert_eq!(s, "[CONTENT] 00000000000000000000000000000001:abcdef01");
    }

    #[test]
    fn content_key_debug_signing() {
        let key = ContentKey {
            kid: [0xFF; 16],
            key: vec![0x00],
            key_type: KeyType::Signing,
        };
        let s = format!("{key:?}");
        assert!(s.starts_with("[SIGNING]"));
    }

    #[test]
    fn content_key_hex_accessors() {
        let key = sample_key();
        assert_eq!(key.kid_hex(), "00000000000000000000000000000001");
        assert_eq!(key.key_hex(), "abcdef01");
    }

    #[test]
    fn key_type_display() {
        assert_eq!(format!("{}", KeyType::Content), "CONTENT");
        assert_eq!(format!("{}", KeyType::Signing), "SIGNING");
        assert_eq!(format!("{}", KeyType::KeyControl), "KEY_CONTROL");
        assert_eq!(format!("{}", KeyType::OperatorSession), "OPERATOR_SESSION");
        assert_eq!(format!("{}", KeyType::Entitlement), "ENTITLEMENT");
        assert_eq!(format!("{}", KeyType::OemContent), "OEM_CONTENT");
    }

    #[test]
    fn device_type_round_trip() {
        for val in [1u8, 2] {
            let dt = DeviceType::from_u8(val).unwrap();
            assert_eq!(dt.to_u8(), val);
        }
        assert!(DeviceType::from_u8(0).is_none());
        assert!(DeviceType::from_u8(3).is_none());
    }

    #[test]
    fn security_level_round_trip() {
        for val in [1u8, 2, 3] {
            let sl = SecurityLevel::from_u8(val).unwrap();
            assert_eq!(sl.to_u8(), val);
        }
        assert!(SecurityLevel::from_u8(0).is_none());
        assert!(SecurityLevel::from_u8(4).is_none());
    }

    #[test]
    fn license_type_default_is_streaming() {
        assert_eq!(LicenseType::default(), LicenseType::Streaming);
    }

    #[test]
    fn key_type_proto_round_trip() {
        let variants = [
            KeyType::Signing,
            KeyType::Content,
            KeyType::KeyControl,
            KeyType::OperatorSession,
            KeyType::Entitlement,
            KeyType::OemContent,
        ];
        for kt in variants {
            let proto: ProtoKeyType = kt.into();
            let back: KeyType = proto.into();
            assert_eq!(back, kt);
        }
    }
}
