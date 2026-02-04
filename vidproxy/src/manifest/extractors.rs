use std::collections::HashMap;

use anyhow::{Result, anyhow};
use regex::Regex;

use super::types::{Extractor, ExtractorKind};

/**
    Result of extracting from an array - a list of objects with string fields.
*/
pub type ExtractedArray = Vec<HashMap<String, Option<String>>>;

/**
    Run an extractor on the given content.
    Returns a single string value.
*/
pub fn extract(extractor: &Extractor, content: &str, url: &str) -> Result<String> {
    match extractor.kind {
        ExtractorKind::Url => Ok(url.to_string()),
        ExtractorKind::UrlRegex => extract_regex(extractor, url),
        ExtractorKind::JsonPath => extract_jsonpath(extractor, content),
        ExtractorKind::JsonPathArray => {
            Err(anyhow!("Use extract_array() for jsonpath_array extractors"))
        }
        ExtractorKind::JsonPathRegex => extract_jsonpath_regex(extractor, content),
        ExtractorKind::XPath => extract_xpath(extractor, content),
        ExtractorKind::Regex => extract_regex(extractor, content),
        ExtractorKind::Line => extract_line(content),
        ExtractorKind::Pssh => extract_pssh(content, url),
    }
}

/**
    Run a jsonpath_array extractor on the given content.
    Returns an array of objects, each with the fields defined in `each`.

    For discovery: Objects missing the "id" field are skipped.
    For metadata: Objects missing "channel_id" or "title" are skipped.

    Supports `$parent` in field paths to reference parent objects when using
    nested array paths like `$.result[*].content.epg[*]`.
*/
pub fn extract_array(extractor: &Extractor, content: &str) -> Result<ExtractedArray> {
    if extractor.kind != ExtractorKind::JsonPathArray {
        return Err(anyhow!(
            "extract_array() only works with jsonpath_array extractors"
        ));
    }

    let path = extractor
        .path
        .as_ref()
        .ok_or_else(|| anyhow!("jsonpath_array extractor requires 'path'"))?;

    let each = extractor
        .each
        .as_ref()
        .ok_or_else(|| anyhow!("jsonpath_array extractor requires 'each'"))?;

    extract_jsonpath_array(content, path, each)
}

/**
    Extract using JSONPath, returning array of objects.
    Supports $parent references in field paths for nested array extractions.
*/
fn extract_jsonpath_array(
    content: &str,
    path: &str,
    each: &HashMap<String, String>,
) -> Result<ExtractedArray> {
    let json: serde_json::Value =
        serde_json::from_str(content).map_err(|e| anyhow!("Failed to parse JSON: {}", e))?;

    // Check if any field uses $parent - if so, we need to track parent context
    let needs_parent = each.values().any(|p| p.contains("$parent"));

    if needs_parent {
        extract_with_parent_context(&json, path, each)
    } else {
        extract_simple(&json, path, each)
    }
}

/**
    Simple extraction without parent context tracking.
*/
fn extract_simple(
    json: &serde_json::Value,
    path: &str,
    each: &HashMap<String, String>,
) -> Result<ExtractedArray> {
    use jsonpath_rust::JsonPath;
    use std::str::FromStr;

    let jsonpath =
        JsonPath::from_str(path).map_err(|e| anyhow!("Invalid JSONPath '{}': {}", path, e))?;

    let results = jsonpath.find_slice(json);

    if results.is_empty() {
        return Err(anyhow!("JSONPath '{}' returned no results", path));
    }

    let mut extracted = Vec::new();

    for result in results {
        let obj = result.clone().to_data();

        let mut fields: HashMap<String, Option<String>> = HashMap::new();

        for (field_name, field_path) in each {
            let value = extract_jsonpath_value(&obj, field_path);
            fields.insert(field_name.clone(), value);
        }

        // Skip based on what fields are present
        if should_skip_item(&fields) {
            continue;
        }

        extracted.push(fields);
    }

    if extracted.is_empty() {
        return Err(anyhow!(
            "JSONPath '{}' returned results but none had required fields",
            path
        ));
    }

    Ok(extracted)
}

