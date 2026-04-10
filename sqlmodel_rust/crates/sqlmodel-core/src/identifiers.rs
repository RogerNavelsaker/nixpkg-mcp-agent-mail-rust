//! SQL identifier quoting and sanitization utilities.
//!
//! This module provides functions for safely quoting SQL identifiers
//! (table names, column names, etc.) to prevent SQL injection and
//! handle special characters.

/// Quote a SQL identifier using ANSI double-quoting.
///
/// Embedded double-quotes are escaped by doubling them (`"` → `""`).
/// This function is safe against SQL injection for any input string.
///
/// # Examples
///
/// ```
/// use sqlmodel_core::quote_ident;
///
/// assert_eq!(quote_ident("users"), "\"users\"");
/// assert_eq!(quote_ident("user\"name"), "\"user\"\"name\"");
/// assert_eq!(quote_ident("select"), "\"select\""); // SQL keyword
/// ```
#[inline]
pub fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Quote a SQL identifier using MySQL backtick quoting.
///
/// Embedded backticks are escaped by doubling them (`` ` `` → ``` `` ```).
/// This function is safe against SQL injection for any input string.
///
/// # Examples
///
/// ```
/// use sqlmodel_core::quote_ident_mysql;
///
/// assert_eq!(quote_ident_mysql("users"), "`users`");
/// assert_eq!(quote_ident_mysql("user`name"), "`user``name`");
/// ```
#[inline]
pub fn quote_ident_mysql(name: &str) -> String {
    format!("`{}`", name.replace('`', "``"))
}

/// Sanitize a SQL identifier by removing non-alphanumeric/underscore characters.
///
/// Use this when quoting is not possible (e.g., PRAGMA commands, SHOW commands).
/// This is a more restrictive approach that only allows safe characters.
///
/// **Note:** This function strips characters rather than erroring. If the input
/// contains only invalid characters, the result will be an empty string.
///
/// # Examples
///
/// ```
/// use sqlmodel_core::sanitize_identifier;
///
/// assert_eq!(sanitize_identifier("users"), "users");
/// assert_eq!(sanitize_identifier("user_name"), "user_name");
/// assert_eq!(sanitize_identifier("user\"name"), "username");
/// assert_eq!(sanitize_identifier("user;DROP TABLE--"), "userDROPTABLE");
/// ```
#[inline]
pub fn sanitize_identifier(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ANSI Double-Quote Tests ====================

    #[test]
    fn test_quote_ident_simple() {
        assert_eq!(quote_ident("users"), "\"users\"");
    }

    #[test]
    fn test_quote_ident_empty() {
        // Empty string is valid SQL (though unusual)
        assert_eq!(quote_ident(""), "\"\"");
    }

    #[test]
    fn test_quote_ident_embedded_double_quote() {
        assert_eq!(quote_ident("user\"name"), "\"user\"\"name\"");
    }

    #[test]
    fn test_quote_ident_multiple_quotes() {
        assert_eq!(quote_ident("a\"b\"c"), "\"a\"\"b\"\"c\"");
    }

    #[test]
    fn test_quote_ident_sql_keyword() {
        // Quoting makes SQL keywords safe to use as identifiers
        assert_eq!(quote_ident("select"), "\"select\"");
        assert_eq!(quote_ident("from"), "\"from\"");
        assert_eq!(quote_ident("where"), "\"where\"");
    }

    #[test]
    fn test_quote_ident_spaces() {
        assert_eq!(quote_ident("first name"), "\"first name\"");
    }

    #[test]
    fn test_quote_ident_unicode() {
        assert_eq!(quote_ident("用户"), "\"用户\"");
        assert_eq!(quote_ident("naïve"), "\"naïve\"");
    }

    #[test]
    fn test_quote_ident_semicolon() {
        // Semicolons inside quoted identifiers are safe
        assert_eq!(quote_ident("a;b"), "\"a;b\"");
    }

    #[test]
    fn test_quote_ident_newline() {
        // Newlines inside quoted identifiers are safe
        assert_eq!(quote_ident("a\nb"), "\"a\nb\"");
    }

    #[test]
    fn test_quote_ident_null_byte() {
        // Null bytes are preserved (database will handle or reject)
        assert_eq!(quote_ident("a\0b"), "\"a\0b\"");
    }

    #[test]
    fn test_quote_ident_backslash() {
        // Backslash is literal in ANSI SQL
        assert_eq!(quote_ident("a\\b"), "\"a\\b\"");
    }

    #[test]
    fn test_quote_ident_only_quotes() {
        // Edge case: identifier consisting only of quotes
        assert_eq!(quote_ident("\"\""), "\"\"\"\"\"\"");
    }

    #[test]
    fn test_quote_ident_sql_injection_attempt() {
        // SQL injection attempt is safely quoted
        let malicious = "users\"; DROP TABLE secrets; --";
        let quoted = quote_ident(malicious);
        assert_eq!(quoted, "\"users\"\"; DROP TABLE secrets; --\"");
        // The result is a valid identifier, not executable SQL
    }

    // ==================== MySQL Backtick Tests ====================

    #[test]
    fn test_quote_ident_mysql_simple() {
        assert_eq!(quote_ident_mysql("users"), "`users`");
    }

    #[test]
    fn test_quote_ident_mysql_embedded_backtick() {
        assert_eq!(quote_ident_mysql("user`name"), "`user``name`");
    }

    #[test]
    fn test_quote_ident_mysql_keyword() {
        assert_eq!(quote_ident_mysql("select"), "`select`");
    }

    #[test]
    fn test_quote_ident_mysql_multiple_backticks() {
        assert_eq!(quote_ident_mysql("a`b`c"), "`a``b``c`");
    }

    #[test]
    fn test_quote_ident_mysql_empty() {
        assert_eq!(quote_ident_mysql(""), "``");
    }

    // ==================== Sanitize Identifier Tests ====================

    #[test]
    fn test_sanitize_simple() {
        assert_eq!(sanitize_identifier("users"), "users");
    }

    #[test]
    fn test_sanitize_strips_quotes() {
        assert_eq!(sanitize_identifier("user\"name"), "username");
    }

    #[test]
    fn test_sanitize_strips_semicolons() {
        assert_eq!(sanitize_identifier("a;b"), "ab");
    }

    #[test]
    fn test_sanitize_preserves_underscore() {
        assert_eq!(sanitize_identifier("user_name"), "user_name");
    }

    #[test]
    fn test_sanitize_strips_spaces() {
        assert_eq!(sanitize_identifier("user name"), "username");
    }

    #[test]
    fn test_sanitize_empty_input() {
        assert_eq!(sanitize_identifier(""), "");
    }

    #[test]
    fn test_sanitize_only_invalid_chars() {
        assert_eq!(sanitize_identifier("!@#$%"), "");
    }

    #[test]
    fn test_sanitize_sql_injection_attempt() {
        assert_eq!(
            sanitize_identifier("users; DROP TABLE secrets; --"),
            "usersDROPTABLEsecrets"
        );
    }

    #[test]
    fn test_sanitize_unicode_stripped() {
        // Unicode is stripped (only ASCII alphanumeric + underscore allowed)
        assert_eq!(sanitize_identifier("用户"), "");
        assert_eq!(sanitize_identifier("naïve"), "nave");
    }

    #[test]
    fn test_sanitize_numbers_preserved() {
        assert_eq!(sanitize_identifier("table123"), "table123");
        assert_eq!(sanitize_identifier("123table"), "123table");
    }
}
