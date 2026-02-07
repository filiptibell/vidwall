use core::fmt;

use crate::error::FormatError;

/**
    Content encryption algorithm used by a PlayReady content key.
*/
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    pub const fn name(self) -> &'static str {
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

impl TryFrom<u16> for KeyType {
    type Error = FormatError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0x0000 => Ok(Self::Invalid),
            0x0001 => Ok(Self::Aes128Ctr),
            0x0002 => Ok(Self::Rc4Cipher),
            0x0003 => Ok(Self::Aes128Ecb),
            0x0004 => Ok(Self::Cocktail),
            0x0005 => Ok(Self::Aes128Cbc),
            0x0006 => Ok(Self::KeyExchange),
            _ => Err(FormatError::InvalidEnumValue {
                kind: "KeyType",
                value,
            }),
        }
    }
}

impl fmt::Display for KeyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/**
    Key wrapping cipher used to encrypt content keys in XMR licenses.
*/
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    pub const fn name(self) -> &'static str {
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

impl TryFrom<u16> for CipherType {
    type Error = FormatError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0x0000 => Ok(Self::Invalid),
            0x0001 => Ok(Self::Rsa1024),
            0x0002 => Ok(Self::ChainedLicense),
            0x0003 => Ok(Self::Ecc256),
            0x0004 => Ok(Self::Ecc256WithKz),
            0x0005 => Ok(Self::TeeTransient),
            0x0006 => Ok(Self::Ecc256ViaSymmetric),
            _ => Err(FormatError::InvalidEnumValue {
                kind: "CipherType",
                value,
            }),
        }
    }
}

impl fmt::Display for CipherType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
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
            let v = kt as u16;
            assert_eq!(KeyType::try_from(v).unwrap(), kt);
        }
    }

    #[test]
    fn key_type_invalid_value() {
        assert!(KeyType::try_from(0x0007).is_err());
        assert!(KeyType::try_from(0xFFFF).is_err());
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
            let v = ct as u16;
            assert_eq!(CipherType::try_from(v).unwrap(), ct);
        }
    }

    #[test]
    fn cipher_type_invalid_value() {
        assert!(CipherType::try_from(0x0007).is_err());
        assert!(CipherType::try_from(0xFFFF).is_err());
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
}
