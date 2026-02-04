use tokio::sync::watch;

/// Credentials needed to access a DRM-protected stream.
#[derive(Clone, Debug)]
pub struct StreamCredentials {
    /// The DASH/HLS manifest URL
    pub mpd_url: String,
    /// Decryption key in "key_id:key" format
    pub decryption_key: String,
    /// License server URL (for potential refresh)
    pub license_url: String,
    /// PSSH box in base64 (for potential refresh)
    pub pssh: String,
}

pub type CredentialsReceiver = watch::Receiver<Option<StreamCredentials>>;
pub type CredentialsSender = watch::Sender<Option<StreamCredentials>>;

/// Create a new credentials channel pair.
pub fn credentials_channel() -> (CredentialsSender, CredentialsReceiver) {
    watch::channel(None)
}
