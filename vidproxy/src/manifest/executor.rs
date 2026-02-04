use anyhow::{Result, anyhow};
use chrome_browser::{ChromeBrowser, ChromeBrowserTab, NetworkRequestStream};
use regex::Regex;

use super::extractors::extract;
use super::interpolate::InterpolationContext;
use super::types::{Manifest, ManifestOutputs, Step, StepKind};

/**
    Execute a manifest using the given browser.
*/
pub async fn execute(manifest: &Manifest, browser: &ChromeBrowser) -> Result<ManifestOutputs> {
    let tab = browser
        .get_tab(0)
        .await
        .ok_or_else(|| anyhow!("No browser tab available"))?;

    let mut context = InterpolationContext::new();

    // Start monitoring network requests
    let mut requests = tab.network().requests();

    for step in &manifest.steps {
        println!("[executor] Running step: {}", step.name);

        match step.kind {
            StepKind::Navigate => {
                execute_navigate(step, &tab, &context).await?;
            }
            StepKind::Sniff => {
                execute_sniff(step, &mut requests, &mut context).await?;
            }
        }
    }

    // Resolve final outputs
    let channel_name = match &manifest.outputs.channel_name {
        Some(name) => context.interpolate(name)?,
        None => manifest.channel.name.clone(),
    };
    let mpd_url = context.interpolate(&manifest.outputs.mpd_url)?;
    let license_url = manifest
        .outputs
        .license_url
        .as_ref()
        .map(|l| context.interpolate(l))
        .transpose()?;
    let thumbnail_url = manifest
        .outputs
        .thumbnail_url
        .as_ref()
        .map(|t| context.interpolate(t))
        .transpose()?;
    let expires_at = manifest
        .outputs
        .expires_at
        .as_ref()
        .map(|e| context.interpolate(e))
        .transpose()?
        .and_then(|s| s.parse::<u64>().ok());

    Ok(ManifestOutputs {
        channel_name,
        mpd_url,
        license_url,
        thumbnail_url,
        expires_at,
    })
}

/**
    Execute a Navigate step.
*/
async fn execute_navigate(
    step: &Step,
    tab: &ChromeBrowserTab,
    context: &InterpolationContext,
) -> Result<()> {
    let url_template = step
        .url
        .as_ref()
        .ok_or_else(|| anyhow!("Navigate step '{}' requires 'url'", step.name))?;

    let url = context.interpolate(url_template)?;
    println!("[executor] Navigating to: {}", url);
    tab.navigate(&url).await?;

    // Wait for condition if specified
    if let Some(wait_for) = &step.wait_for {
        if let Some(selector) = &wait_for.selector {
            println!("[executor] Waiting for selector: {}", selector);
            tab.wait_for_selector(selector).await?;
        }
        if let Some(expr) = &wait_for.function {
            println!("[executor] Waiting for function: {}", expr);
            tab.wait_for_function(expr).await?;
        }
        if let Some(delay) = wait_for.delay {
            println!("[executor] Waiting {} seconds", delay);
            tokio::time::sleep(std::time::Duration::from_secs_f64(delay)).await;
        }
    }

    Ok(())
}

/**
    Execute a Sniff step.
*/
async fn execute_sniff(
    step: &Step,
    requests: &mut NetworkRequestStream,
    context: &mut InterpolationContext,
) -> Result<()> {
    use std::time::Duration;

    let request_match = step
        .request
        .as_ref()
        .ok_or_else(|| anyhow!("Sniff step '{}' requires 'request'", step.name))?;

    let url_pattern = &request_match.url;
    let method_filter = request_match.method.as_deref();
    let timeout_secs = request_match.timeout.unwrap_or(30.0);

    let url_regex = Regex::new(url_pattern)
        .map_err(|e| anyhow!("Invalid URL regex '{}': {}", url_pattern, e))?;

    println!(
        "[executor] Waiting for request matching: {} (timeout: {}s)",
        url_pattern, timeout_secs
    );

    let deadline = tokio::time::Instant::now() + Duration::from_secs_f64(timeout_secs);

    // Wait for matching request
    loop {
        let next_request = tokio::time::timeout_at(deadline, requests.next()).await;

        let request = match next_request {
            Ok(Some(req)) => req,
            Ok(None) => {
                return Err(anyhow!(
                    "Network stream closed before finding match for step '{}'",
                    step.name
                ));
            }
            Err(_) => {
                return Err(anyhow!(
                    "Timeout waiting for request matching '{}' in step '{}'",
                    url_pattern,
                    step.name
                ));
            }
        };

        let url = request.url().to_string();
        let method = request.method();

        // Check URL pattern (regex)
        if !url_regex.is_match(&url) {
            continue;
        }

        // Check method filter
        if let Some(expected_method) = method_filter
            && method.as_str() != expected_method
        {
            continue;
        }

        println!("[executor] Matched request: {}", &url[..url.len().min(80)]);

        // Get response body
        let body = if let Ok(response) = request.response().await {
            response.text().await.unwrap_or_default()
        } else {
            String::new()
        };

        // Run extractors - all must succeed for this request to be accepted
        let mut extracted = std::collections::HashMap::new();
        let mut all_succeeded = true;

        for (output_name, extractor) in &step.extract {
            match extract(extractor, &body, &url) {
                Ok(value) => {
                    extracted.insert(output_name.clone(), value);
                }
                Err(_) => {
                    all_succeeded = false;
                    break;
                }
            }
        }

        if all_succeeded {
            // All extractors succeeded, commit the results
            for (output_name, value) in extracted {
                println!("[executor] Extracted {}.{}", step.name, output_name);
                context.set(&step.name, &output_name, value);
            }
            return Ok(());
        }

        // Extraction failed, try next matching request
        println!("[executor] Extraction failed, trying next request...");
    }
}
