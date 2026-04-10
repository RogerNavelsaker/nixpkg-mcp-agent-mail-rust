//! Sample data for testing console components.

#![allow(dead_code)] // These fixtures may not all be used yet

use sqlmodel_console::renderables::{ErrorPanel, ErrorSeverity};
use sqlmodel_schema::introspect::{
    ColumnInfo, ForeignKeyInfo, IndexInfo, ParsedSqlType, TableInfo,
};

/// Helper to create a column with parsed type.
fn col(
    name: &str,
    sql_type: &str,
    nullable: bool,
    default: Option<&str>,
    pk: bool,
    auto: bool,
) -> ColumnInfo {
    ColumnInfo {
        name: name.to_string(),
        sql_type: sql_type.to_string(),
        parsed_type: ParsedSqlType::parse(sql_type),
        nullable,
        default: default.map(String::from),
        primary_key: pk,
        auto_increment: auto,
        comment: None,
    }
}

/// Sample user table schema.
pub fn user_table_info() -> TableInfo {
    TableInfo {
        name: "users".to_string(),
        columns: vec![
            col("id", "INTEGER", false, None, true, true),
            col("name", "TEXT", false, None, false, false),
            col("email", "TEXT", false, None, false, false),
            col(
                "created_at",
                "TIMESTAMP",
                false,
                Some("NOW()"),
                false,
                false,
            ),
        ],
        primary_key: vec!["id".to_string()],
        foreign_keys: Vec::new(),
        unique_constraints: Vec::new(),
        check_constraints: Vec::new(),
        indexes: vec![IndexInfo {
            name: "idx_users_email".to_string(),
            columns: vec!["email".to_string()],
            unique: true,
            index_type: None,
            primary: false,
        }],
        comment: None,
    }
}

/// Sample posts table schema with foreign key.
pub fn posts_table_info() -> TableInfo {
    TableInfo {
        name: "posts".to_string(),
        columns: vec![
            col("id", "INTEGER", false, None, true, true),
            col("user_id", "INTEGER", false, None, false, false),
            col("title", "TEXT", false, None, false, false),
            col("content", "TEXT", true, None, false, false),
        ],
        primary_key: vec!["id".to_string()],
        foreign_keys: vec![ForeignKeyInfo {
            name: Some("fk_posts_user".to_string()),
            column: "user_id".to_string(),
            foreign_table: "users".to_string(),
            foreign_column: "id".to_string(),
            on_delete: Some("CASCADE".to_string()),
            on_update: None,
        }],
        unique_constraints: Vec::new(),
        check_constraints: Vec::new(),
        indexes: vec![IndexInfo {
            name: "idx_posts_user".to_string(),
            columns: vec!["user_id".to_string()],
            unique: false,
            index_type: None,
            primary: false,
        }],
        comment: None,
    }
}

/// Sample query results - small dataset.
pub fn sample_query_results_small() -> (Vec<String>, Vec<Vec<String>>) {
    let columns = vec!["id".to_string(), "name".to_string(), "email".to_string()];
    let rows = vec![
        vec![
            "1".to_string(),
            "Alice".to_string(),
            "alice@example.com".to_string(),
        ],
        vec![
            "2".to_string(),
            "Bob".to_string(),
            "bob@example.com".to_string(),
        ],
        vec![
            "3".to_string(),
            "Carol".to_string(),
            "carol@example.com".to_string(),
        ],
    ];
    (columns, rows)
}

/// Sample query results - large dataset.
pub fn sample_query_results_large(rows: usize, cols: usize) -> (Vec<String>, Vec<Vec<String>>) {
    let columns: Vec<String> = (0..cols).map(|i| format!("col_{i}")).collect();
    let rows: Vec<Vec<String>> = (0..rows)
        .map(|r| (0..cols).map(|c| format!("r{r}c{c}")).collect())
        .collect();
    (columns, rows)
}

/// Sample SQL syntax error.
pub fn sample_syntax_error() -> ErrorPanel {
    ErrorPanel::new("SQL Syntax Error", "Unexpected token near 'FORM'")
        .with_sql("SELECT * FORM users WHERE id = 1")
        .with_position(10)
        .with_sqlstate("42601")
        .with_hint("Did you mean 'FROM'?")
}

/// Sample connection error.
pub fn sample_connection_error() -> ErrorPanel {
    ErrorPanel::new("Connection Failed", "Could not connect to database")
        .severity(ErrorSeverity::Critical)
        .with_detail("Connection refused (os error 111)")
        .add_context("Host: localhost:5432")
        .add_context("User: postgres")
        .with_hint("Check that the database server is running")
}

/// Sample timeout error.
pub fn sample_timeout_error() -> ErrorPanel {
    ErrorPanel::new("Query Timeout", "Query exceeded maximum execution time")
        .severity(ErrorSeverity::Warning)
        .with_sql("SELECT * FROM large_table WHERE complex_condition")
        .with_detail("Timeout after 30 seconds")
        .with_hint("Consider adding an index or simplifying the query")
}
