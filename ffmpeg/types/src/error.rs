/*!
    Error types for the ffmpeg crate ecosystem.
*/

use std::fmt;

/**
    Error type for the ffmpeg crate ecosystem.
*/
#[derive(Debug)]
pub enum Error {
    /// I/O error (file not found, network error, etc.)
    Io(std::io::Error),
    /// Codec error (decode/encode failure)
    Codec { message: String },
    /// Invalid data (malformed input)
    InvalidData { message: String },
    /// Unsupported format (valid but not handled)
    UnsupportedFormat { message: String },
    /// End of stream (not really an error, but part of control flow)
    Eof,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Codec { message } => write!(f, "codec error: {message}"),
            Self::InvalidData { message } => write!(f, "invalid data: {message}"),
            Self::UnsupportedFormat { message } => write!(f, "unsupported format: {message}"),
            Self::Eof => write!(f, "end of stream"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl Error {
    /**
        Create a codec error with the given message.
    */
    pub fn codec(message: impl Into<String>) -> Self {
        Self::Codec {
            message: message.into(),
        }
    }

    /**
        Create an invalid data error with the given message.
    */
    pub fn invalid_data(message: impl Into<String>) -> Self {
        Self::InvalidData {
            message: message.into(),
        }
    }

    /**
        Create an unsupported format error with the given message.
    */
    pub fn unsupported_format(message: impl Into<String>) -> Self {
        Self::UnsupportedFormat {
            message: message.into(),
        }
    }

    /**
        Returns true if this is an EOF error.
    */
    pub fn is_eof(&self) -> bool {
        matches!(self, Self::Eof)
    }
}

/**
    Result type alias for the ffmpeg crate ecosystem.
*/
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as StdError;

    #[test]
    fn error_display() {
        let e = Error::codec("decode failed");
        assert_eq!(format!("{e}"), "codec error: decode failed");

        let e = Error::invalid_data("corrupted header");
        assert_eq!(format!("{e}"), "invalid data: corrupted header");

        let e = Error::unsupported_format("unknown codec");
        assert_eq!(format!("{e}"), "unsupported format: unknown codec");

        let e = Error::Eof;
        assert_eq!(format!("{e}"), "end of stream");
    }

    #[test]
    fn error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let e: Error = io_err.into();
        assert!(matches!(e, Error::Io(_)));
        assert!(format!("{e}").contains("file not found"));
    }

    #[test]
    fn error_is_eof() {
        assert!(Error::Eof.is_eof());
        assert!(!Error::codec("test").is_eof());
    }

    #[test]
    fn error_source() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let e = Error::Io(io_err);
        assert!(StdError::source(&e).is_some());

        let e = Error::Eof;
        assert!(StdError::source(&e).is_none());
    }
}
