//! Indeterminate spinner for unknown-length operations.
//!
//! `IndeterminateSpinner` shows activity feedback when the total count or duration
//! is not known. Useful for connection establishment, complex queries, etc.
//!
//! # Example
//!
//! ```rust
//! use sqlmodel_console::renderables::{IndeterminateSpinner, SpinnerStyle};
//!
//! let spinner = IndeterminateSpinner::new("Connecting to database")
//!     .style(SpinnerStyle::Dots);
//!
//! // Plain text: "[...] Connecting to database (2.3s)"
//! println!("{}", spinner.render_plain());
//! ```

use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::OperationProgress;
use crate::theme::Theme;

/// Spinner animation style.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpinnerStyle {
    /// Three dots cycling: ".", "..", "..."
    #[default]
    Dots,
    /// Unicode braille pattern animation
    Braille,
    /// Rotating line: -, \, |, /
    Line,
    /// Rotating arrow
    Arrow,
    /// Simple asterisk blinking
    Simple,
}

impl SpinnerStyle {
    /// Get the animation frames for this style.
    #[must_use]
    pub fn frames(&self) -> &'static [&'static str] {
        match self {
            Self::Dots => &[".", "..", "...", ".."],
            Self::Braille => &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            Self::Line => &["-", "\\", "|", "/"],
            Self::Arrow => &["←", "↖", "↑", "↗", "→", "↘", "↓", "↙"],
            Self::Simple => &["*", " "],
        }
    }

    /// Get the interval between frames in milliseconds.
    #[must_use]
    pub const fn interval_ms(&self) -> u64 {
        match self {
            Self::Dots => 250,
            Self::Braille => 80,
            Self::Line => 100,
            Self::Arrow => 120,
            Self::Simple => 500,
        }
    }

    /// Get the frame for a given elapsed time.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // frame_index is bounded by frames.len()
    pub fn frame_at(&self, elapsed_ms: u64) -> &'static str {
        let frames = self.frames();
        let interval = self.interval_ms();
        let frame_index = ((elapsed_ms / interval) as usize) % frames.len();
        frames[frame_index]
    }
}

/// A spinner for operations with unknown total count or duration.
///
/// Shows activity with elapsed time, useful for:
/// - Establishing database connections
/// - Running complex queries
/// - Waiting for locks
/// - Initial data discovery
///
/// # Rendering Modes
///
/// - **Rich mode**: Animated spinner with message and elapsed time
/// - **Plain mode**: Static `[...] message (elapsed)` format for agents
/// - **JSON mode**: Structured data for programmatic consumption
///
/// # Example
///
/// ```rust
/// use sqlmodel_console::renderables::{IndeterminateSpinner, SpinnerStyle};
///
/// let spinner = IndeterminateSpinner::new("Loading data")
///     .style(SpinnerStyle::Braille);
///
/// // Can convert to progress bar when total becomes known
/// let progress = spinner.into_progress(1000);
/// ```
#[derive(Debug, Clone)]
pub struct IndeterminateSpinner {
    /// Status message to display
    message: String,
    /// When the spinner started
    started_at: Instant,
    /// Animation style
    style: SpinnerStyle,
    /// Optional theme for styling
    theme: Option<Theme>,
}

