//! DDL (Data Definition Language) syntax highlighting for schema output.
//!
//! Provides syntax highlighting for CREATE TABLE, CREATE INDEX, ALTER TABLE,
//! and other DDL statements with dialect-specific keyword support.
//!
//! # Example
//!
//! ```rust
//! use sqlmodel_console::renderables::{DdlDisplay, SqlDialect};
//! use sqlmodel_console::Theme;
//!
//! let ddl = "CREATE TABLE users (
//!     id SERIAL PRIMARY KEY,
//!     name TEXT NOT NULL,
//!     email TEXT UNIQUE
//! );";
//!
//! let display = DdlDisplay::new(ddl)
//!     .dialect(SqlDialect::PostgreSQL)
//!     .line_numbers(true);
//!
//! // Rich mode with syntax highlighting
//! println!("{}", display.render(80));
//!
//! // Plain mode for agents
//! println!("{}", display.render_plain());
//! ```

use crate::renderables::sql_syntax::{SqlHighlighter, SqlSegment, SqlToken};
use crate::theme::Theme;

/// SQL dialect for DDL generation.
///
/// Different databases have different DDL syntax and keywords.
/// This enum determines which dialect-specific keywords to highlight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SqlDialect {
    /// PostgreSQL dialect with SERIAL, BIGSERIAL, array types, etc.
    #[default]
    PostgreSQL,
    /// SQLite dialect with AUTOINCREMENT, special type handling.
    SQLite,
    /// MySQL dialect with AUTO_INCREMENT, ENGINE clause, etc.
    MySQL,
}

impl SqlDialect {
    /// Get dialect-specific DDL keywords.
    #[must_use]
    pub fn ddl_keywords(&self) -> &'static [&'static str] {
        match self {
            Self::PostgreSQL => &[
                // PostgreSQL-specific
                "SERIAL",
                "BIGSERIAL",
                "SMALLSERIAL",
                "RETURNING",
                "INHERITS",
                "PARTITION",
                "TABLESPACE",
                "OWNED",
                "STORAGE",
                "EXCLUDE",
                "DEFERRABLE",
                "INITIALLY",
                "DEFERRED",
                "IMMEDIATE",
                "CONCURRENTLY",
            ],
            Self::SQLite => &[
                // SQLite-specific
                "AUTOINCREMENT",
                "WITHOUT",
                "ROWID",
                "STRICT",
                "VIRTUAL",
                "USING",
                "FTS5",
                "RTREE",
            ],
            Self::MySQL => &[
                // MySQL-specific
                "AUTO_INCREMENT",
                "ENGINE",
                "CHARSET",
                "COLLATE",
                "ROW_FORMAT",
                "COMMENT",
                "PARTITION",
                "PARTITIONS",
                "SUBPARTITION",
                "ALGORITHM",
                "LOCK",
                "UNSIGNED",
                "ZEROFILL",
            ],
        }
    }

    /// Get the dialect name as a string.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PostgreSQL => "PostgreSQL",
            Self::SQLite => "SQLite",
            Self::MySQL => "MySQL",
        }
    }
}

impl std::fmt::Display for SqlDialect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Kind of change for diff highlighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// Line was added.
    Added,
    /// Line was removed.
    Removed,
    /// Line was modified.
    Modified,
}

/// A region of changed lines for diff highlighting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeRegion {
    /// Starting line number (1-indexed).
    pub start_line: usize,
    /// Ending line number (1-indexed, inclusive).
    pub end_line: usize,
    /// Kind of change.
    pub kind: ChangeKind,
}

impl ChangeRegion {
    /// Create a new change region.
    #[must_use]
    pub fn new(start_line: usize, end_line: usize, kind: ChangeKind) -> Self {
        Self {
            start_line,
            end_line,
            kind,
        }
    }

    /// Check if a line number is within this region.
    #[must_use]
    pub fn contains_line(&self, line: usize) -> bool {
        line >= self.start_line && line <= self.end_line
    }
}

/// DDL display for schema output with syntax highlighting.
///
/// Displays DDL statements with optional line numbers, dialect-specific
/// highlighting, and change region highlighting for migration diffs.
#[derive(Debug, Clone)]
pub struct DdlDisplay {
    /// The DDL SQL statement(s).
    sql: String,
    /// SQL dialect for syntax highlighting.
    dialect: SqlDialect,
    /// Theme for coloring.
    theme: Theme,
    /// Whether to show line numbers.
    line_numbers: bool,
    /// Regions to highlight (for diffs).
    change_regions: Vec<ChangeRegion>,
}

