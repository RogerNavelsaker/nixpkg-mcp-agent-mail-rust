//! End-to-End Test Suite for sqlmodel-console
//!
//! This is the main test harness that includes all e2e test modules.
//!
//! # Running Tests
//!
//! ```bash
//! # Run all e2e tests (single-threaded for env var safety)
//! cargo test -p sqlmodel-console --test e2e_suite -- --test-threads=1
//!
//! # Run with verbose output
//! cargo test -p sqlmodel-console --test e2e_suite -- --test-threads=1 --nocapture
//!
//! # Run specific module
//! cargo test -p sqlmodel-console --test e2e_suite mode_switching
//! cargo test -p sqlmodel-console --test e2e_suite error_display
//! cargo test -p sqlmodel-console --test e2e_suite full_workflow
//! ```

// Include the e2e module directory
#[path = "e2e/mod.rs"]
mod e2e;

// Re-export tests so they're discoverable
pub use e2e::*;