impl IndeterminateSpinner {
    /// Create a new spinner with a message.
    ///
    /// # Arguments
    /// - `message`: Status message describing the operation
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            started_at: Instant::now(),
            style: SpinnerStyle::default(),
            theme: None,
        }
    }

    /// Set the animation style.
    #[must_use]
    pub fn style(mut self, style: SpinnerStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the theme for styled output.
    #[must_use]
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Update the status message.
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }

    /// Get the current message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Get the current style.
    #[must_use]
    pub fn current_style(&self) -> SpinnerStyle {
        self.style
    }

    /// Reset the start time.
    pub fn reset_timer(&mut self) {
        self.started_at = Instant::now();
    }

    /// Get elapsed time in seconds.
    #[must_use]
    pub fn elapsed_secs(&self) -> f64 {
        self.started_at.elapsed().as_secs_f64()
    }

    /// Get elapsed time in milliseconds.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // milliseconds won't overflow u64 for practical durations
    pub fn elapsed_ms(&self) -> u64 {
        self.started_at.elapsed().as_millis() as u64
    }

    /// Get the current animation frame.
    #[must_use]
    pub fn current_frame(&self) -> &'static str {
        self.style.frame_at(self.elapsed_ms())
    }

    /// Convert to a progress bar when total becomes known.
    ///
    /// The progress bar inherits the spinner's message as the operation name
    /// and starts with 0 completed items.
    ///
    /// # Arguments
    /// - `total`: The total number of items to process
    #[must_use]
    pub fn into_progress(self, total: u64) -> OperationProgress {
        let mut progress = OperationProgress::new(self.message, total);
        if let Some(theme) = self.theme {
            progress = progress.theme(theme);
        }
        progress
    }

    /// Render as plain text for agents.
    ///
    /// Format: `[...] message (elapsed)`
    #[must_use]
    pub fn render_plain(&self) -> String {
        format!(
            "[...] {} ({})",
            self.message,
            format_elapsed(self.elapsed_secs())
        )
    }

    /// Render with ANSI styling and animation.
    ///
    /// Shows the current animation frame with colors.
    #[must_use]
    pub fn render_styled(&self) -> String {
        let theme = self.theme.clone().unwrap_or_default();
        let frame = self.current_frame();

        let color = theme.info.color_code();
        let reset = "\x1b[0m";

        format!(
            "{color}[{frame}]{reset} {} ({})",
            self.message,
            format_elapsed(self.elapsed_secs())
        )
    }

    /// Render as JSON for structured output.
    #[must_use]
    pub fn to_json(&self) -> String {
        #[derive(Serialize)]
        struct SpinnerJson<'a> {
            message: &'a str,
            elapsed_secs: f64,
            style: &'a str,
            frame: &'a str,
        }

        let style_str = match self.style {
            SpinnerStyle::Dots => "dots",
            SpinnerStyle::Braille => "braille",
            SpinnerStyle::Line => "line",
            SpinnerStyle::Arrow => "arrow",
            SpinnerStyle::Simple => "simple",
        };

        let json = SpinnerJson {
            message: &self.message,
            elapsed_secs: self.elapsed_secs(),
            style: style_str,
            frame: self.current_frame(),
        };

        serde_json::to_string(&json).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Format elapsed time as human-readable string.