impl DdlDisplay {
    /// Create a new DDL display from SQL.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sqlmodel_console::renderables::DdlDisplay;
    ///
    /// let display = DdlDisplay::new("CREATE TABLE users (id INT);");
    /// ```
    #[must_use]
    pub fn new(sql: impl Into<String>) -> Self {
        Self {
            sql: sql.into(),
            dialect: SqlDialect::default(),
            theme: Theme::default(),
            line_numbers: false,
            change_regions: Vec::new(),
        }
    }

    /// Set the SQL dialect for highlighting.
    #[must_use]
    pub fn dialect(mut self, dialect: SqlDialect) -> Self {
        self.dialect = dialect;
        self
    }

    /// Set whether to show line numbers.
    #[must_use]
    pub fn line_numbers(mut self, show: bool) -> Self {
        self.line_numbers = show;
        self
    }

    /// Set the theme for coloring.
    #[must_use]
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Add change regions for diff highlighting.
    #[must_use]
    pub fn highlight_changes(mut self, regions: Vec<ChangeRegion>) -> Self {
        self.change_regions = regions;
        self
    }

    /// Add a single change region.
    #[must_use]
    pub fn add_change(mut self, region: ChangeRegion) -> Self {
        self.change_regions.push(region);
        self
    }

    /// Get the SQL content.
    #[must_use]
    pub fn sql(&self) -> &str {
        &self.sql
    }

    /// Get the dialect.
    #[must_use]
    pub fn get_dialect(&self) -> SqlDialect {
        self.dialect
    }

    /// Get whether line numbers are shown.
    #[must_use]
    pub fn shows_line_numbers(&self) -> bool {
        self.line_numbers
    }

    /// Get the change regions.
    #[must_use]
    pub fn change_regions(&self) -> &[ChangeRegion] {
        &self.change_regions
    }

    /// Render as plain text (no ANSI codes).
    ///
    /// Returns the SQL with optional line numbers, suitable for
    /// agent consumption or non-TTY output.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sqlmodel_console::renderables::DdlDisplay;
    ///
    /// let display = DdlDisplay::new("SELECT 1;").line_numbers(true);
    /// let plain = display.render_plain();
    /// assert!(plain.contains("1 |"));
    /// ```
    #[must_use]
    pub fn render_plain(&self) -> String {
        let lines: Vec<&str> = self.sql.lines().collect();

        if self.line_numbers {
            let max_line_num = lines.len();
            let width = max_line_num.to_string().len();

            lines
                .iter()
                .enumerate()
                .map(|(i, line)| format!("{:>width$} | {}", i + 1, line, width = width))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            self.sql.clone()
        }
    }

    /// Render with syntax highlighting (ANSI codes).
    ///
    /// Returns the DDL with syntax highlighting, optional line numbers,
    /// and change region highlighting.
    #[must_use]
    pub fn render(&self, _width: usize) -> String {
        let highlighter = SqlHighlighter::with_theme(self.theme.clone());
        let lines: Vec<&str> = self.sql.lines().collect();
        let max_line_num = lines.len();
        let line_width = max_line_num.to_string().len();

        let reset = "\x1b[0m";
        let dim = "\x1b[2m";

        let mut result = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1;
            let mut line_output = String::new();

            // Add line number if enabled
            if self.line_numbers {
                let line_num_str = format!("{:>width$}", line_num, width = line_width);
                line_output.push_str(&format!("{dim}{line_num_str} │{reset} "));
            }

            // Check for change region background
            let change_bg = self.get_change_background(line_num);

            // Apply change background if present
            if let Some(bg) = &change_bg {
                line_output.push_str(bg);
            }

            // Highlight the line with dialect-aware highlighting
            let styled_line = self.highlight_line(line, &highlighter);
            line_output.push_str(&styled_line);

            // Reset at end of line if we had a background
            if change_bg.is_some() {
                line_output.push_str(reset);
            }

            result.push(line_output);
        }

