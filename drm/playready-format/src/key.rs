use core::fmt;
use core::str::FromStr;

use drm_core::{ParseError, eq_ignore_ascii_case, trim_ascii};

/**
    Content encryption algorithm used by a PlayReady content key.
*/
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeyType {
    Invalid = 0x0000,
    Aes128Ctr = 0x0001,
    Rc4Cipher = 0x0002,
    Aes128Ecb = 0x0003,
    Cocktail = 0x0004,
    Aes128Cbc = 0x0005,
    KeyExchange = 0x0006,
}

impl KeyType {
    pub const fn from_u16(u: u16) -> Option<Self> {
        match u {
            0x0000 => Some(Self::Invalid),
            0x0001 => Some(Self::Aes128Ctr),
            0x0002 => Some(Self::Rc4Cipher),
            0x0003 => Some(Self::Aes128Ecb),
            0x0004 => Some(Self::Cocktail),
            0x0005 => Some(Self::Aes128Cbc),
            0x0006 => Some(Self::KeyExchange),
            _ => None,
        }
    }

    pub const fn to_u16(self) -> u16 {
        self as u16
    }

    pub const fn from_name(name: &[u8]) -> Option<Self> {
        let name = trim_ascii(name);
        match name.len() {
            7 if eq_ignore_ascii_case(name, b"INVALID") => Some(Self::Invalid),
            11 if eq_ignore_ascii_case(name, b"AES_128_CTR") => Some(Self::Aes128Ctr),
            10 if eq_ignore_ascii_case(name, b"RC4_CIPHER") => Some(Self::Rc4Cipher),
            11 if eq_ignore_ascii_case(name, b"AES_128_ECB") => Some(Self::Aes128Ecb),
            8 if eq_ignore_ascii_case(name, b"COCKTAIL") => Some(Self::Cocktail),
            11 if eq_ignore_ascii_case(name, b"AES_128_CBC") => Some(Self::Aes128Cbc),
            12 if eq_ignore_ascii_case(name, b"KEY_EXCHANGE") => Some(Self::KeyExchange),
            _ => None,
        }
    }

    pub const fn to_name(self) -> &'static str {
        match self {
            Self::Invalid => "INVALID",
            Self::Aes128Ctr => "AES_128_CTR",
            Self::Rc4Cipher => "RC4_CIPHER",
            Self::Aes128Ecb => "AES_128_ECB",
            Self::Cocktail => "COCKTAIL",
            Self::Aes128Cbc => "AES_128_CBC",
            Self::KeyExchange => "KEY_EXCHANGE",
        }
    }
}

impl fmt::Display for KeyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_name())
    }
}

impl FromStr for KeyType {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_name(s.as_bytes()).ok_or_else(|| ParseError {
            kind: "key type",
            value: s.to_owned(),
        })
    }
}

/**
    Key wrapping cipher used to encrypt content keys in XMR licenses.
*/
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CipherType {
    Invalid = 0x0000,
    Rsa1024 = 0x0001,
    ChainedLicense = 0x0002,
    Ecc256 = 0x0003,
    Ecc256WithKz = 0x0004,
    TeeTransient = 0x0005,
    Ecc256ViaSymmetric = 0x0006,
}

impl CipherType {
    pub const fn from_u16(u: u16) -> Option<Self> {
        match u {
            0x0000 => Some(Self::Invalid),
            0x0001 => Some(Self::Rsa1024),
            0x0002 => Some(Self::ChainedLicense),
            0x0003 => Some(Self::Ecc256),
            0x0004 => Some(Self::Ecc256WithKz),
            0x0005 => Some(Self::TeeTransient),
            0x0006 => Some(Self::Ecc256ViaSymmetric),
            _ => None,
        }
    }

    pub const fn to_u16(self) -> u16 {
        self as u16
    }

    pub const fn from_name(name: &[u8]) -> Option<Self> {
        let name = trim_ascii(name);
        match name.len() {
            7 if eq_ignore_ascii_case(name, b"INVALID") => Some(Self::Invalid),
            8 if eq_ignore_ascii_case(name, b"RSA_1024") => Some(Self::Rsa1024),
            15 if eq_ignore_ascii_case(name, b"CHAINED_LICENSE") => Some(Self::ChainedLicense),
            7 if eq_ignore_ascii_case(name, b"ECC_256") => Some(Self::Ecc256),
            15 if eq_ignore_ascii_case(name, b"ECC_256_WITH_KZ") => Some(Self::Ecc256WithKz),
            13 if eq_ignore_ascii_case(name, b"TEE_TRANSIENT") => Some(Self::TeeTransient),
            21 if eq_ignore_ascii_case(name, b"ECC_256_VIA_SYMMETRIC") => {
                Some(Self::Ecc256ViaSymmetric)
            }
            _ => None,
        }
    }

