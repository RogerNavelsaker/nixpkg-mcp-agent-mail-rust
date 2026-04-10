//! Utilities for capturing and analyzing console output in tests.
//!
//! This module provides tools for:
//! - Capturing stdout/stderr during test execution
//! - Detecting ANSI escape codes in output
//! - Asserting output contains expected content
//! - Measuring output timing
//!
//! # Example
//!
//! ```rust,ignore
//! use crate::e2e::output_capture::CapturedOutput;
//!
//! let output = CapturedOutput::from_strings(
//!     "Hello, world!".to_string(),
//!     "Status message".to_string(),
//! );
//!
//! output.assert_stdout_contains("Hello");
//! output.assert_plain_mode_clean();
//! ```

use std::time::Duration;

/// Captured output from a test run.
///
/// Contains stdout, stderr, timing, and helper methods for assertions.
#[derive(Debug, Clone)]
pub struct CapturedOutput {
    /// Captured stdout content.
    pub stdout: String,
    /// Captured stderr content.
    pub stderr: String,
    /// How long the capture took.
    pub duration: Duration,
}

impl CapturedOutput {
    /// Create a CapturedOutput from strings (for testing).
    #[must_use]
    pub fn from_strings(stdout: String, stderr: String) -> Self {
        Self {
            stdout,
            stderr,
            duration: Duration::ZERO,
        }
    }

    /// Create with timing information.
    #[must_use]
    pub fn with_duration(stdout: String, stderr: String, duration: Duration) -> Self {
        Self {
            stdout,
            stderr,
            duration,
        }
    }

    /// Check if stdout contains ANSI escape codes.
    #[must_use]
    pub fn stdout_has_ansi(&self) -> bool {
        has_ansi_codes(&self.stdout)
    }

    /// Check if stderr contains ANSI escape codes.
    #[must_use]
    pub fn stderr_has_ansi(&self) -> bool {
        has_ansi_codes(&self.stderr)
    }

    /// Check if any output contains ANSI escape codes.
    #[must_use]
    pub fn has_any_ansi(&self) -> bool {
        self.stdout_has_ansi() || self.stderr_has_ansi()
    }

    /// Get stdout as lines.
    #[must_use]
    pub fn stdout_lines(&self) -> Vec<&str> {
        self.stdout.lines().collect()
    }

    /// Get stderr as lines.
    #[must_use]
    pub fn stderr_lines(&self) -> Vec<&str> {
        self.stderr.lines().collect()
    }

