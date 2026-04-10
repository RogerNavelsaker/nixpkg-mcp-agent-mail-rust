//! Traits for console-aware components.
//!
//! This module defines traits that allow database connections, pools, and other
//! components to receive and use console output capabilities.
//!
//! # Example
//!
//! ```rust
//! use sqlmodel_console::{ConsoleAware, SqlModelConsole};
//! use std::sync::Arc;
//!
//! struct MyConnection {
//!     console: Option<Arc<SqlModelConsole>>,
//! }
//!
//! impl ConsoleAware for MyConnection {
//!     fn set_console(&mut self, console: Option<Arc<SqlModelConsole>>) {
//!         self.console = console;
//!     }
//!
//!     fn console(&self) -> Option<&Arc<SqlModelConsole>> {
//!         self.console.as_ref()
//!     }
//! }
//!
//! let mut conn = MyConnection { console: None };
//! let console = Arc::new(SqlModelConsole::new());
//! conn.set_console(Some(console));
//!
//! // Now the connection can emit rich output
//! conn.emit_status("Connecting...");
//! ```

use std::sync::Arc;

use crate::SqlModelConsole;

/// Trait for components that can accept a console for rich output.
///
/// Implementing this trait allows database connections, pools, and other
/// components to emit styled console output when a console is attached.
///
/// The trait uses `Arc<SqlModelConsole>` to allow sharing a single console
/// across multiple components without lifetime complications.
///
/// # Design Notes
///
/// - **Optional attachment**: Components work without a console attached,
///   silently ignoring output calls. This makes console support opt-in.
///
/// - **Thread-safe sharing**: Using `Arc` allows the same console to be
///   shared across threads and async tasks.
///
/// - **Default method implementations**: The `emit_*` methods have default
///   implementations that only require `set_console` and `console` to be defined.
///
/// # Example
///
/// ```rust
/// use sqlmodel_console::{ConsoleAware, SqlModelConsole, OutputMode};
/// use std::sync::Arc;
///
/// struct DatabasePool {
///     console: Option<Arc<SqlModelConsole>>,
///     connections: Vec<String>,
/// }
///
/// impl ConsoleAware for DatabasePool {
///     fn set_console(&mut self, console: Option<Arc<SqlModelConsole>>) {
///         self.console = console;
///     }
///
///     fn console(&self) -> Option<&Arc<SqlModelConsole>> {
///         self.console.as_ref()
///     }
/// }
///
/// let mut pool = DatabasePool {
///     console: None,
///     connections: Vec::new(),
/// };
///
/// // No console attached - emit calls are silently ignored
/// pool.emit_status("Starting pool...");
///
/// // Attach a console
/// let console = Arc::new(SqlModelConsole::with_mode(OutputMode::Plain));
/// pool.set_console(Some(console));
///
/// // Now emit calls produce output
/// pool.emit_success("Pool ready with 5 connections");
/// ```
pub trait ConsoleAware {
    /// Attach or detach a console.
    ///
    /// Pass `Some(console)` to enable rich output.
    /// Pass `None` to disable console output.
    fn set_console(&mut self, console: Option<Arc<SqlModelConsole>>);

    /// Get reference to the attached console, if any.
    fn console(&self) -> Option<&Arc<SqlModelConsole>>;

    /// Check if a console is attached.
    ///
    /// This is a convenience method that returns `true` if a console
    /// is currently attached.
    fn has_console(&self) -> bool {
        self.console().is_some()
    }

    /// Emit a status message if console is attached.
    ///
    /// Status messages are informational and go to stderr.
    /// If no console is attached, this is a no-op.
    fn emit_status(&self, message: &str) {
        if let Some(console) = self.console() {
            console.status(message);
        }
    }

    /// Emit a success message if console is attached.
    ///
    /// Success messages indicate successful completion of an operation.
    /// If no console is attached, this is a no-op.
    fn emit_success(&self, message: &str) {
        if let Some(console) = self.console() {
            console.success(message);
        }
    }