/**
    Extraction with parent context tracking for $parent references.
    Handles paths like `$.result[*].content.epg[*]` where we need to access
    fields from the parent `result[*]` item while iterating over `epg[*]`.
*/
fn extract_with_parent_context(
    json: &serde_json::Value,
    path: &str,
    each: &HashMap<String, String>,
) -> Result<ExtractedArray> {
    use jsonpath_rust::JsonPath;
    use std::str::FromStr;

    // Parse the path to find where the nested arrays are
    // e.g., "$.result[*].content.epg[*]" -> parent: "$.result[*]", child: "$.content.epg[*]"
    let (parent_path, child_path) = split_nested_path(path)?;

    let parent_jsonpath = JsonPath::from_str(&parent_path)
        .map_err(|e| anyhow!("Invalid parent JSONPath '{}': {}", parent_path, e))?;

    let child_jsonpath = JsonPath::from_str(&child_path)
        .map_err(|e| anyhow!("Invalid child JSONPath '{}': {}", child_path, e))?;

    let parent_results = parent_jsonpath.find_slice(json);

    if parent_results.is_empty() {
        return Err(anyhow!(
            "Parent JSONPath '{}' returned no results",
            parent_path
        ));
    }

    let mut extracted = Vec::new();

    for parent_result in parent_results {
        let parent_obj = parent_result.clone().to_data();
        let child_results = child_jsonpath.find_slice(&parent_obj);

        for child_result in child_results {
            let child_obj = child_result.clone().to_data();

            let mut fields: HashMap<String, Option<String>> = HashMap::new();

            for (field_name, field_path) in each {
                let value = if field_path.starts_with("$parent") {
                    // Extract from parent object
                    let parent_field_path = field_path.replacen("$parent", "$", 1);
                    extract_jsonpath_value(&parent_obj, &parent_field_path)
                } else {
                    // Extract from child object
                    extract_jsonpath_value(&child_obj, field_path)
                };
                fields.insert(field_name.clone(), value);
            }

            if should_skip_item(&fields) {
                continue;
            }

            extracted.push(fields);
        }
    }

    if extracted.is_empty() {
        return Err(anyhow!(
            "Nested extraction returned no valid items with required fields"
        ));
    }

    Ok(extracted)
}

/**
    Split a nested JSONPath into parent and child portions.
    e.g., "$.result[*].content.epg[*]" -> ("$.result[*]", "$.content.epg[*]")

    Finds the last `[*]` and splits there, as that's typically where
    the parent/child boundary is for nested array access.
*/
fn split_nested_path(path: &str) -> Result<(String, String)> {
    // Find all occurrences of [*]
    let indices: Vec<_> = path.match_indices("[*]").collect();

    if indices.len() < 2 {
        return Err(anyhow!(
            "Path '{}' needs at least two [*] for $parent support",
            path
        ));
    }

    // Split at the first [*] - that's the parent array
    let (first_idx, _) = indices[0];
    let split_point = first_idx + 3; // After the first [*]

    let parent_path = path[..split_point].to_string();
    let child_path = format!("${}", &path[split_point..]);

    Ok((parent_path, child_path))
}

/**
    Determine if an extracted item should be skipped based on required fields.
    - For discovery: requires "id"
    - For metadata: requires "channel_id" and "title"
*/
fn should_skip_item(fields: &HashMap<String, Option<String>>) -> bool {
    let has_id = fields.get("id").and_then(|v| v.as_ref()).is_some();
    let has_channel_id = fields.get("channel_id").and_then(|v| v.as_ref()).is_some();
    let has_title = fields.get("title").and_then(|v| v.as_ref()).is_some();

    // If it has channel_id, it's metadata - require channel_id and title
    if fields.contains_key("channel_id") {
        return !has_channel_id || !has_title;
    }

    // Otherwise it's discovery - require id
    !has_id
}

/**
    Extract a single value from a JSON object using JSONPath.
    Returns None if the path doesn't match or returns null.
*/
fn extract_jsonpath_value(obj: &serde_json::Value, path: &str) -> Option<String> {
    use jsonpath_rust::JsonPath;
    use std::str::FromStr;

    let jsonpath = JsonPath::from_str(path).ok()?;
    let results = jsonpath.find_slice(obj);

    if results.is_empty() {
        return None;
    }

    match results[0].clone().to_data() {
        serde_json::Value::String(s) => Some(s),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        serde_json::Value::Null => None,
        other => Some(other.to_string()),
    }
}

/**
    Extract using JSONPath (single value).
*/
fn extract_jsonpath(extractor: &Extractor, content: &str) -> Result<String> {
    use jsonpath_rust::JsonPath;
    use std::str::FromStr;

    let path = extractor
        .path
        .as_ref()
        .ok_or_else(|| anyhow!("JSONPath extractor requires 'path'"))?;

    let json: serde_json::Value =
        serde_json::from_str(content).map_err(|e| anyhow!("Failed to parse JSON: {}", e))?;

    let jsonpath =
        JsonPath::from_str(path).map_err(|e| anyhow!("Invalid JSONPath '{}': {}", path, e))?;

    let results = jsonpath.find_slice(&json);

    if results.is_empty() {
        return Err(anyhow!("JSONPath '{}' returned no results", path));
    }

    // Return the first result as a string
    match results[0].clone().to_data() {
        serde_json::Value::String(s) => Ok(s),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        serde_json::Value::Bool(b) => Ok(b.to_string()),
        serde_json::Value::Null => Err(anyhow!("JSONPath '{}' returned null", path)),
        other => Ok(other.to_string()),
    }
}

