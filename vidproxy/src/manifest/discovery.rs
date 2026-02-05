use anyhow::{Result, anyhow};
use chrome_browser::ChromeBrowserTab;

use super::executor::execute_steps;
use super::interpolate::InterpolationContext;
use super::types::{DiscoveredChannel, DiscoveryPhase};

/**
    Result of running the discovery phase.
*/
pub struct DiscoveryResult {
    /// Discovered channels
    pub channels: Vec<DiscoveredChannel>,
    /// Expiration timestamp (if extracted or specified)
    pub expires_at: Option<u64>,
}

/**
    Execute the discovery phase, returning a list of discovered channels.

    Supports two modes:
    1. Multi-channel: Uses a jsonpath_array extractor to discover multiple channels
    2. Single-channel: Uses scalar extractors to discover a single channel
*/
pub async fn execute_discovery(
    phase: &DiscoveryPhase,
    tab: &ChromeBrowserTab,
    source_id: &str,
    proxy: Option<&str>,
) -> Result<DiscoveryResult> {
    let context = InterpolationContext::new();

    let (context, array_result) = execute_steps(&phase.steps, tab, context, proxy).await?;

    let channels = if let Some((_array_key, items)) = array_result {
        // Multi-channel mode: build channels from the extracted array
        let mut channels = Vec::new();

        for item in items {
            // Get required id field
            let id = match item.get("id").and_then(|v| v.clone()) {
                Some(id) => id,
                None => continue, // Skip items without id
            };

            // Get optional fields
            let name = item.get("name").and_then(|v| v.clone());
            let image = item.get("image").and_then(|v| v.clone());

            channels.push(DiscoveredChannel {
                id,
                name,
                image,
                category: None,
                description: None,
                source: source_id.to_string(),
            });
        }

        channels
    } else {
        // Single-channel mode: interpolate outputs directly from context
        let id = context.interpolate(&phase.outputs.id)?;

        let name = phase
            .outputs
            .name
            .as_ref()
            .map(|t| context.interpolate(t))
            .transpose()?;

        let image = phase
            .outputs
            .image
            .as_ref()
            .map(|t| context.interpolate(t))
            .transpose()?;

        vec![DiscoveredChannel {
            id,
            name,
            image,
            category: None,
            description: None,
            source: source_id.to_string(),
        }]
    };

    if channels.is_empty() {
        return Err(anyhow!("Discovery found no channels"));
    }

    println!(
        "[discovery] Found {} channel(s) from '{}'",
        channels.len(),
        source_id
    );

    // Resolve expiration
    let expires_at = resolve_expiration(&phase.outputs, &context)?;

    Ok(DiscoveryResult {
        channels,
        expires_at,
    })
}

/**
    Resolve expiration from outputs (either expires_at interpolation or expires_in static).
*/
fn resolve_expiration(
    outputs: &super::types::DiscoveryOutputs,
    context: &InterpolationContext,
) -> Result<Option<u64>> {
    // Try expires_at first (interpolated)
    if let Some(expires_at_template) = &outputs.expires_at
        && let Ok(expires_str) = context.interpolate(expires_at_template)
        && let Ok(expires) = expires_str.parse::<u64>()
    {
        return Ok(Some(expires));
    }

    // Fall back to expires_in (static duration from now)
    if let Some(expires_in) = outputs.expires_in {
        return Ok(Some(crate::time::now() + expires_in));
    }

    Ok(None)
}
