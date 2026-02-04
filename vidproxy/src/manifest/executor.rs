use std::collections::HashMap;

use anyhow::{Result, anyhow};
use chrome_browser::{ChromeBrowserTab, NetworkRequestStream};
use regex::Regex;

use super::extractors::{ExtractedArray, extract, extract_array};
use super::interpolate::InterpolationContext;
use super::types::{ExtractorKind, Step, StepKind};

/**
    Execute a Navigate step.
*/
pub async fn execute_navigate(
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
    Result from executing a sniff step.
*/
pub enum SniffResult {
    /// Single values extracted (normal extractors)
    Single(HashMap<String, String>),
    /// Array of objects extracted (jsonpath_array extractor)
    Array { name: String, items: ExtractedArray },
}

/**
    Execute a Sniff step, returning extracted values.
*/
pub async fn execute_sniff(
    step: &Step,
    requests: &mut NetworkRequestStream,
    _context: &InterpolationContext,
) -> Result<SniffResult> {
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

    // Check if any extractor is jsonpath_array
    let has_array_extractor = step
        .extract
        .values()
        .any(|e| e.kind == ExtractorKind::JsonPathArray);

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

        // Handle array extractor specially
        if has_array_extractor {
            // Find the array extractor
            for (output_name, extractor) in &step.extract {
                if extractor.kind == ExtractorKind::JsonPathArray {
                    match extract_array(extractor, &body) {
                        Ok(items) => {
                            println!(
                                "[executor] Extracted {} items from {}.{}",
                                items.len(),
                                step.name,
                                output_name
                            );
                            return Ok(SniffResult::Array {
                                name: output_name.clone(),
                                items,
                            });
                        }
                        Err(e) => {
                            println!(
                                "[executor] Array extraction failed: {}, trying next request...",
                                e
                            );
                            break;
                        }
                    }
                }
            }
            continue;
        }

        // Run normal extractors - all must succeed for this request to be accepted
        let mut extracted = HashMap::new();
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
            for output_name in extracted.keys() {
                println!("[executor] Extracted {}.{}", step.name, output_name);
            }
            return Ok(SniffResult::Single(extracted));
        }

        // Extraction failed, try next matching request
        println!("[executor] Extraction failed, trying next request...");
    }
}

/**
    Execute a SniffMany step, collecting multiple matching requests and aggregating results.
*/
pub async fn execute_sniff_many(
    step: &Step,
    requests: &mut NetworkRequestStream,
    _context: &InterpolationContext,
) -> Result<SniffResult> {
    use std::time::Duration;

    let request_match = step
        .request
        .as_ref()
        .ok_or_else(|| anyhow!("SniffMany step '{}' requires 'request'", step.name))?;

    let url_pattern = &request_match.url;
    let method_filter = request_match.method.as_deref();
    let timeout_secs = request_match.timeout.unwrap_or(30.0);
    let idle_timeout_secs = request_match.idle_timeout.unwrap_or(2.0);

    let url_regex = Regex::new(url_pattern)
        .map_err(|e| anyhow!("Invalid URL regex '{}': {}", url_pattern, e))?;

    println!(
        "[executor] SniffMany: collecting requests matching: {} (timeout: {}s, idle: {}s)",
        url_pattern, timeout_secs, idle_timeout_secs
    );

    let deadline = tokio::time::Instant::now() + Duration::from_secs_f64(timeout_secs);
    let idle_duration = Duration::from_secs_f64(idle_timeout_secs);

    // Check if any extractor is jsonpath_array
    let has_array_extractor = step
        .extract
        .values()
        .any(|e| e.kind == ExtractorKind::JsonPathArray);

    // Collect all matching requests
    let mut all_items: ExtractedArray = Vec::new();
    let mut array_extractor_name: Option<String> = None;
    let mut match_count = 0;

    loop {
        // Use idle timeout for subsequent requests, but overall deadline still applies
        let wait_timeout = if match_count == 0 {
            deadline
        } else {
            let idle_deadline = tokio::time::Instant::now() + idle_duration;
            std::cmp::min(idle_deadline, deadline)
        };

        let next_request = tokio::time::timeout_at(wait_timeout, requests.next()).await;

        let request = match next_request {
            Ok(Some(req)) => req,
            Ok(None) => {
                // Stream closed
                break;
            }
            Err(_) => {
                // Timeout - if we have matches, we're done; otherwise it's an error
                if match_count > 0 {
                    println!(
                        "[executor] SniffMany: idle timeout, collected {} matches",
                        match_count
                    );
                    break;
                } else {
                    return Err(anyhow!(
                        "Timeout waiting for request matching '{}' in step '{}'",
                        url_pattern,
                        step.name
                    ));
                }
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

        println!(
            "[executor] SniffMany: matched request #{}: {}",
            match_count + 1,
            &url[..url.len().min(80)]
        );

        // Get response body
        let body = if let Ok(response) = request.response().await {
            response.text().await.unwrap_or_default()
        } else {
            continue;
        };

        // Handle array extractor - aggregate items from all responses
        if has_array_extractor {
            for (output_name, extractor) in &step.extract {
                if extractor.kind == ExtractorKind::JsonPathArray {
                    if array_extractor_name.is_none() {
                        array_extractor_name = Some(output_name.clone());
                    }
                    match extract_array(extractor, &body) {
                        Ok(items) => {
                            println!(
                                "[executor] SniffMany: extracted {} items from response",
                                items.len()
                            );
                            all_items.extend(items);
                            match_count += 1;
                        }
                        Err(e) => {
                            println!("[executor] SniffMany: extraction failed: {}", e);
                        }
                    }
                    break;
                }
            }
        }
    }

    if has_array_extractor && let Some(name) = array_extractor_name {
        println!(
            "[executor] SniffMany: total {} items from {} responses",
            all_items.len(),
            match_count
        );
        return Ok(SniffResult::Array {
            name,
            items: all_items,
        });
    }

    Err(anyhow!(
        "SniffMany step '{}' requires a jsonpath_array extractor",
        step.name
    ))
}

/**
    Execute a list of steps, returning the interpolation context.
    This is used by both discovery and content phases.
*/
pub async fn execute_steps(
    steps: &[Step],
    tab: &ChromeBrowserTab,
    initial_context: InterpolationContext,
) -> Result<(InterpolationContext, Option<(String, ExtractedArray)>)> {
    let mut context = initial_context;
    let mut requests = tab.network().requests();
    let mut array_result: Option<(String, ExtractedArray)> = None;

    for step in steps {
        println!("[executor] Running step: {}", step.name);

        match step.kind {
            StepKind::Navigate => {
                execute_navigate(step, tab, &context).await?;
            }
            StepKind::Sniff => {
                match execute_sniff(step, &mut requests, &context).await? {
                    SniffResult::Single(values) => {
                        for (output_name, value) in values {
                            context.set(&step.name, &output_name, value);
                        }
                    }
                    SniffResult::Array { name, items } => {
                        // Store array result for later processing
                        // The step.name and extractor name form the reference
                        array_result = Some((format!("{}.{}", step.name, name), items));
                    }
                }
            }
            StepKind::SniffMany => {
                match execute_sniff_many(step, &mut requests, &context).await? {
                    SniffResult::Single(values) => {
                        for (output_name, value) in values {
                            context.set(&step.name, &output_name, value);
                        }
                    }
                    SniffResult::Array { name, items } => {
                        // Store array result for later processing
                        // The step.name and extractor name form the reference
                        array_result = Some((format!("{}.{}", step.name, name), items));
                    }
                }
            }
        }
    }

    Ok((context, array_result))
}
