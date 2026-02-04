use std::time::Duration;

use anyhow::{Result, anyhow};
use base64::{Engine, prelude::BASE64_STANDARD};
use chrome_browser::{ChromeBrowser, ChromeLaunchOptions};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use crate::credentials::{CredentialsSender, StreamCredentials};

const DEFAULT_CDRM_API_URL: &str = "https://cdrm-project.com/api/decrypt";

#[derive(Debug, Deserialize)]
struct ApiResponse {
    result: Vec<ContentItem>,
}

#[derive(Debug, Deserialize)]
struct ContentItem {
    id: String,
    content: ContentInfo,
}

#[derive(Debug, Deserialize)]
struct ContentInfo {
    title: String,
}

#[derive(Debug, Serialize)]
struct CdrmRequest {
    pssh: String,
    licurl: String,
    headers: String,
}

#[derive(Debug, Deserialize)]
struct CdrmResponse {
    message: String,
}

/**
    Configuration for the DRM sniffer.
*/
#[derive(Clone, Debug)]
pub struct SnifferConfig {
    /// Target site URL (e.g., "https://www.canalrcn.com")
    pub site_url: String,
    /// Content title to search for (e.g., "Señal Principal")
    pub content_title: String,
    /// Optional SOCKS5 proxy for Chrome (e.g., "socks5://127.0.0.1:1080")
    pub proxy_server: Option<String>,
    /// Run Chrome in headless mode
    pub headless: bool,
    /// CDRM API URL for key extraction
    pub cdrm_api_url: String,
}

impl Default for SnifferConfig {
    fn default() -> Self {
        Self {
            site_url: "https://www.canalrcn.com".to_string(),
            content_title: "Señal Principal".to_string(),
            proxy_server: Some("socks5://127.0.0.1:1080".to_string()),
            headless: false,
            cdrm_api_url: DEFAULT_CDRM_API_URL.to_string(),
        }
    }
}

/**
    Extract PSSH box (base64) from MPD content.
*/
fn extract_pssh(mpd: &str) -> Option<String> {
    for line in mpd.lines() {
        if line.contains("cenc:pssh") || line.contains("<pssh>") {
            if let Some(start) = line.find('>') {
                if let Some(end) = line[start + 1..].find('<') {
                    let pssh = &line[start + 1..start + 1 + end];
                    if !pssh.is_empty()
                        && pssh
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '=')
                    {
                        return Some(pssh.to_string());
                    }
                }
            }
        }
    }
    None
}

/**
    Extract KID from decoded PSSH bytes.
*/
fn extract_kid(pssh_bytes: &[u8]) -> Option<String> {
    if pssh_bytes.len() >= 48 {
        Some(
            pssh_bytes[32..48]
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect(),
        )
    } else {
        None
    }
}

/**
    DRM sniffer that discovers stream credentials using Chrome browser automation.
*/
pub struct DrmSniffer {
    config: SnifferConfig,
    credentials_tx: CredentialsSender,
}

impl DrmSniffer {
    pub fn new(config: SnifferConfig, credentials_tx: CredentialsSender) -> Self {
        Self {
            config,
            credentials_tx,
        }
    }

