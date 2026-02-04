use std::collections::HashMap;
use std::sync::RwLock;

use crate::manifest::{ChannelEntry, StreamInfo};

/**
    Full channel ID combining source and channel ID.
*/
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChannelId {
    pub source: String,
    pub id: String,
}

impl ChannelId {
    pub fn new(source: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            id: id.into(),
        }
    }

    /**
        Parse from "source:id" format
    */
    #[allow(dead_code)]
    pub fn parse(s: &str) -> Option<Self> {
        let (source, id) = s.split_once(':')?;
        Some(Self::new(source, id))
    }

    /**
        Format as "source:id"
    */
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        format!("{}:{}", self.source, self.id)
    }
}

/**
    In-memory registry of all discovered channels.
*/
pub struct ChannelRegistry {
    channels: RwLock<HashMap<ChannelId, ChannelEntry>>,
    /// When each source's discovery results expire
    discovery_expiration: RwLock<HashMap<String, Option<u64>>>,
}

impl ChannelRegistry {
    pub fn new() -> Self {
        Self {
            channels: RwLock::new(HashMap::new()),
            discovery_expiration: RwLock::new(HashMap::new()),
        }
    }

    /**
        Register channels from a source discovery result.
    */
    pub fn register_source(
        &self,
        source_name: &str,
        channels: Vec<ChannelEntry>,
        discovery_expires_at: Option<u64>,
    ) {
        let mut registry = self.channels.write().unwrap();

        // Remove old channels from this source
        registry.retain(|id, _| id.source != source_name);

        // Add new channels
        for entry in channels {
            let id = ChannelId::new(source_name, &entry.channel.id);
            registry.insert(id, entry);
        }

        // Update discovery expiration
        let mut expirations = self.discovery_expiration.write().unwrap();
        expirations.insert(source_name.to_string(), discovery_expires_at);
    }

    /**
        Get a channel by its full ID.
    */
    pub fn get(&self, id: &ChannelId) -> Option<ChannelEntry> {
        self.channels.read().unwrap().get(id).cloned()
    }

    /**
        Get a channel by source and channel ID strings.
    */
    #[allow(dead_code)]
    pub fn get_by_parts(&self, source: &str, channel_id: &str) -> Option<ChannelEntry> {
        let id = ChannelId::new(source, channel_id);
        self.get(&id)
    }

    /**
        List all channels.
    */
    pub fn list_all(&self) -> Vec<(ChannelId, ChannelEntry)> {
        self.channels
            .read()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /**
        List channels from a specific source.
    */
    #[allow(dead_code)]
    pub fn list_by_source(&self, source: &str) -> Vec<ChannelEntry> {
        self.channels
            .read()
            .unwrap()
            .iter()
            .filter(|(id, _)| id.source == source)
            .map(|(_, v)| v.clone())
            .collect()
    }

    /**
        Update stream info for a channel.
    */
    pub fn update_stream_info(&self, id: &ChannelId, stream_info: StreamInfo) {
        let mut registry = self.channels.write().unwrap();
        if let Some(entry) = registry.get_mut(id) {
            entry.stream_info = Some(stream_info);
            entry.last_error = None;
        }
    }

    /**
        Mark a channel as having an error.
    */
    pub fn set_error(&self, id: &ChannelId, error: String) {
        let mut registry = self.channels.write().unwrap();
        if let Some(entry) = registry.get_mut(id) {
            entry.last_error = Some(error);
        }
    }

    /**
        Check if a channel's stream info has expired.
    */
    pub fn is_stream_expired(&self, id: &ChannelId) -> bool {
        let registry = self.channels.read().unwrap();
        if let Some(entry) = registry.get(id) {
            if let Some(ref stream_info) = entry.stream_info
                && let Some(expires_at) = stream_info.expires_at
            {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                return now >= expires_at;
            }
            // No stream info or no expiration = treat as expired
            return entry.stream_info.is_none();
        }
        true // Channel not found = expired
    }

    /**
        Check if a source's discovery has expired.
    */
    pub fn is_discovery_expired(&self, source: &str) -> bool {
        let expirations = self.discovery_expiration.read().unwrap();
        if let Some(Some(expires_at)) = expirations.get(source) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            return now >= *expires_at;
        }
        // No expiration set = not expired (discovery runs once at startup)
        false
    }

    /**
        Get total channel count.
    */
    pub fn len(&self) -> usize {
        self.channels.read().unwrap().len()
    }

    /**
        Check if registry is empty.
    */
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.channels.read().unwrap().is_empty()
    }
}

impl Default for ChannelRegistry {
    fn default() -> Self {
        Self::new()
    }
}
