//! End-to-End Test Suite for sqlmodel-console
//!
//! This module provides comprehensive e2e tests that validate the entire console
//! system working together, with detailed logging for debugging test failures.
//!
//! # Running Tests
//!
//! ```bash
//! # Run all e2e tests
//! cargo test -p sqlmodel-console --test e2e
//!
//! # Run with logging enabled
//! RUST_LOG=sqlmodel_console=debug cargo test -p sqlmodel-console --test e2e -- --nocapture
//!
//! # Run specific test
//! cargo test -p sqlmodel-console --test e2e mode_switching
//! ```
//!
//! # Test Categories
//!
//! - `output_capture`: Test utilities for capturing and analyzing output
//! - `mode_switching`: Tests for output mode detection and switching
//! - `error_display`: Tests for error panel rendering
//! - `query_results`: Tests for query result table display
//! - `progress_tracking`: Tests for progress bars and spinners
//! - `full_workflow`: Complete workflow tests simulating real usage

pub mod error_display;
pub mod full_workflow;
pub mod mode_switching;
pub mod output_capture;
pub mod progress_tracking;
pub mod query_results;