/**
    Extract using JSONPath followed by regex on the result.
*/
fn extract_jsonpath_regex(extractor: &Extractor, content: &str) -> Result<String> {
    use jsonpath_rust::JsonPath;
    use std::str::FromStr;

    let path = extractor
        .path
        .as_ref()
        .ok_or_else(|| anyhow!("JSONPath regex extractor requires 'path'"))?;

    let regex_pattern = extractor
        .regex
        .as_ref()
        .ok_or_else(|| anyhow!("JSONPath regex extractor requires 'regex'"))?;

    let json: serde_json::Value =
        serde_json::from_str(content).map_err(|e| anyhow!("Failed to parse JSON: {}", e))?;

    let jsonpath =
        JsonPath::from_str(path).map_err(|e| anyhow!("Invalid JSONPath '{}': {}", path, e))?;

    let results = jsonpath.find_slice(&json);

    if results.is_empty() {
        return Err(anyhow!("JSONPath '{}' returned no results", path));
    }

    // Get the first result as a string
    let value = match results[0].clone().to_data() {
        serde_json::Value::String(s) => s,
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => return Err(anyhow!("JSONPath '{}' returned null", path)),
        other => other.to_string(),
    };

    // Apply regex to the extracted value
    let re = Regex::new(regex_pattern)
        .map_err(|e| anyhow!("Invalid regex '{}': {}", regex_pattern, e))?;

    let captures = re
        .captures(&value)
        .ok_or_else(|| anyhow!("Regex '{}' did not match value '{}'", regex_pattern, value))?;

    // Return first capture group, or whole match if no groups
    if captures.len() > 1 {
        Ok(captures.get(1).unwrap().as_str().to_string())
    } else {
        Ok(captures.get(0).unwrap().as_str().to_string())
    }
}

/**
    Extract using XPath.
*/
fn extract_xpath(extractor: &Extractor, content: &str) -> Result<String> {
    let path = extractor
        .path
        .as_ref()
        .ok_or_else(|| anyhow!("XPath extractor requires 'path'"))?;

    let package = sxd_document::parser::parse(content)
        .map_err(|e| anyhow!("Failed to parse XML: {:?}", e))?;
    let document = package.as_document();

    let factory = sxd_xpath::Factory::new();
    let xpath = factory
        .build(path)
        .map_err(|e| anyhow!("Invalid XPath '{}': {:?}", path, e))?
        .ok_or_else(|| anyhow!("XPath '{}' is empty", path))?;

    let context = sxd_xpath::Context::new();
    let value = xpath
        .evaluate(&context, document.root())
        .map_err(|e| anyhow!("XPath evaluation failed: {:?}", e))?;

    match value {
        sxd_xpath::Value::String(s) => Ok(s),
        sxd_xpath::Value::Number(n) => Ok(n.to_string()),
        sxd_xpath::Value::Boolean(b) => Ok(b.to_string()),
        sxd_xpath::Value::Nodeset(nodes) => {
            if nodes.size() == 0 {
                return Err(anyhow!("XPath '{}' returned no nodes", path));
            }
            // Get text content of first node
            let node = nodes.iter().next().unwrap();
            Ok(node.string_value())
        }
    }
}

/**
    Extract using regex with capture group.
*/
fn extract_regex(extractor: &Extractor, content: &str) -> Result<String> {
    let pattern = extractor
        .path
        .as_ref()
        .ok_or_else(|| anyhow!("Regex extractor requires 'path'"))?;

    let re = Regex::new(pattern).map_err(|e| anyhow!("Invalid regex '{}': {}", pattern, e))?;

    let captures = re
        .captures(content)
        .ok_or_else(|| anyhow!("Regex '{}' did not match", pattern))?;

    // Return first capture group, or whole match if no groups
    if captures.len() > 1 {
        Ok(captures.get(1).unwrap().as_str().to_string())
    } else {
        Ok(captures.get(0).unwrap().as_str().to_string())
    }
}

/**
    Extract first line containing ":".
*/
fn extract_line(content: &str) -> Result<String> {
    content
        .lines()
        .find(|line| line.contains(':'))
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("No line containing ':' found"))
}