    /// Get duration in milliseconds.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn duration_ms(&self) -> u64 {
        self.duration.as_millis() as u64
    }

    /// Assert stdout matches expected exactly.
    ///
    /// # Panics
    ///
    /// Panics if stdout doesn't match expected, with detailed diff output.
    pub fn assert_stdout_eq(&self, expected: &str) {
        if self.stdout != expected {
            eprintln!("=== STDOUT MISMATCH ===");
            eprintln!("Expected ({} bytes):", expected.len());
            for (i, line) in expected.lines().enumerate() {
                eprintln!("  {:3}: {}", i + 1, line);
            }
            eprintln!("Actual ({} bytes):", self.stdout.len());
            for (i, line) in self.stdout.lines().enumerate() {
                eprintln!("  {:3}: {}", i + 1, line);
            }
            eprintln!("=== END MISMATCH ===");
            panic!("stdout mismatch");
        }
    }

    /// Assert stdout contains a substring.
    ///
    /// # Panics
    ///
    /// Panics if stdout doesn't contain the substring.
    pub fn assert_stdout_contains(&self, substring: &str) {
        if !self.stdout.contains(substring) {
            eprintln!("=== STDOUT MISSING SUBSTRING ===");
            eprintln!("Looking for: {substring}");
            eprintln!("Full stdout ({} bytes):", self.stdout.len());
            for line in self.stdout.lines() {
                eprintln!("  {line}");
            }
            eprintln!("=== END ===");
            panic!("stdout missing expected substring: {substring}");
        }
    }

    /// Assert stderr contains a substring.
    ///
    /// # Panics
    ///
    /// Panics if stderr doesn't contain the substring.
    pub fn assert_stderr_contains(&self, substring: &str) {
        if !self.stderr.contains(substring) {
            eprintln!("=== STDERR MISSING SUBSTRING ===");
            eprintln!("Looking for: {substring}");
            eprintln!("Full stderr ({} bytes):", self.stderr.len());
            for line in self.stderr.lines() {
                eprintln!("  {line}");
            }
            eprintln!("=== END ===");
            panic!("stderr missing expected substring: {substring}");
        }
    }

    /// Assert stdout does NOT contain a substring.
    ///
    /// # Panics
    ///
    /// Panics if stdout contains the substring.
    pub fn assert_stdout_not_contains(&self, substring: &str) {
        if self.stdout.contains(substring) {
            eprintln!("=== STDOUT CONTAINS UNWANTED SUBSTRING ===");
            eprintln!("Unwanted: {substring}");
            eprintln!("Full stdout:");
            for line in self.stdout.lines() {
                eprintln!("  {line}");
            }
            eprintln!("=== END ===");
            panic!("stdout contains unwanted substring: {substring}");
        }
    }

    /// Assert no ANSI escape codes in stdout (for plain mode validation).
    ///
    /// # Panics
    ///
    /// Panics if ANSI codes are found in stdout.
    pub fn assert_plain_mode_clean(&self) {
        if self.stdout_has_ansi() {
            eprintln!("=== ANSI CODES FOUND IN PLAIN MODE ===");
            eprintln!("stdout bytes: {:?}", self.stdout.as_bytes());
            let ansi_locations = find_ansi_locations(&self.stdout);
            eprintln!("ANSI code locations: {ansi_locations:?}");
            eprintln!("=== END ===");
            panic!("plain mode should have no ANSI codes in stdout");
        }
    }

    /// Assert no ANSI escape codes in stderr.
    ///
    /// # Panics
    ///
    /// Panics if ANSI codes are found in stderr.
    pub fn assert_stderr_plain(&self) {
        if self.stderr_has_ansi() {
            eprintln!("=== ANSI CODES FOUND IN STDERR ===");
            eprintln!("stderr bytes: {:?}", self.stderr.as_bytes());
            eprintln!("=== END ===");
            panic!("stderr should have no ANSI codes");
        }
    }

    /// Assert both streams have no ANSI codes.
    pub fn assert_all_plain(&self) {
        self.assert_plain_mode_clean();
        self.assert_stderr_plain();
    }

    /// Assert output completed within time limit.
    ///
    /// # Panics
    ///
    /// Panics if duration exceeds the limit.
    pub fn assert_duration_under(&self, max_ms: u64) {
        let actual_ms = self.duration_ms();
        assert!(
            actual_ms <= max_ms,
            "Output took {actual_ms}ms, expected under {max_ms}ms"
        );
    }
}

/// Check if a string contains ANSI escape codes.
///
/// Checks for:
/// - CSI sequences: `\x1b[`
/// - OSC sequences: `\x1b]`
/// - DCS sequences: `\x1bP`
/// - C1 CSI: `\u{009b}`
#[must_use]
pub fn has_ansi_codes(s: &str) -> bool {
    s.contains("\x1b[")
        || s.contains("\x1b]")
        || s.contains("\x1bP")
        || s.contains("\u{009b}")
        || s.contains("\x1b\\")
}