        result.join("\n")
    }

    /// Highlight a single line with dialect awareness.
    fn highlight_line(&self, line: &str, highlighter: &SqlHighlighter) -> String {
        let segments = highlighter.tokenize(line);
        let reset = "\x1b[0m";

        segments
            .iter()
            .map(|seg| self.colorize_segment(seg))
            .collect::<String>()
            + reset
    }

    /// Colorize a segment with dialect-specific keyword detection.
    fn colorize_segment(&self, seg: &SqlSegment) -> String {
        let reset = "\x1b[0m";

        // Check if this is a dialect-specific keyword
        if seg.token == SqlToken::Identifier {
            let upper = seg.text.to_uppercase();
            let dialect_keywords = self.dialect.ddl_keywords();
            if dialect_keywords.contains(&upper.as_str()) {
                // Highlight as keyword
                let color = self.theme.sql_keyword.color_code();
                return format!("{color}{}{reset}", seg.text);
            }
        }

        // Use standard coloring
        let color = match seg.token {
            SqlToken::Keyword => self.theme.sql_keyword.color_code(),
            SqlToken::String => self.theme.sql_string.color_code(),
            SqlToken::Number => self.theme.sql_number.color_code(),
            SqlToken::Comment => self.theme.sql_comment.color_code(),
            SqlToken::Operator => self.theme.sql_operator.color_code(),
            SqlToken::Identifier => self.theme.sql_identifier.color_code(),
            SqlToken::Parameter => self.theme.info.color_code(),
            SqlToken::Punctuation | SqlToken::Whitespace => String::new(),
        };

        if color.is_empty() {
            seg.text.clone()
        } else {
            format!("{color}{}{reset}", seg.text)
        }
    }

    /// Get the background color for a line if it's in a change region.
    fn get_change_background(&self, line: usize) -> Option<String> {
        for region in &self.change_regions {
            if region.contains_line(line) {
                return Some(match region.kind {
                    ChangeKind::Added => "\x1b[48;2;0;80;0m".to_string(), // Dark green bg
                    ChangeKind::Removed => "\x1b[48;2;80;0;0m".to_string(), // Dark red bg
                    ChangeKind::Modified => "\x1b[48;2;80;80;0m".to_string(), // Dark yellow bg
                });
            }
        }
        None
    }
}

