use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

const CDRM_API_URL: &str = "https://cdrm-project.com/api/decrypt";

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

/// Extract PSSH from an MPD manifest
pub fn extract_pssh_from_mpd(mpd_url: &str, mpd_content: &str) -> Result<String> {
    use ffmpeg_source::reader::stream::StreamFormat;
    use ffmpeg_source::reader::stream::dash::DashFormat;

    let dash = DashFormat::from_manifest(mpd_url, mpd_content.as_bytes())
        .map_err(|e| anyhow!("Failed to parse MPD: {}", e))?;

    let drm_info = dash.drm_info();

    // Get Widevine PSSH first, fall back to any PSSH
    let pssh = drm_info
        .widevine_pssh()
        .into_iter()
        .next()
        .map(|p| &p.data_base64)
        .or_else(|| drm_info.pssh_boxes.first().map(|p| &p.data_base64))
        .ok_or_else(|| anyhow!("No PSSH found in MPD"))?;

    Ok(pssh.clone())
}

/// Fetch decryption key from CDRM API
pub async fn fetch_decryption_key(pssh: &str, license_url: &str) -> Result<String> {
    println!("[cdrm] Requesting decryption key from CDRM API...");

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
            ])
        ),
    };

    let resp = client.post(CDRM_API_URL).json(&cdrm_req).send().await?;

    if !resp.status().is_success() {
        return Err(anyhow!("CDRM API error: {}", resp.status()));
    }

    let cdrm_resp: CdrmResponse = resp.json().await?;

    // Extract first line containing ":" (the key format is "kid:key")
    let key = cdrm_resp
        .message
        .lines()
        .find(|line| line.contains(':'))
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("No decryption key found in CDRM response"))?;

    println!("[cdrm] Got decryption key");
    Ok(key)
}

/// Fetch MPD content and extract PSSH, then get decryption key
pub async fn get_decryption_key(mpd_url: &str, license_url: &str) -> Result<String> {
    println!("[cdrm] Fetching MPD to extract PSSH...");

    let client = reqwest::Client::new();
    let mpd_content = client.get(mpd_url).send().await?.text().await?;

    let pssh = extract_pssh_from_mpd(mpd_url, &mpd_content)?;
    println!("[cdrm] Extracted PSSH: {}...", &pssh[..pssh.len().min(30)]);

    fetch_decryption_key(&pssh, license_url).await
}