/// Find byte positions of ANSI escape sequences.
///
/// Returns a list of (position, sequence_start) tuples.
#[must_use]
pub fn find_ansi_locations(s: &str) -> Vec<(usize, String)> {
    let mut locations = Vec::new();
    let bytes = s.as_bytes();

    for (i, window) in bytes.windows(2).enumerate() {
        if window[0] == 0x1b {
            // Found escape character
            let seq_start: String = bytes[i..std::cmp::min(i + 6, bytes.len())]
                .iter()
                .map(|&b| if b.is_ascii_graphic() { b as char } else { '.' })
                .collect();
            locations.push((i, seq_start));
        }
    }

    locations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_captured_output_creation() {
        let output = CapturedOutput::from_strings(
            "stdout content".to_string(),
            "stderr content".to_string(),
        );

        assert_eq!(output.stdout, "stdout content");
        assert_eq!(output.stderr, "stderr content");
        assert_eq!(output.duration_ms(), 0);
    }

    #[test]
    fn test_captured_output_with_duration() {
        let output = CapturedOutput::with_duration(
            "test".to_string(),
            String::new(),
            Duration::from_millis(100),
        );

        assert_eq!(output.duration_ms(), 100);
    }

    #[test]
    fn test_stdout_lines() {
        let output = CapturedOutput::from_strings("line1\nline2\nline3".to_string(), String::new());

        assert_eq!(output.stdout_lines(), vec!["line1", "line2", "line3"]);
    }

    #[test]
    fn test_has_ansi_codes_positive() {
        assert!(has_ansi_codes("\x1b[31mred\x1b[0m"));
        assert!(has_ansi_codes("text\x1b[1mbold\x1b[0m"));
    }

    #[test]
    fn test_has_ansi_codes_negative() {
        assert!(!has_ansi_codes("plain text"));
        assert!(!has_ansi_codes("no escape codes here"));
        assert!(!has_ansi_codes(""));
    }

    #[test]
    fn test_ansi_detection_in_output() {
        let plain =
            CapturedOutput::from_strings("plain text".to_string(), "plain stderr".to_string());
        assert!(!plain.stdout_has_ansi());
        assert!(!plain.stderr_has_ansi());
        assert!(!plain.has_any_ansi());

        let with_ansi =
            CapturedOutput::from_strings("\x1b[31mred\x1b[0m".to_string(), String::new());
        assert!(with_ansi.stdout_has_ansi());
        assert!(with_ansi.has_any_ansi());
    }

    #[test]
    fn test_assert_stdout_contains() {
        let output = CapturedOutput::from_strings("Hello, world!".to_string(), String::new());

        output.assert_stdout_contains("Hello");
        output.assert_stdout_contains("world");
        output.assert_stdout_contains(", ");
    }

    #[test]
    #[should_panic(expected = "stdout missing expected substring")]
    fn test_assert_stdout_contains_fails() {
        let output = CapturedOutput::from_strings("Hello, world!".to_string(), String::new());

        output.assert_stdout_contains("goodbye");
    }

    #[test]
    fn test_assert_stdout_not_contains() {
        let output = CapturedOutput::from_strings("Hello, world!".to_string(), String::new());

        output.assert_stdout_not_contains("goodbye");
        output.assert_stdout_not_contains("ANSI");
    }

    #[test]
    #[should_panic(expected = "stdout contains unwanted substring")]
    fn test_assert_stdout_not_contains_fails() {
        let output = CapturedOutput::from_strings("Hello, world!".to_string(), String::new());

        output.assert_stdout_not_contains("Hello");
    }

    #[test]
    fn test_assert_plain_mode_clean() {
        let output = CapturedOutput::from_strings(
            "Plain text without escape codes".to_string(),
            String::new(),
        );

        output.assert_plain_mode_clean();
    }

    #[test]
    #[should_panic(expected = "plain mode should have no ANSI codes")]
    fn test_assert_plain_mode_clean_fails() {
        let output =
            CapturedOutput::from_strings("\x1b[31mred text\x1b[0m".to_string(), String::new());

        output.assert_plain_mode_clean();
    }

    #[test]
    fn test_find_ansi_locations() {
        let s = "text\x1b[31mred\x1b[0mmore";
        let locations = find_ansi_locations(s);

        assert_eq!(locations.len(), 2);
        assert_eq!(locations[0].0, 4); // First escape at position 4
        // Second escape: "text" (4) + "\x1b[31m" (5) + "red" (3) = 12
        assert_eq!(locations[1].0, 12);
    }

    #[test]
    fn test_assert_duration_under() {
        let output =
            CapturedOutput::with_duration(String::new(), String::new(), Duration::from_millis(50));

        output.assert_duration_under(100);
        output.assert_duration_under(51);
    }

    #[test]
    #[should_panic(expected = "expected under")]
    fn test_assert_duration_under_fails() {
        let output =
            CapturedOutput::with_duration(String::new(), String::new(), Duration::from_millis(100));

        output.assert_duration_under(50);
    }
}
