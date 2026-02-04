use std::time::Duration;

use chrono::Utc;
use tokio::sync::watch;

use crate::stream_info::{StreamInfo, StreamInfoReceiver};

/**
    Refresh signal sender type.
*/
pub type RefreshSender = watch::Sender<bool>;
/**
    Refresh signal receiver type.
*/
pub type RefreshReceiver = watch::Receiver<bool>;

/**
    Create a refresh signal channel.
*/
pub fn refresh_channel() -> (RefreshSender, RefreshReceiver) {
    watch::channel(false)
}

/**
    Coordinator that monitors stream info and triggers refreshes when needed.

    The coordinator no longer runs the pipeline directly - that's handled by
    PipelineManager. Instead, it just watches for stream info changes and
    handles expiration-triggered refreshes.
*/
pub struct Coordinator {
    stream_info_rx: StreamInfoReceiver,
    refresh_tx: RefreshSender,
    shutdown_rx: watch::Receiver<bool>,
}

impl Coordinator {
    pub fn new(
        stream_info_rx: StreamInfoReceiver,
        refresh_tx: RefreshSender,
        shutdown_rx: watch::Receiver<bool>,
    ) -> Self {
        Self {
            stream_info_rx,
            refresh_tx,
            shutdown_rx,
        }
    }

    /**
        Run the coordinator loop.

        This loop monitors stream info and triggers refreshes before
        the stream credentials expire.
    */
    pub async fn run(&mut self) -> anyhow::Result<()> {
        println!("[coordinator] Starting, waiting for stream info...");

        loop {
            // Check for shutdown
            if *self.shutdown_rx.borrow() {
                println!("[coordinator] Shutdown requested");
                break;
            }

            // Wait for stream info
            let stream_info = match self.wait_for_stream_info().await {
                Some(info) => info,
                None => {
                    println!("[coordinator] Shutdown during stream info wait");
                    break;
                }
            };

            println!(
                "[coordinator] Got stream info: {}",
                &stream_info.mpd_url[..stream_info.mpd_url.len().min(60)]
            );

            // Wait for expiration or stream info change
            self.wait_for_refresh_trigger(&stream_info).await;

            // Check for shutdown before requesting refresh
            if *self.shutdown_rx.borrow() {
                println!("[coordinator] Shutdown requested");
                break;
            }

            // Trigger refresh
            println!("[coordinator] Requesting stream refresh...");
            let _ = self.refresh_tx.send(true);
            tokio::time::sleep(Duration::from_millis(100)).await;
            let _ = self.refresh_tx.send(false);
        }

        Ok(())
    }

    /**
        Wait for stream info to become available.
    */
    async fn wait_for_stream_info(&mut self) -> Option<StreamInfo> {
        loop {
            // Check if we already have stream info
            if let Some(ref info) = *self.stream_info_rx.borrow() {
                return Some(info.clone());
            }

            // Wait for stream info or shutdown
            tokio::select! {
                result = self.stream_info_rx.changed() => {
                    if result.is_err() {
                        return None;
                    }
                    if let Some(ref info) = *self.stream_info_rx.borrow() {
                        return Some(info.clone());
                    }
                }
                _ = self.shutdown_rx.changed() => {
                    if *self.shutdown_rx.borrow() {
                        return None;
                    }
                }
            }
        }
    }

    /**
        Wait until we need to refresh the stream.

        This returns when:
        - The stream is about to expire (based on expires_at)
        - The stream info changes (external refresh)
        - Shutdown is requested
    */
    async fn wait_for_refresh_trigger(&mut self, stream_info: &StreamInfo) {
        // Calculate when we need to refresh
        let refresh_after = stream_info.expires_at.map(|expires| {
            let now = Utc::now().timestamp() as u64;
            if expires > now {
                // Refresh 60 seconds before expiration
                let refresh_in = expires.saturating_sub(now).saturating_sub(60);
                if refresh_in > 0 {
                    println!(
                        "[coordinator] Stream expires in {}s, will refresh in {}s",
                        expires - now,
                        refresh_in
                    );
                    Duration::from_secs(refresh_in)
                } else {
                    // Already close to expiring, refresh soon
                    println!("[coordinator] Stream expiring soon, will refresh in 5s");
                    Duration::from_secs(5)
                }
            } else {
                println!("[coordinator] Stream already expired, refreshing immediately");
                Duration::ZERO
            }
        });

        // If no expiration, wait indefinitely for stream info change or shutdown
        let sleep_future = async {
            if let Some(duration) = refresh_after {
                if duration > Duration::ZERO {
                    tokio::time::sleep(duration).await;
                }
            } else {
                // No expiration - wait forever (until stream info change or shutdown)
                std::future::pending::<()>().await;
            }
        };

        tokio::select! {
            _ = sleep_future => {
                // Expiration timer fired
            }
            _ = self.stream_info_rx.changed() => {
                // Stream info changed externally
                println!("[coordinator] Stream info changed");
            }
            _ = self.shutdown_rx.changed() => {
                // Shutdown requested
            }
        }
    }
}
