use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};
use tokio::sync::{Mutex, oneshot, watch};

use crate::cdrm;
use crate::coordinator::RefreshSender;
use crate::proxy;
use crate::segments::SegmentManager;
use crate::stream_info::StreamInfo;

/// State of the pipeline
#[derive(Debug)]
enum PipelineState {
    /// Pipeline is not running
    Idle,
    /// Pipeline is starting up
    Starting,
    /// Pipeline is running
    Running {
        /// Send to stop the pipeline
        stop_tx: oneshot::Sender<()>,
    },
    /// Pipeline is stopping
    Stopping,
}

/// Manages the lifecycle of a remux pipeline.
///
/// The pipeline is started on-demand when a client requests the playlist,
/// and stopped after a period of inactivity.
pub struct PipelineManager {
    state: Arc<Mutex<PipelineState>>,
    stream_info_rx: watch::Receiver<Option<StreamInfo>>,
    refresh_tx: RefreshSender,
    segment_manager: Arc<SegmentManager>,
    segment_duration: Duration,
    output_dir: PathBuf,
    idle_timeout: Duration,
    startup_timeout: Duration,
    last_activity: AtomicU64,
}

impl PipelineManager {
    pub fn new(
        stream_info_rx: watch::Receiver<Option<StreamInfo>>,
        refresh_tx: RefreshSender,
        segment_manager: Arc<SegmentManager>,
        segment_duration: Duration,
        output_dir: PathBuf,
        idle_timeout: Duration,
        startup_timeout: Duration,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(PipelineState::Idle)),
            stream_info_rx,
            refresh_tx,
            segment_manager,
            segment_duration,
            output_dir,
            idle_timeout,
            startup_timeout,
            last_activity: AtomicU64::new(0),
        }
    }

    /// Get the output directory
    pub fn output_dir(&self) -> &std::path::Path {
        &self.output_dir
    }

    /// Check if the pipeline is currently running
    pub async fn is_running(&self) -> bool {
        matches!(*self.state.lock().await, PipelineState::Running { .. })
    }

    /// Record activity (called on segment requests)
    pub fn record_activity(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_activity.store(now, Ordering::Relaxed);
    }

    /// Get seconds since last activity
    fn seconds_since_activity(&self) -> u64 {
        let last = self.last_activity.load(Ordering::Relaxed);
        if last == 0 {
            return 0;
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now.saturating_sub(last)
    }

    /// Ensure the pipeline is running. If not running, start it.
    /// Returns immediately if already running.
    pub async fn ensure_running(&self) -> Result<()> {
        // Check current state
        {
            let state = self.state.lock().await;
            match *state {
                PipelineState::Running { .. } => {
                    self.record_activity();
                    return Ok(());
                }
                PipelineState::Starting => {
                    // Already starting, caller should wait_for_ready
                    return Ok(());
                }
                PipelineState::Stopping => {
                    return Err(anyhow!("Pipeline is stopping, try again later"));
                }
                PipelineState::Idle => {
                    // Need to start
                }
            }
        }

        // Start the pipeline
        self.start().await
    }

    /// Start the pipeline
    async fn start(&self) -> Result<()> {
        // Transition to Starting
        {
            let mut state = self.state.lock().await;
            if !matches!(*state, PipelineState::Idle) {
                return Ok(()); // Already starting or running
            }
            *state = PipelineState::Starting;
        }

        // Get stream info
        let stream_info = self
            .stream_info_rx
            .borrow()
            .clone()
            .ok_or_else(|| anyhow!("No stream info available"))?;

        // Clear old segments
        self.segment_manager.clear();

        // Record initial activity
        self.record_activity();

        // Create stop channel
        let (stop_tx, stop_rx) = oneshot::channel();

        // Clone what we need for the spawned task
        let mpd_url = stream_info.mpd_url.clone();
        let license_url = stream_info.license_url.clone();
        let output_dir = self.output_dir.clone();
        let segment_duration = self.segment_duration;
        let segment_manager = Arc::clone(&self.segment_manager);
        let state = Arc::clone(&self.state);
        let refresh_tx = self.refresh_tx.clone();

        // Spawn the pipeline task
        tokio::spawn(async move {
            // Helper to reset state to Idle on exit
            let reset_state = |auth_error: bool| {
                let state = Arc::clone(&state);
                let refresh_tx = refresh_tx.clone();
                async move {
                    let mut state_guard = state.lock().await;
                    // Only reset if we're still in Running state (not being stopped externally)
                    if matches!(*state_guard, PipelineState::Running { .. }) {
                        *state_guard = PipelineState::Idle;

                        // If this looks like an auth error, request credential refresh
                        if auth_error {
                            println!(
                                "[pipeline] Auth error detected, requesting credential refresh"
                            );
                            let _ = refresh_tx.send(true);
                        }
                    }
                }
            };

            // Fetch decryption key if needed
            let decryption_key = if let Some(ref lic_url) = license_url {
                match cdrm::get_decryption_key(&mpd_url, lic_url).await {
                    Ok(key) => Some(key),
                    Err(e) => {
                        let err_str = e.to_string();
                        eprintln!("[pipeline] Failed to get decryption key: {}", err_str);
                        let is_auth_error = is_auth_error(&err_str);
                        reset_state(is_auth_error).await;
                        return;
                    }
                }
            } else {
                None
            };

            // Create a watch channel for shutdown signaling within the pipeline
            let (shutdown_tx, shutdown_rx) = watch::channel(false);

            // Spawn the stop listener
            let shutdown_tx_clone = shutdown_tx.clone();
            tokio::spawn(async move {
                let _ = stop_rx.await;
                let _ = shutdown_tx_clone.send(true);
            });

            // Run the pipeline
            println!("[pipeline] Starting remux pipeline");
            let result = tokio::task::spawn_blocking(move || {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(proxy::run_remux_pipeline(
                    &mpd_url,
                    &[],
                    decryption_key.as_deref(),
                    &output_dir,
                    segment_duration,
                    segment_manager,
                    shutdown_rx,
                ))
            })
            .await;

            let auth_error = match &result {
                Ok(Ok(())) => {
                    println!("[pipeline] Pipeline completed normally");
                    false
                }
                Ok(Err(e)) => {
                    let err_str = e.to_string();
                    eprintln!("[pipeline] Pipeline error: {}", err_str);
                    is_auth_error(&err_str)
                }
                Err(e) => {
                    eprintln!("[pipeline] Pipeline task panicked: {}", e);
                    false
                }
            };

            // Pipeline ended (either normally or with error), reset state
            reset_state(auth_error).await;
        });

        // Transition to Running
        {
            let mut state = self.state.lock().await;
            *state = PipelineState::Running { stop_tx };
        }

        println!("[pipeline] Pipeline started");
        Ok(())
    }

    /// Stop the pipeline if running
    pub async fn stop(&self) {
        let stop_tx = {
            let mut state = self.state.lock().await;
            match std::mem::replace(&mut *state, PipelineState::Stopping) {
                PipelineState::Running { stop_tx } => Some(stop_tx),
                other => {
                    *state = other;
                    None
                }
            }
        };

        if let Some(tx) = stop_tx {
            println!("[pipeline] Stopping pipeline");
            let _ = tx.send(());

            // Give it a moment to clean up
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        // Transition to Idle
        {
            let mut state = self.state.lock().await;
            *state = PipelineState::Idle;
        }

        println!("[pipeline] Pipeline stopped");
    }

    /// Wait for the pipeline to be ready (has at least one segment)
    pub async fn wait_for_ready(&self) -> Result<()> {
        let deadline = Instant::now() + self.startup_timeout;

        loop {
            if self.segment_manager.segment_count() > 0 {
                return Ok(());
            }

            if Instant::now() > deadline {
                return Err(anyhow!("Timeout waiting for first segment"));
            }

            // Check if pipeline failed to start
            {
                let state = self.state.lock().await;
                if matches!(*state, PipelineState::Idle) {
                    return Err(anyhow!("Pipeline failed to start"));
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Run background management tasks for this pipeline.
    ///
    /// This handles:
    /// - Stopping the pipeline after idle timeout
    /// - Restarting the pipeline when stream info changes (credential refresh)
    /// - Graceful shutdown
    pub async fn run_background_tasks(self: Arc<Self>, mut shutdown_rx: watch::Receiver<bool>) {
        let mut stream_info_rx = self.stream_info_rx.clone();

        loop {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    // Check if we should stop due to inactivity
                    if self.is_running().await {
                        let idle_secs = self.seconds_since_activity();
                        if idle_secs > self.idle_timeout.as_secs() {
                            println!("[pipeline] Idle for {}s, stopping pipeline", idle_secs);
                            self.stop().await;
                        }
                    }
                }
                _ = stream_info_rx.changed() => {
                    // Stream info changed - if pipeline is running, restart it
                    if self.is_running().await {
                        println!("[pipeline] Stream info changed, restarting pipeline");
                        self.stop().await;
                        // Small delay before restart
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        if let Err(e) = self.ensure_running().await {
                            eprintln!("[pipeline] Failed to restart after stream info change: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        // Shutdown requested, stop pipeline if running
                        self.stop().await;
                        return;
                    }
                }
            }
        }
    }
}

/// Check if an error message indicates an authentication/authorization failure
fn is_auth_error(err: &str) -> bool {
    err.contains("401")
        || err.contains("403")
        || err.contains("410")
        || err.contains("Unauthorized")
        || err.contains("Forbidden")
        || err.contains("expired")
}
