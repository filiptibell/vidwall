/// Configuration for a video grid layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridConfig {
    /// Number of columns in the grid
    pub cols: u32,
    /// Number of rows in the grid
    pub rows: u32,
}

/// All candidate grid configurations.
/// Constraints: max 4 videos total, max 3 on any axis.
const CANDIDATE_GRIDS: &[(u32, u32)] = &[
    (1, 1), // 1 video
    (2, 1), // 2 videos, side by side
    (1, 2), // 2 videos, stacked
    (2, 2), // 4 videos, 2x2
    (3, 1), // 3 videos, side by side
    (1, 3), // 3 videos, stacked
];

impl GridConfig {
    /// Create a new grid configuration.
    pub fn new(cols: u32, rows: u32) -> Self {
        Self { cols, rows }
    }

    /// Get the total number of slots in this grid.
    pub fn total_slots(&self) -> u32 {
        self.cols * self.rows
    }

    /// Calculate the aspect ratio of this grid, assuming 16:9 videos in each cell.
    pub fn aspect_ratio(&self) -> f32 {
        (self.cols as f32 * 16.0) / (self.rows as f32 * 9.0)
    }

    /// Find the optimal grid configuration for the given window dimensions.
    ///
    /// The algorithm:
    /// 1. Calculate the window's aspect ratio
    /// 2. For each candidate grid, calculate how close its aspect ratio is to the window
    /// 3. Select the grid with the smallest difference
    /// 4. Tie-breaker: prefer grids with more videos
    pub fn optimal_for_window(width: f32, height: f32) -> Self {
        let window_ratio = width / height;

        let mut best_config = GridConfig::new(2, 2); // Default fallback
        let mut best_score = f32::MAX;
        let mut best_slots = 0u32;

        for &(cols, rows) in CANDIDATE_GRIDS {
            let config = GridConfig::new(cols, rows);
            let grid_ratio = config.aspect_ratio();

            // Score is the absolute difference in aspect ratios
            let score = (window_ratio - grid_ratio).abs();

            // Select if better score, or same score but more videos
            if score < best_score || (score == best_score && config.total_slots() > best_slots) {
                best_config = config;
                best_score = score;
                best_slots = config.total_slots();
            }
        }

        best_config
    }
}

impl Default for GridConfig {
    fn default() -> Self {
        Self::new(2, 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aspect_ratios() {
        // 1x1 grid = 16:9 = 1.78
        assert!((GridConfig::new(1, 1).aspect_ratio() - 1.778).abs() < 0.01);

        // 2x2 grid = 32:18 = 16:9 = 1.78
        assert!((GridConfig::new(2, 2).aspect_ratio() - 1.778).abs() < 0.01);

        // 2x1 grid = 32:9 = 3.56
        assert!((GridConfig::new(2, 1).aspect_ratio() - 3.556).abs() < 0.01);

        // 1x2 grid = 16:18 = 0.89
        assert!((GridConfig::new(1, 2).aspect_ratio() - 0.889).abs() < 0.01);

        // 3x1 grid = 48:9 = 5.33
        assert!((GridConfig::new(3, 1).aspect_ratio() - 5.333).abs() < 0.01);

        // 1x3 grid = 16:27 = 0.59
        assert!((GridConfig::new(1, 3).aspect_ratio() - 0.593).abs() < 0.01);
    }

    #[test]
    fn test_optimal_for_16_9_window() {
        // 16:9 window should prefer 2x2 (more videos) over 1x1
        let config = GridConfig::optimal_for_window(1920.0, 1080.0);
        assert_eq!(config.cols, 2);
        assert_eq!(config.rows, 2);
    }

    #[test]
    fn test_optimal_for_wide_window() {
        // Very wide window (32:9) should prefer 2x1
        let config = GridConfig::optimal_for_window(3200.0, 900.0);
        assert_eq!(config.cols, 2);
        assert_eq!(config.rows, 1);
    }

    #[test]
    fn test_optimal_for_tall_window() {
        // Tall window (16:18) should prefer 1x2
        let config = GridConfig::optimal_for_window(800.0, 900.0);
        assert_eq!(config.cols, 1);
        assert_eq!(config.rows, 2);
    }

    #[test]
    fn test_optimal_for_ultrawide() {
        // Ultra-wide window (48:9) should prefer 3x1
        let config = GridConfig::optimal_for_window(4800.0, 900.0);
        assert_eq!(config.cols, 3);
        assert_eq!(config.rows, 1);
    }

    #[test]
    fn test_optimal_for_very_tall() {
        // Very tall window (16:27) should prefer 1x3
        let config = GridConfig::optimal_for_window(800.0, 1350.0);
        assert_eq!(config.cols, 1);
        assert_eq!(config.rows, 3);
    }
}
