use std::sync::Arc;
use std::time::Instant;

use std::rc::Rc;

use gpui::{
    Context, Entity, IntoElement, Pixels, PlatformDisplay, Point, Render, Size, Window, div,
    prelude::*, rgb,
};

use crate::video::ReadyVideos;
use crate::window_state::WindowState;

use super::grid_config::GridConfig;
use super::grid_view::GridView;

/// Minimum time between window state saves (to avoid excessive disk writes during resize)
const SAVE_DEBOUNCE_SECS: f32 = 1.0;

/// The root view of the application.
///
/// Contains the video grid and handles window resize events to reconfigure the grid.
pub struct RootView {
    grid: Entity<GridView>,
    ready_videos: Arc<ReadyVideos>,
    last_size: Option<Size<Pixels>>,
    last_origin: Option<Point<Pixels>>,
    last_save_time: Option<Instant>,
}

impl RootView {
    /// Create a new root view with the given ready videos storage.
    pub fn new(ready_videos: Arc<ReadyVideos>, cx: &mut Context<Self>) -> Self {
        let ready_videos_clone = Arc::clone(&ready_videos);
        let grid = cx.new(|cx| GridView::new(ready_videos_clone, cx));

        Self {
            grid,
            ready_videos,
            last_size: None,
            last_origin: None,
            last_save_time: None,
        }
    }

    /// Get the grid view entity.
    pub fn grid(&self) -> &Entity<GridView> {
        &self.grid
    }

    /// Handle window resize by reconfiguring the grid if needed.
    fn handle_resize(&mut self, size: Size<Pixels>, cx: &mut Context<Self>) {
        // Calculate optimal grid for new size
        let new_config = GridConfig::optimal_for_window(size.width.into(), size.height.into());

        // Reconfigure grid if needed
        self.grid.update(cx, |grid, cx| {
            grid.reconfigure(new_config, cx);
        });
    }

    /// Save window state to disk (debounced).
    ///
    /// Uses window origin from bounds() but content size from viewport_size()
    /// to avoid the title bar height being added on each restore.
    fn maybe_save_window_state(
        &mut self,
        display: Option<Rc<dyn PlatformDisplay>>,
        origin: Point<Pixels>,
        size: Size<Pixels>,
    ) {
        // Check if state actually changed
        if self.last_size == Some(size) && self.last_origin == Some(origin) {
            return;
        }
        self.last_size = Some(size);
        self.last_origin = Some(origin);

        // Debounce saves
        let now = Instant::now();
        if let Some(last_save) = self.last_save_time {
            if now.duration_since(last_save).as_secs_f32() < SAVE_DEBOUNCE_SECS {
                return;
            }
        }

        // Save the state (display UUID, origin, and content size)
        let state = WindowState::new(display, origin, size);
        if let Err(e) = state.save() {
            eprintln!("Failed to save window state: {}", e);
        }
        self.last_save_time = Some(now);
    }

    /// Try to fill the grid with videos (called when videos become available).
    pub fn try_fill_grid(&mut self, cx: &mut Context<Self>) {
        self.grid.update(cx, |grid, cx| {
            grid.fill_empty_slots(cx);
        });
    }
}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Get current content size, window origin, and display
        let size = window.viewport_size();
        let origin = window.bounds().origin;
        let display = window.display(&*cx);

        // Check if size changed (for grid reconfiguration)
        if self.last_size != Some(size) {
            self.handle_resize(size, cx);
        }

        // Save window state (display + origin + content size)
        self.maybe_save_window_state(display, origin, size);

        div()
            .id("root")
            .size_full()
            .bg(rgb(0x000000))
            .child(self.grid.clone())
    }
}
