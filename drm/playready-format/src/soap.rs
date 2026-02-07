/*!
    SOAP/XML namespace constants and algorithm URIs for PlayReady license acquisition.
*/

/**
    SOAP 1.1 namespace.
*/
pub const SOAP_NS: &str = "http://schemas.xmlsoap.org/soap/envelope/";

/**
    PlayReady protocol namespace.
*/
pub const PROTOCOL_NS: &str = "http://schemas.microsoft.com/DRM/2007/03/protocols";

/**
    PlayReady message namespace.
*/
pub const MESSAGE_NS: &str = "http://schemas.microsoft.com/DRM/2007/03/protocols/messages";

/**
    XML Digital Signature namespace.
*/
pub const XMLDSIG_NS: &str = "http://www.w3.org/2000/09/xmldsig#";

/**
    XML Encryption namespace.
*/
pub const XMLENC_NS: &str = "http://www.w3.org/2001/04/xmlenc#";

/**
    PlayReady ECC-256 encryption algorithm URI.
*/
pub const ECC256_ALGORITHM: &str = "http://schemas.microsoft.com/DRM/2007/03/protocols#ecc256";

/**
    PlayReady ECDSA-SHA256 signature algorithm URI.
*/
pub const ECDSA_SHA256_ALGORITHM: &str =
    "http://schemas.microsoft.com/DRM/2007/03/protocols#ecdsa-sha256";

/**
    PlayReady SHA-256 digest algorithm URI.
*/
pub const SHA256_ALGORITHM: &str = "http://schemas.microsoft.com/DRM/2007/03/protocols#sha256";

/**
    C14N canonicalization algorithm URI.
*/
pub const C14N_ALGORITHM: &str = "http://www.w3.org/TR/2001/REC-xml-c14n-20010315";

/**
    AES-128-CBC encryption algorithm URI.
*/
pub const AES128_CBC_ALGORITHM: &str = "http://www.w3.org/2001/04/xmlenc#aes128-cbc";

/**
    Client version string included in license challenges.
*/
pub const CLIENT_VERSION: &str = "10.0.16384.10011";