    /**
        Run the sniffer loop. Discovers credentials and publishes them.
        Re-discovers when refresh is requested.
    */
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
            match self.discover_credentials().await {
                Ok(credentials) => {
                    println!("[sniffer] Credentials discovered successfully");
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

    /**
        Discover stream credentials by launching Chrome and sniffing network traffic.
    */
    async fn discover_credentials(&self) -> Result<StreamCredentials> {
        println!("[sniffer] Launching Chrome...");

        let mut options = ChromeLaunchOptions::default()
            .headless(self.config.headless)
            .devtools(false);

        if let Some(ref proxy) = self.config.proxy_server {
            options = options.proxy_server(proxy);
        }

        let browser = ChromeBrowser::new(options).await?;
        let tab = browser
            .get_tab(0)
            .await
            .ok_or_else(|| anyhow!("No tab available"))?;

        // Start monitoring network requests
        let mut requests = tab.network().requests();

        // Phase 1: Navigate to site and find content ID
        println!("[sniffer] Navigating to: {}", self.config.site_url);
        tab.navigate(&self.config.site_url).await?;

        println!(
            "[sniffer] Looking for '{}' in API responses...",
            self.config.content_title
        );

        let content_id = loop {
            let Some(request) = requests.next().await else {
                browser.close().await?;
                return Err(anyhow!("Network stream closed before finding content"));
            };

            let url = request.url().to_string();

            if url.contains("unity.tbxapis.com") && url.contains("/items/") && url.contains(".json")
            {
                if let Ok(response) = request.response().await {
                    if let Ok(body) = response.text().await {
                        if let Ok(api_response) = serde_json::from_str::<ApiResponse>(&body) {
                            if let Some(item) = api_response
                                .result
                                .iter()
                                .find(|i| i.content.title == self.config.content_title)
                            {
                                println!("[sniffer] Found content ID: {}", item.id);
                                break item.id.clone();
                            }
                        }
                    }
                }
            }
        };

        // Phase 2: Navigate to player
        let player_url = format!(
            "{}/co/player/{}",
            self.config.site_url.trim_end_matches('/'),
            content_id
        );
        println!("[sniffer] Navigating to player: {}", player_url);
        tab.navigate(&player_url).await?;

        // Phase 3: Monitor for MPD and license requests
        println!("[sniffer] Monitoring for DRM requests...");

        let mut pssh: Option<String> = None;
        let mut mpd_url: Option<String> = None;
        let mut license_url: Option<String> = None;

        while let Some(request) = requests.next().await {
            let url = request.url().to_string();
            let method = request.method().clone();

            let is_license = url.contains("license") && url.contains("widevine");
            let is_mpd = url.contains(".mpd");

            if is_mpd && pssh.is_none() {
                if let Ok(response) = request.response().await {
                    if let Ok(body) = response.text().await {
                        if let Some(extracted_pssh) = extract_pssh(&body) {
                            println!("[sniffer] Found PSSH in MPD");
                            if let Ok(decoded) = BASE64_STANDARD.decode(&extracted_pssh) {
                                if let Some(kid) = extract_kid(&decoded) {
                                    println!("[sniffer] KID: {}", kid);
                                }
                            }
                            pssh = Some(extracted_pssh);
                            mpd_url = Some(url.clone());
                        }
                    }
                }
            } else if is_license && method == "POST" && license_url.is_none() {
                println!("[sniffer] Found license URL");
                license_url = Some(url.clone());

                // Once we have both, get the keys
                if let (Some(p), Some(l), Some(m)) = (&pssh, &license_url, &mpd_url) {
                    let decryption_key = self.fetch_decryption_key(p, l).await?;

                    browser.close().await?;

                    return Ok(StreamCredentials {
                        mpd_url: m.clone(),
                        decryption_key,
                        license_url: l.clone(),
                        pssh: p.clone(),
                    });
                }
            }
        }

        browser.close().await?;
        Err(anyhow!("Failed to discover all required DRM information"))
    }

    /**
        Fetch decryption key from CDRM API.
    */
    async fn fetch_decryption_key(&self, pssh: &str, license_url: &str) -> Result<String> {
        println!("[sniffer] Requesting decryption keys from CDRM API...");

        let client = reqwest::Client::new();
        let cdrm_req = CdrmRequest {
            pssh: pssh.to_string(),
            licurl: license_url.to_string(),
            headers: format!(
                "{:?}",
                std::collections::HashMap::from([
                    (
                        "User-Agent",
                        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36"
                    ),
                    ("Accept", "*/*"),
                    ("Origin", &self.config.site_url),
                    ("Referer", &format!("{}/", self.config.site_url)),
                ])
            ),
        };

        let resp = client
            .post(&self.config.cdrm_api_url)
            .json(&cdrm_req)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow!("CDRM API error: {}", resp.status()));
        }

        let cdrm_resp: CdrmResponse = resp.json().await?;

        // Parse keys (format: "kid:key\nkid:key\n...")
        // Return the first key pair in "kid:key" format
        let first_key = cdrm_resp
            .message
            .lines()
            .find(|l| l.contains(':'))
            .ok_or_else(|| anyhow!("No keys in CDRM response"))?;

        println!("[sniffer] Got decryption key");
        Ok(first_key.to_string())
    }
}