fn format_elapsed(secs: f64) -> String {
    if secs < 60.0 {
        format!("{secs:.1}s")
    } else if secs < 3600.0 {
        let mins = (secs / 60.0).floor();
        let remaining = secs % 60.0;
        format!("{mins:.0}m{remaining:.0}s")
    } else {
        let hours = (secs / 3600.0).floor();
        let remaining_mins = ((secs % 3600.0) / 60.0).floor();
        format!("{hours:.0}h{remaining_mins:.0}m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_creation() {
        let spinner = IndeterminateSpinner::new("Connecting");
        assert_eq!(spinner.message(), "Connecting");
        assert_eq!(spinner.current_style(), SpinnerStyle::Dots);
    }

    #[test]
    fn test_spinner_all_styles() {
        for style in [
            SpinnerStyle::Dots,
            SpinnerStyle::Braille,
            SpinnerStyle::Line,
            SpinnerStyle::Arrow,
            SpinnerStyle::Simple,
        ] {
            let spinner = IndeterminateSpinner::new("Test").style(style);
            assert_eq!(spinner.current_style(), style);
            // Verify frames exist
            assert!(!spinner.current_frame().is_empty() || style == SpinnerStyle::Simple);
        }
    }

    #[test]
    fn test_spinner_style_frames() {
        assert_eq!(SpinnerStyle::Dots.frames().len(), 4);
        assert_eq!(SpinnerStyle::Braille.frames().len(), 10);
        assert_eq!(SpinnerStyle::Line.frames().len(), 4);
        assert_eq!(SpinnerStyle::Arrow.frames().len(), 8);
        assert_eq!(SpinnerStyle::Simple.frames().len(), 2);
    }

    #[test]
    fn test_spinner_frame_generation() {
        let style = SpinnerStyle::Dots;
        // Frame 0 at 0ms
        assert_eq!(style.frame_at(0), ".");
        // Frame 1 at 250ms
        assert_eq!(style.frame_at(250), "..");
        // Frame 2 at 500ms
        assert_eq!(style.frame_at(500), "...");
        // Frame 3 at 750ms
        assert_eq!(style.frame_at(750), "..");
        // Wraps back to frame 0 at 1000ms
        assert_eq!(style.frame_at(1000), ".");
    }

    #[test]
    fn test_spinner_style_intervals() {
        assert_eq!(SpinnerStyle::Dots.interval_ms(), 250);
        assert_eq!(SpinnerStyle::Braille.interval_ms(), 80);
        assert_eq!(SpinnerStyle::Line.interval_ms(), 100);
        assert_eq!(SpinnerStyle::Arrow.interval_ms(), 120);
        assert_eq!(SpinnerStyle::Simple.interval_ms(), 500);
    }

    #[test]
    fn test_spinner_elapsed_time() {
        let spinner = IndeterminateSpinner::new("Test");
        // Elapsed time should be very small initially
        assert!(spinner.elapsed_secs() < 1.0);
        assert!(spinner.elapsed_ms() < 1000);
    }

    #[test]
    fn test_spinner_render_plain() {
        let spinner = IndeterminateSpinner::new("Connecting to database");
        let plain = spinner.render_plain();

        assert!(plain.starts_with("[...]"));
        assert!(plain.contains("Connecting to database"));
        assert!(plain.contains('s')); // Contains elapsed time with 's'
    }

    #[test]
    fn test_spinner_render_styled() {
        let spinner = IndeterminateSpinner::new("Loading").style(SpinnerStyle::Dots);
        let styled = spinner.render_styled();

        assert!(styled.contains('['));
        assert!(styled.contains(']'));
        assert!(styled.contains("Loading"));
        assert!(styled.contains('\x1b')); // Contains ANSI codes
    }

    #[test]
    fn test_spinner_message_update() {
        let mut spinner = IndeterminateSpinner::new("Initial");
        assert_eq!(spinner.message(), "Initial");

        spinner.set_message("Updated");
        assert_eq!(spinner.message(), "Updated");
    }

    #[test]
    fn test_spinner_convert_to_progress() {
        let spinner = IndeterminateSpinner::new("Processing")
            .style(SpinnerStyle::Braille)
            .theme(Theme::default());

        let progress = spinner.into_progress(1000);

        assert_eq!(progress.operation_name(), "Processing");
        assert_eq!(progress.total_count(), 1000);
        assert_eq!(progress.completed_count(), 0);
    }

    #[test]
    fn test_spinner_json_output() {
        let spinner = IndeterminateSpinner::new("Test").style(SpinnerStyle::Line);
        let json = spinner.to_json();

        assert!(json.contains("\"message\":\"Test\""));
        assert!(json.contains("\"style\":\"line\""));
        assert!(json.contains("\"elapsed_secs\""));
        assert!(json.contains("\"frame\""));
    }

    #[test]
    fn test_spinner_with_theme() {
        let theme = Theme::default();
        let spinner = IndeterminateSpinner::new("Test").theme(theme.clone());

        // Verify styled output uses theme colors
        let styled = spinner.render_styled();
        assert!(styled.contains('\x1b')); // Contains ANSI color codes
    }

    #[test]
    fn test_spinner_reset_timer() {
        let mut spinner = IndeterminateSpinner::new("Test");
        std::thread::sleep(std::time::Duration::from_millis(10));

        let elapsed_before = spinner.elapsed_ms();
        spinner.reset_timer();
        let elapsed_after = spinner.elapsed_ms();

        // After reset, elapsed time should be smaller
        assert!(elapsed_after < elapsed_before);
    }

    #[test]
    fn test_format_elapsed_seconds() {
        assert_eq!(format_elapsed(0.1), "0.1s");
        assert_eq!(format_elapsed(5.5), "5.5s");
        assert_eq!(format_elapsed(59.9), "59.9s");
    }

    #[test]
    fn test_format_elapsed_minutes() {
        let result = format_elapsed(90.0);
        assert!(result.contains('m'));
        assert!(result.contains('s'));
    }

    #[test]
    fn test_format_elapsed_hours() {
        let result = format_elapsed(3700.0);
        assert!(result.contains('h'));
        assert!(result.contains('m'));
    }

    #[test]
    fn test_spinner_default_style() {
        let spinner = IndeterminateSpinner::new("Test");
        assert_eq!(spinner.current_style(), SpinnerStyle::Dots);
    }

    #[test]
    fn test_spinner_braille_animation() {
        let style = SpinnerStyle::Braille;
        // Verify braille frames are Unicode braille characters
        let frames = style.frames();
        for frame in frames {
            assert!(frame.chars().all(|c| c.is_alphabetic() || c > '\u{2800}'));
        }
    }

    #[test]
    fn test_spinner_line_animation() {
        let style = SpinnerStyle::Line;
        let expected = ["-", "\\", "|", "/"];
        for (i, frame) in style.frames().iter().enumerate() {
            assert_eq!(*frame, expected[i]);
        }
    }

    #[test]
    fn test_spinner_arrow_animation() {
        let style = SpinnerStyle::Arrow;
        assert_eq!(style.frames().len(), 8);
        // Verify all arrow frames are single characters
        for frame in style.frames() {
            assert_eq!(frame.chars().count(), 1);
        }
    }
}
