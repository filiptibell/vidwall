use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::{Duration, Utc};
use tokio::sync::watch;
use tokio_util::io::ReaderStream;

use crate::pipeline::PipelineManager;
use crate::stream_info::StreamInfoReceiver;

#[derive(Clone)]
struct AppState {
    fallback_channel_name: String,
    stream_info_rx: StreamInfoReceiver,
    pipeline_manager: Arc<PipelineManager>,
    output_dir: PathBuf,
}

async fn channels_m3u(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost:8080");

    let stream_info = state.stream_info_rx.borrow();
    let channel_name = stream_info
        .as_ref()
        .map(|info| info.channel_name.as_str())
        .unwrap_or(&state.fallback_channel_name);
    let thumbnail_url = stream_info
        .as_ref()
        .and_then(|info| info.thumbnail_url.as_deref());

    let logo_attr = thumbnail_url
        .map(|url| format!(" tvg-logo=\"{}\"", url))
        .unwrap_or_default();

    // url-tvg points to EPG data
    // tvg-id uses channel name as identifier for EPG matching
    // tvg-type="live" indicates 24/7 live stream (not VOD)
    // group-title categorizes the channel
    let playlist = format!(
        "#EXTM3U url-tvg=\"http://{host}/epg.xml\"\n\
         #EXTINF:-1 tvg-id=\"{name}\" tvg-name=\"{name}\" tvg-type=\"live\" group-title=\"Live TV\"{logo},{name}\n\
         http://{host}/playlist.m3u8\n",
        name = channel_name,
        logo = logo_attr,
        host = host,
    );

    ([(header::CONTENT_TYPE, "audio/x-mpegurl")], playlist)
}

/**
    Generate XMLTV EPG data for 24/7 live channels
*/
async fn epg_xml(State(state): State<AppState>) -> impl IntoResponse {
    let stream_info = state.stream_info_rx.borrow();
    let channel_name = stream_info
        .as_ref()
        .map(|info| info.channel_name.as_str())
        .unwrap_or(&state.fallback_channel_name);
    let thumbnail_url = stream_info
        .as_ref()
        .and_then(|info| info.thumbnail_url.as_deref());

    let icon_element = thumbnail_url
        .map(|url| format!("    <icon src=\"{}\"/>\n", escape_xml(url)))
        .unwrap_or_default();

    // Generate program entries for the next 7 days (one entry per day)
    let now = Utc::now();
    let start_of_day = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
    let start = start_of_day.and_utc();

    let mut programmes = String::new();
    for day in 0..7 {
        let day_start = start + Duration::days(day);
        let day_end = day_start + Duration::days(1);

        let start_str = day_start.format("%Y%m%d%H%M%S %z");
        let end_str = day_end.format("%Y%m%d%H%M%S %z");

        programmes.push_str(&format!(
            "  <programme start=\"{}\" stop=\"{}\" channel=\"{}\">\n\
             \x20   <title lang=\"es\">{}</title>\n\
             \x20   <desc lang=\"es\">Transmisión en vivo 24/7</desc>\n\
             \x20 </programme>\n",
            start_str,
            end_str,
            escape_xml(channel_name),
            escape_xml(channel_name),
        ));
    }

    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE tv SYSTEM \"xmltv.dtd\">\n\
         <tv generator-info-name=\"vidproxy\">\n\
         \x20 <channel id=\"{id}\">\n\
         \x20   <display-name lang=\"es\">{name}</display-name>\n\
         {icon}\
         \x20 </channel>\n\
         {programmes}\
         </tv>\n",
        id = escape_xml(channel_name),
        name = escape_xml(channel_name),
        icon = icon_element,
        programmes = programmes,
    );

    ([(header::CONTENT_TYPE, "application/xml")], xml)
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Serve the HLS playlist, starting the pipeline if needed
async fn playlist_m3u8(State(state): State<AppState>) -> Result<Response, StatusCode> {
    // Ensure pipeline is running
    state.pipeline_manager.ensure_running().await.map_err(|e| {
        eprintln!("[server] Failed to start pipeline: {}", e);
        StatusCode::SERVICE_UNAVAILABLE
    })?;

    // Wait for first segment to be ready
    state.pipeline_manager.wait_for_ready().await.map_err(|e| {
        eprintln!("[server] Timeout waiting for pipeline: {}", e);
        StatusCode::GATEWAY_TIMEOUT
    })?;

    // Record activity
    state.pipeline_manager.record_activity();

    // Serve the playlist file
    let playlist_path = state.output_dir.join("playlist.m3u8");
    serve_file(&playlist_path, "application/vnd.apple.mpegurl").await
}

/// Serve a segment file and track activity
async fn serve_segment(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> Result<Response, StatusCode> {
    // Record activity for idle tracking
    state.pipeline_manager.record_activity();

    // Serve the segment file
    let segment_path = state.output_dir.join(&filename);
    serve_file(&segment_path, "video/mp2t").await
}

/// Helper to serve a file with given content type
async fn serve_file(path: &std::path::Path, content_type: &str) -> Result<Response, StatusCode> {
    let file = tokio::fs::File::open(path).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            StatusCode::NOT_FOUND
        } else {
            eprintln!("[server] Error opening file {:?}: {}", path, e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .body(body)
        .unwrap())
}

/**
    Run the HTTP server that serves HLS content.
*/
pub async fn run_server(
    addr: SocketAddr,
    pipeline_manager: Arc<PipelineManager>,
    mut shutdown_rx: watch::Receiver<bool>,
    channel_name: &str,
    stream_info_rx: StreamInfoReceiver,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let output_dir = pipeline_manager.output_dir().to_path_buf();

    let state = AppState {
        fallback_channel_name: channel_name.to_string(),
        stream_info_rx,
        pipeline_manager,
        output_dir,
    };

    let app = Router::new()
        .route("/channels.m3u", get(channels_m3u))
        .route("/epg.xml", get(epg_xml))
        .route("/playlist.m3u8", get(playlist_m3u8))
        .route("/{filename}", get(serve_segment))
        .with_state(state);

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
