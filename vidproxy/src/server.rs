use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;
use tokio::sync::watch;
use tower_http::services::ServeDir;

use crate::segments::SegmentManager;

#[derive(Clone)]
struct AppState {
    channel_name: String,
    port: u16,
}

async fn channels_m3u(State(state): State<AppState>) -> impl IntoResponse {
    let playlist = format!(
        "#EXTM3U\n#EXTINF:-1 tvg-name=\"{name}\",{name}\nhttp://localhost:{port}/playlist.m3u8\n",
        name = state.channel_name,
        port = state.port,
    );

    ([(header::CONTENT_TYPE, "audio/x-mpegurl")], playlist)
}

/**
    Run the HTTP server that serves HLS content.
*/
pub async fn run_server(
    addr: SocketAddr,
    segment_manager: Arc<SegmentManager>,
    mut shutdown_rx: watch::Receiver<bool>,
    channel_name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let serve_dir =
        ServeDir::new(segment_manager.output_dir()).append_index_html_on_directories(false);

    let state = AppState {
        channel_name: channel_name.to_string(),
        port: addr.port(),
    };

    let app = Router::new()
        .route("/channels.m3u", get(channels_m3u))
        .with_state(state)
        .fallback_service(serve_dir);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("HTTP server listening on http://{}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            // Wait for shutdown signal
            while !*shutdown_rx.borrow_and_update() {
                if shutdown_rx.changed().await.is_err() {
                    break;
                }
            }
        })
        .await?;

    Ok(())
}
