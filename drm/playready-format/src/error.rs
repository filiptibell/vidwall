use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("invalid magic: expected {expected}, got {got}")]
    InvalidMagic { expected: &'static str, got: String },

    #[error("unexpected end of data: need {needed} bytes, have {have}")]
    UnexpectedEof { needed: usize, have: usize },

    #[error("unsupported version: {0}")]
    UnsupportedVersion(u8),

    #[error("invalid enum value {value} for {kind}")]
    InvalidEnumValue { kind: &'static str, value: u16 },

    #[error("malformed structure: {0}")]
    Malformed(String),

    #[error("invalid UTF-16: {0}")]
    InvalidUtf16(String),

    #[error("invalid XML: {0}")]
    InvalidXml(String),
}