    pub const fn to_name(self) -> &'static str {
        match self {
            Self::Invalid => "INVALID",
            Self::Rsa1024 => "RSA_1024",
            Self::ChainedLicense => "CHAINED_LICENSE",
            Self::Ecc256 => "ECC_256",
            Self::Ecc256WithKz => "ECC_256_WITH_KZ",
            Self::TeeTransient => "TEE_TRANSIENT",
            Self::Ecc256ViaSymmetric => "ECC_256_VIA_SYMMETRIC",
        }
    }
}

impl fmt::Display for CipherType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_name())
    }
}

impl FromStr for CipherType {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_name(s.as_bytes()).ok_or_else(|| ParseError {
            kind: "cipher type",
            value: s.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_type_round_trip() {
        let variants = [
            KeyType::Invalid,
            KeyType::Aes128Ctr,
            KeyType::Rc4Cipher,
            KeyType::Aes128Ecb,
            KeyType::Cocktail,
            KeyType::Aes128Cbc,
            KeyType::KeyExchange,
        ];
        for kt in variants {
            let v = kt.to_u16();
            assert_eq!(KeyType::from_u16(v), Some(kt));
        }
    }

    #[test]
    fn key_type_invalid_value() {
        assert!(KeyType::from_u16(0x0007).is_none());
        assert!(KeyType::from_u16(0xFFFF).is_none());
    }

    #[test]
    fn cipher_type_round_trip() {
        let variants = [
            CipherType::Invalid,
            CipherType::Rsa1024,
            CipherType::ChainedLicense,
            CipherType::Ecc256,
            CipherType::Ecc256WithKz,
            CipherType::TeeTransient,
            CipherType::Ecc256ViaSymmetric,
        ];
        for ct in variants {
            let v = ct.to_u16();
            assert_eq!(CipherType::from_u16(v), Some(ct));
        }
    }

    #[test]
    fn cipher_type_invalid_value() {
        assert!(CipherType::from_u16(0x0007).is_none());
        assert!(CipherType::from_u16(0xFFFF).is_none());
    }

    #[test]
    fn key_type_name_round_trip() {
        for kt in [
            KeyType::Invalid,
            KeyType::Aes128Ctr,
            KeyType::Rc4Cipher,
            KeyType::Aes128Ecb,
            KeyType::Cocktail,
            KeyType::Aes128Cbc,
            KeyType::KeyExchange,
        ] {
            let name = kt.to_name();
            let parsed: KeyType = name.parse().unwrap();
            assert_eq!(parsed, kt);
        }
    }

    #[test]
    fn key_type_from_name_case_insensitive() {
        assert_eq!(KeyType::from_name(b"aes_128_ctr"), Some(KeyType::Aes128Ctr));
        assert_eq!(KeyType::from_name(b"AES_128_CTR"), Some(KeyType::Aes128Ctr));
        assert_eq!(KeyType::from_name(b"Cocktail"), Some(KeyType::Cocktail));
        assert_eq!(KeyType::from_name(b"unknown"), None);
    }

    #[test]
    fn cipher_type_name_round_trip() {
        for ct in [
            CipherType::Invalid,
            CipherType::Rsa1024,
            CipherType::ChainedLicense,
            CipherType::Ecc256,
            CipherType::Ecc256WithKz,
            CipherType::TeeTransient,
            CipherType::Ecc256ViaSymmetric,
        ] {
            let name = ct.to_name();
            let parsed: CipherType = name.parse().unwrap();
            assert_eq!(parsed, ct);
        }
    }

    #[test]
    fn cipher_type_from_name_case_insensitive() {
        assert_eq!(CipherType::from_name(b"ecc_256"), Some(CipherType::Ecc256));
        assert_eq!(
            CipherType::from_name(b"ECC_256_VIA_SYMMETRIC"),
            Some(CipherType::Ecc256ViaSymmetric)
        );
        assert_eq!(CipherType::from_name(b"unknown"), None);
    }

    #[test]
    fn display() {
        assert_eq!(KeyType::Aes128Ctr.to_string(), "AES_128_CTR");
        assert_eq!(CipherType::Ecc256.to_string(), "ECC_256");
        assert_eq!(
            CipherType::Ecc256ViaSymmetric.to_string(),
            "ECC_256_VIA_SYMMETRIC"
        );
    }

    #[test]
    fn from_str_invalid() {
        assert!("BAD".parse::<KeyType>().is_err());
        assert!("BAD".parse::<CipherType>().is_err());
    }
}
