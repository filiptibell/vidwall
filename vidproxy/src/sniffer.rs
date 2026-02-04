use std::time::Duration;

use anyhow::Result;
use chrome_browser::{ChromeBrowser, ChromeLaunchOptions};
use tokio::sync::watch;

use crate::credentials::{CredentialsSender, StreamCredentials};
use crate::manifest::{self, Manifest};

/// DRM sniffer that discovers stream credentials using Chrome browser automation.
pub struct DrmSniffer {
    manifest: Manifest,
    headless: bool,
    credentials_tx: CredentialsSender,
}

impl DrmSniffer {
    pub fn new(manifest: Manifest, headless: bool, credentials_tx: CredentialsSender) -> Self {
        Self {
            manifest,
            headless,
            credentials_tx,
        }
    }

    /// Run the sniffer loop. Discovers credentials and publishes them.
    /// Re-discovers when refresh is requested.
    pub async fn run(
        &mut self,
        mut shutdown_rx: watch::Receiver<bool>,
        mut refresh_rx: watch::Receiver<bool>,
    ) -> Result<()> {
        loop {
            // Check for shutdown
            if *shutdown_rx.borrow() {
                println!("[sniffer] Shutdown requested");
                break;
            }

            // Attempt to discover credentials
            match self.discover_credentials(&mut shutdown_rx).await {
                Ok(Some(credentials)) => {
                    println!("[sniffer] Credentials discovered successfully");
                    println!(
                        "[sniffer] MPD URL: {}...",
                        &credentials.mpd_url[..credentials.mpd_url.len().min(60)]
                    );
                    let _ = self.credentials_tx.send(Some(credentials));

                    // Wait for refresh request or shutdown
                    loop {
                        tokio::select! {
                            _ = shutdown_rx.changed() => {
                                if *shutdown_rx.borrow() {
                                    println!("[sniffer] Shutdown requested");
                                    return Ok(());
                                }
                            }
                            _ = refresh_rx.changed() => {
                                if *refresh_rx.borrow() {
                                    println!("[sniffer] Refresh requested, re-discovering...");
                                    break;
                                }
                            }
                        }
                    }
                }
                Ok(None) => {
                    // Shutdown requested during discovery
                    println!("[sniffer] Shutdown during discovery");
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("[sniffer] Discovery failed: {}", e);
                    // Wait before retrying
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_secs(10)) => {}
                        _ = shutdown_rx.changed() => {
                            if *shutdown_rx.borrow() {
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Discover stream credentials by executing the manifest.
    /// Returns None if shutdown was requested during discovery.
    async fn discover_credentials(
        &self,
        shutdown_rx: &mut watch::Receiver<bool>,
    ) -> Result<Option<StreamCredentials>> {
        println!("[sniffer] Launching Chrome...");

        let mut options = ChromeLaunchOptions::default()
            .headless(self.headless)
            .devtools(false);

        if let Some(ref proxy) = self.manifest.channel.proxy {
            options = options.proxy_server(proxy);
        }

        let browser = ChromeBrowser::new(options).await?;

        // Execute the manifest with shutdown monitoring
        let outputs = tokio::select! {
            result = manifest::execute(&self.manifest, &browser) => {
                let _ = browser.close().await;
                result?
            }
            _ = shutdown_rx.changed() => {
                println!("[sniffer] Shutdown during discovery, closing browser...");
                let _ = browser.close().await;
                return Ok(None);
            }
        };

        Ok(Some(StreamCredentials {
            mpd_url: outputs.mpd_url,
            decryption_key: outputs.decryption_key,
            license_url: String::new(), // Not needed for now
            pssh: String::new(),        // Not needed for now
        }))
    }
}