/**
    Extract Widevine PSSH from MPD manifest using ffmpeg-source DASH parser.
*/
fn extract_pssh(content: &str, url: &str) -> Result<String> {
    use ffmpeg_source::reader::stream::StreamFormat;
    use ffmpeg_source::reader::stream::dash::DashFormat;

    let dash = DashFormat::from_manifest(url, content.as_bytes())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_url() {
        let extractor = Extractor {
            kind: ExtractorKind::Url,
            path: None,
            regex: None,
            each: None,
        };
        let result = extract(&extractor, "body content", "https://example.com/test.mpd").unwrap();
        assert_eq!(result, "https://example.com/test.mpd");
    }

    #[test]
    fn test_extract_line() {
        let extractor = Extractor {
            kind: ExtractorKind::Line,
            path: None,
            regex: None,
            each: None,
        };
        let content = "some header\nabc123:def456\nmore stuff";
        let result = extract(&extractor, content, "").unwrap();
        assert_eq!(result, "abc123:def456");
    }

    #[test]
    fn test_extract_regex() {
        let extractor = Extractor {
            kind: ExtractorKind::Regex,
            path: Some(r"id=(\d+)".to_string()),
            regex: None,
            each: None,
        };
        let result = extract(&extractor, "content?id=12345&other=value", "").unwrap();
        assert_eq!(result, "12345");
    }

    #[test]
    fn test_extract_jsonpath_array() {
        let mut each = HashMap::new();
        each.insert("id".to_string(), "$.id".to_string());
        each.insert("name".to_string(), "$.title".to_string());
        each.insert("image".to_string(), "$.thumbnail".to_string());

        let extractor = Extractor {
            kind: ExtractorKind::JsonPathArray,
            path: Some("$.items[*]".to_string()),
            regex: None,
            each: Some(each),
        };

        let content = r#"{
            "items": [
                {"id": "123", "title": "Channel One", "thumbnail": "http://img1.jpg"},
                {"id": "456", "title": "Channel Two"},
                {"title": "No ID Channel"}
            ]
        }"#;

        let result = extract_array(&extractor, content).unwrap();

        // Should have 2 results (the one without id is skipped)
        assert_eq!(result.len(), 2);

        // First channel
        assert_eq!(result[0].get("id").unwrap(), &Some("123".to_string()));
        assert_eq!(
            result[0].get("name").unwrap(),
            &Some("Channel One".to_string())
        );
        assert_eq!(
            result[0].get("image").unwrap(),
            &Some("http://img1.jpg".to_string())
        );

        // Second channel (no image)
        assert_eq!(result[1].get("id").unwrap(), &Some("456".to_string()));
        assert_eq!(
            result[1].get("name").unwrap(),
            &Some("Channel Two".to_string())
        );
        assert_eq!(result[1].get("image").unwrap(), &None);
    }

    #[test]
    fn test_extract_nested_with_parent() {
        let mut each = HashMap::new();
        each.insert("channel_id".to_string(), "$parent.id".to_string());
        each.insert("title".to_string(), "$.title".to_string());
        each.insert("start_time".to_string(), "$.startTime".to_string());

        let extractor = Extractor {
            kind: ExtractorKind::JsonPathArray,
            path: Some("$.result[*].content.epg[*]".to_string()),
            regex: None,
            each: Some(each),
        };

        let content = r#"{
            "result": [
                {
                    "id": "channel1",
                    "content": {
                        "epg": [
                            {"title": "Show A", "startTime": "2026-01-01T00:00:00Z"},
                            {"title": "Show B", "startTime": "2026-01-01T01:00:00Z"}
                        ]
                    }
                },
                {
                    "id": "channel2",
                    "content": {
                        "epg": [
                            {"title": "Show C", "startTime": "2026-01-01T00:00:00Z"}
                        ]
                    }
                }
            ]
        }"#;

        let result = extract_array(&extractor, content).unwrap();

        // Should have 3 programmes total
        assert_eq!(result.len(), 3);

        // First programme from channel1
        assert_eq!(
            result[0].get("channel_id").unwrap(),
            &Some("channel1".to_string())
        );
        assert_eq!(result[0].get("title").unwrap(), &Some("Show A".to_string()));

        // Second programme from channel1
        assert_eq!(
            result[1].get("channel_id").unwrap(),
            &Some("channel1".to_string())
        );
        assert_eq!(result[1].get("title").unwrap(), &Some("Show B".to_string()));

        // Programme from channel2
        assert_eq!(
            result[2].get("channel_id").unwrap(),
            &Some("channel2".to_string())
        );
        assert_eq!(result[2].get("title").unwrap(), &Some("Show C".to_string()));
    }

    #[test]
    fn test_split_nested_path() {
        let (parent, child) = split_nested_path("$.result[*].content.epg[*]").unwrap();
        assert_eq!(parent, "$.result[*]");
        assert_eq!(child, "$.content.epg[*]");
    }
}