    /// Emit an error message if console is attached.
    ///
    /// Error messages indicate failures or problems.
    /// If no console is attached, this is a no-op.
    fn emit_error(&self, message: &str) {
        if let Some(console) = self.console() {
            console.error(message);
        }
    }

    /// Emit a warning message if console is attached.
    ///
    /// Warning messages indicate potential issues that don't prevent operation.
    /// If no console is attached, this is a no-op.
    fn emit_warning(&self, message: &str) {
        if let Some(console) = self.console() {
            console.warning(message);
        }
    }

    /// Emit an info message if console is attached.
    ///
    /// Info messages provide helpful information to the user.
    /// If no console is attached, this is a no-op.
    fn emit_info(&self, message: &str) {
        if let Some(console) = self.console() {
            console.info(message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OutputMode;

    /// Mock connection for testing ConsoleAware trait.
    struct MockConnection {
        console: Option<Arc<SqlModelConsole>>,
    }

    impl MockConnection {
        fn new() -> Self {
            Self { console: None }
        }
    }

    impl ConsoleAware for MockConnection {
        fn set_console(&mut self, console: Option<Arc<SqlModelConsole>>) {
            self.console = console;
        }

        fn console(&self) -> Option<&Arc<SqlModelConsole>> {
            self.console.as_ref()
        }
    }

    #[test]
    fn test_has_console_false_initially() {
        let conn = MockConnection::new();
        assert!(!conn.has_console());
    }

    #[test]
    fn test_has_console_true_after_set() {
        let mut conn = MockConnection::new();
        let console = Arc::new(SqlModelConsole::with_mode(OutputMode::Plain));
        conn.set_console(Some(console));
        assert!(conn.has_console());
    }

    #[test]
    fn test_set_console_none_detaches() {
        let mut conn = MockConnection::new();
        let console = Arc::new(SqlModelConsole::with_mode(OutputMode::Plain));
        conn.set_console(Some(console));
        assert!(conn.has_console());
        conn.set_console(None);
        assert!(!conn.has_console());
    }

    #[test]
    fn test_console_returns_reference() {
        let mut conn = MockConnection::new();
        let console = Arc::new(SqlModelConsole::with_mode(OutputMode::Plain));
        conn.set_console(Some(console.clone()));

        let returned = conn.console().unwrap();
        // Verify it's the same console (mode matches)
        assert!(returned.is_plain());
    }

    #[test]
    fn test_emit_methods_no_panic_without_console() {
        let conn = MockConnection::new();
        // These should not panic even without a console
        conn.emit_status("test status");
        conn.emit_success("test success");
        conn.emit_error("test error");
        conn.emit_warning("test warning");
        conn.emit_info("test info");
    }

    #[test]
    fn test_emit_methods_with_console() {
        let mut conn = MockConnection::new();
        let console = Arc::new(SqlModelConsole::with_mode(OutputMode::Plain));
        conn.set_console(Some(console));

        // These should not panic with console attached
        // (output goes to stderr but we can't easily capture it)
        conn.emit_status("connecting to database");
        conn.emit_success("connection established");
        conn.emit_error("query failed");
        conn.emit_warning("deprecated feature used");
        conn.emit_info("using connection pool");
    }

    #[test]
    fn test_shared_console_across_components() {
        let console = Arc::new(SqlModelConsole::with_mode(OutputMode::Json));

        let mut conn1 = MockConnection::new();
        let mut conn2 = MockConnection::new();

        conn1.set_console(Some(console.clone()));
        conn2.set_console(Some(console.clone()));

        // Both connections share the same console
        assert!(conn1.console().unwrap().is_json());
        assert!(conn2.console().unwrap().is_json());

        // Arc reference count is 3 (original + 2 connections)
        assert_eq!(Arc::strong_count(&console), 3);
    }

    #[test]
    fn test_console_none_returns_none() {
        let conn = MockConnection::new();
        assert!(conn.console().is_none());
    }
}