impl Default for DdlDisplay {
    fn default() -> Self {
        Self::new("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ddl_display_creation() {
        let ddl = DdlDisplay::new("CREATE TABLE users (id INT);");
        assert_eq!(ddl.sql(), "CREATE TABLE users (id INT);");
        assert_eq!(ddl.get_dialect(), SqlDialect::PostgreSQL);
        assert!(!ddl.shows_line_numbers());
    }

    #[test]
    fn test_ddl_display_postgres_dialect() {
        let ddl = DdlDisplay::new("CREATE TABLE users (id SERIAL PRIMARY KEY);")
            .dialect(SqlDialect::PostgreSQL);
        assert_eq!(ddl.get_dialect(), SqlDialect::PostgreSQL);

        // SERIAL should be recognized as a dialect keyword
        let keywords = ddl.get_dialect().ddl_keywords();
        assert!(keywords.contains(&"SERIAL"));
        assert!(keywords.contains(&"BIGSERIAL"));
    }

    #[test]
    fn test_ddl_display_sqlite_dialect() {
        let ddl = DdlDisplay::new("CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT);")
            .dialect(SqlDialect::SQLite);
        assert_eq!(ddl.get_dialect(), SqlDialect::SQLite);

        // AUTOINCREMENT should be recognized
        let keywords = ddl.get_dialect().ddl_keywords();
        assert!(keywords.contains(&"AUTOINCREMENT"));
    }

    #[test]
    fn test_ddl_display_mysql_dialect() {
        let ddl = DdlDisplay::new(
            "CREATE TABLE users (id INT AUTO_INCREMENT PRIMARY KEY) ENGINE=InnoDB;",
        )
        .dialect(SqlDialect::MySQL);
        assert_eq!(ddl.get_dialect(), SqlDialect::MySQL);

        // AUTO_INCREMENT and ENGINE should be recognized
        let keywords = ddl.get_dialect().ddl_keywords();
        assert!(keywords.contains(&"AUTO_INCREMENT"));
        assert!(keywords.contains(&"ENGINE"));
    }

    #[test]
    fn test_ddl_display_line_numbers() {
        let ddl = DdlDisplay::new("CREATE TABLE users (\n    id INT\n);").line_numbers(true);
        assert!(ddl.shows_line_numbers());
    }

    #[test]
    fn test_ddl_display_render_plain() {
        let ddl = DdlDisplay::new("SELECT 1;\nSELECT 2;");
        let plain = ddl.render_plain();
        assert_eq!(plain, "SELECT 1;\nSELECT 2;");
    }

    #[test]
    fn test_ddl_display_render_plain_with_line_numbers() {
        let ddl = DdlDisplay::new("SELECT 1;\nSELECT 2;").line_numbers(true);
        let plain = ddl.render_plain();
        assert!(plain.contains("1 | SELECT 1;"));
        assert!(plain.contains("2 | SELECT 2;"));
    }

    #[test]
    fn test_ddl_display_render_rich() {
        let ddl = DdlDisplay::new("SELECT 1;");
        let rich = ddl.render(80);
        // Should contain ANSI escape codes
        assert!(rich.contains('\x1b'));
        // Should contain the SQL
        assert!(rich.contains("SELECT"));
        assert!(rich.contains('1'));
    }

    #[test]
    fn test_ddl_display_multi_statement() {
        let ddl =
            DdlDisplay::new("CREATE TABLE users (id INT);\nCREATE INDEX idx_users ON users(id);");
        let plain = ddl.render_plain();
        assert!(plain.contains("CREATE TABLE"));
        assert!(plain.contains("CREATE INDEX"));
    }

    #[test]
    fn test_ddl_display_with_comments() {
        let ddl = DdlDisplay::new("-- This is a comment\nSELECT 1;");
        let plain = ddl.render_plain();
        assert!(plain.contains("-- This is a comment"));
        assert!(plain.contains("SELECT 1;"));
    }

    #[test]
    fn test_ddl_display_change_highlighting() {
        let ddl = DdlDisplay::new("Line 1\nLine 2\nLine 3")
            .highlight_changes(vec![ChangeRegion::new(2, 2, ChangeKind::Added)]);
        assert_eq!(ddl.change_regions().len(), 1);
        assert!(ddl.change_regions()[0].contains_line(2));
        assert!(!ddl.change_regions()[0].contains_line(1));
    }

    #[test]
    fn test_change_region_contains_line() {
        let region = ChangeRegion::new(5, 10, ChangeKind::Modified);
        assert!(!region.contains_line(4));
        assert!(region.contains_line(5));
        assert!(region.contains_line(7));
        assert!(region.contains_line(10));
        assert!(!region.contains_line(11));
    }

    #[test]
    fn test_highlight_create_table() {
        let ddl = DdlDisplay::new("CREATE TABLE users (id INT);");
        let rich = ddl.render(80);
        // CREATE and TABLE should be highlighted as keywords
        assert!(rich.contains("CREATE"));
        assert!(rich.contains("TABLE"));
    }

    #[test]
    fn test_highlight_alter_table() {
        let ddl = DdlDisplay::new("ALTER TABLE users ADD COLUMN name TEXT;");
        let rich = ddl.render(80);
        assert!(rich.contains("ALTER"));
        assert!(rich.contains("TABLE"));
        assert!(rich.contains("ADD"));
    }

    #[test]
    fn test_highlight_drop_table() {
        let ddl = DdlDisplay::new("DROP TABLE IF EXISTS users;");
        let rich = ddl.render(80);
        assert!(rich.contains("DROP"));
        assert!(rich.contains("TABLE"));
        assert!(rich.contains("IF"));
        assert!(rich.contains("EXISTS"));
    }

    #[test]
    fn test_highlight_create_index() {
        let ddl = DdlDisplay::new("CREATE INDEX idx_name ON users (name);");
        let rich = ddl.render(80);
        assert!(rich.contains("CREATE"));
        assert!(rich.contains("INDEX"));
        assert!(rich.contains("ON"));
    }

    #[test]
    fn test_highlight_keywords() {
        let ddl = DdlDisplay::new("CREATE TABLE t (id INT PRIMARY KEY NOT NULL);");
        let rich = ddl.render(80);
        // All SQL keywords should be present and highlighted
        assert!(rich.contains("CREATE"));
        assert!(rich.contains("TABLE"));
        assert!(rich.contains("PRIMARY"));
        assert!(rich.contains("KEY"));
        assert!(rich.contains("NOT"));
        assert!(rich.contains("NULL"));
    }

    #[test]
    fn test_highlight_identifiers() {
        let ddl = DdlDisplay::new("CREATE TABLE my_table (my_column INT);");
        let rich = ddl.render(80);
        // Identifiers should be present
        assert!(rich.contains("my_table"));
        assert!(rich.contains("my_column"));
    }

    #[test]
    fn test_highlight_types() {
        let ddl = DdlDisplay::new("CREATE TABLE t (a INT, b TEXT, c BOOLEAN);");
        let rich = ddl.render(80);
        // Types should be highlighted as keywords
        assert!(rich.contains("INT"));
        assert!(rich.contains("TEXT"));
        assert!(rich.contains("BOOLEAN"));
    }

    #[test]
    fn test_highlight_constraints() {
        let ddl = DdlDisplay::new(
            "CREATE TABLE t (id INT PRIMARY KEY, fk INT REFERENCES other(id), u TEXT UNIQUE);",
        );
        let rich = ddl.render(80);
        assert!(rich.contains("PRIMARY"));
        assert!(rich.contains("KEY"));
        assert!(rich.contains("REFERENCES"));
        assert!(rich.contains("UNIQUE"));
    }

    #[test]
    fn test_plain_mode_no_color() {
        let ddl = DdlDisplay::new("CREATE TABLE t (id INT);");
        let plain = ddl.render_plain();
        // Plain output should not contain ANSI escape codes
        assert!(!plain.contains('\x1b'));
    }

    #[test]
    fn test_multiline_ddl() {
        let sql = "CREATE TABLE users (\n    id SERIAL PRIMARY KEY,\n    name TEXT NOT NULL\n);";
        let ddl = DdlDisplay::new(sql).line_numbers(true);
        let plain = ddl.render_plain();

        // Should have 4 lines with proper numbering
        let lines: Vec<&str> = plain.lines().collect();
        assert_eq!(lines.len(), 4);
        assert!(lines[0].contains("1 | CREATE TABLE"));
        assert!(lines[3].contains("4 | );"));
    }

    #[test]
    fn test_dialect_as_str() {
        assert_eq!(SqlDialect::PostgreSQL.as_str(), "PostgreSQL");
        assert_eq!(SqlDialect::SQLite.as_str(), "SQLite");
        assert_eq!(SqlDialect::MySQL.as_str(), "MySQL");
    }

    #[test]
    fn test_dialect_display() {
        assert_eq!(format!("{}", SqlDialect::PostgreSQL), "PostgreSQL");
        assert_eq!(format!("{}", SqlDialect::SQLite), "SQLite");
        assert_eq!(format!("{}", SqlDialect::MySQL), "MySQL");
    }

    #[test]
    fn test_default_dialect() {
        let ddl = DdlDisplay::new("SELECT 1");
        assert_eq!(ddl.get_dialect(), SqlDialect::PostgreSQL);
    }

    #[test]
    fn test_change_kind_variants() {
        let added = ChangeRegion::new(1, 1, ChangeKind::Added);
        let removed = ChangeRegion::new(2, 2, ChangeKind::Removed);
        let modified = ChangeRegion::new(3, 3, ChangeKind::Modified);

        assert_eq!(added.kind, ChangeKind::Added);
        assert_eq!(removed.kind, ChangeKind::Removed);
        assert_eq!(modified.kind, ChangeKind::Modified);
    }

    #[test]
    fn test_theme_customization() {
        let ddl = DdlDisplay::new("SELECT 1;").theme(Theme::light());
        // Just verify it compiles and renders without panic
        let _ = ddl.render(80);
    }

    #[test]
    fn test_add_change_builder() {
        let ddl = DdlDisplay::new("Line 1\nLine 2")
            .add_change(ChangeRegion::new(1, 1, ChangeKind::Added))
            .add_change(ChangeRegion::new(2, 2, ChangeKind::Removed));

        assert_eq!(ddl.change_regions().len(), 2);
    }

    #[test]
    fn test_empty_sql() {
        let ddl = DdlDisplay::new("");
        assert_eq!(ddl.sql(), "");
        assert_eq!(ddl.render_plain(), "");
    }

    #[test]
    fn test_default_impl() {
        let ddl = DdlDisplay::default();
        assert_eq!(ddl.sql(), "");
    }
}
