mod fixtures;

use fixtures::mock_types::{MockConnection, MockPoolStats};
use fixtures::*;
use sqlmodel_console::ConsoleAware;
use sqlmodel_console::renderables::PoolStatsProvider;
use sqlmodel_console::renderables::{QueryResultTable, SchemaTree};

fn table_info_to_table_data(
    table: sqlmodel_schema::introspect::TableInfo,
) -> sqlmodel_console::renderables::TableData {
    use sqlmodel_console::renderables::{ColumnData, ForeignKeyData, IndexData, TableData};
    TableData {
        name: table.name,
        columns: table
            .columns
            .into_iter()
            .map(|c| ColumnData {
                name: c.name,
                sql_type: c.sql_type,
                nullable: c.nullable,
                default: c.default,
                primary_key: c.primary_key,
                auto_increment: c.auto_increment,
            })
            .collect(),
        primary_key: table.primary_key,
        foreign_keys: table
            .foreign_keys
            .into_iter()
            .map(|fk| ForeignKeyData {
                name: fk.name,
                column: fk.column,
                foreign_table: fk.foreign_table,
                foreign_column: fk.foreign_column,
                on_delete: fk.on_delete,
                on_update: fk.on_update,
            })
            .collect(),
        indexes: table
            .indexes
            .into_iter()
            .map(|idx| IndexData {
                name: idx.name,
                columns: idx.columns,
                unique: idx.unique,
            })
            .collect(),
    }
}

#[test]
fn test_user_table_info() {
    let table = user_table_info();
    assert_eq!(table.name, "users");
    assert_eq!(table.columns.len(), 4);
    assert_eq!(table.primary_key, vec!["id".to_string()]);
}

#[test]
fn test_posts_table_info() {
    let table = posts_table_info();
    assert_eq!(table.name, "posts");
    assert_eq!(table.foreign_keys.len(), 1);
    assert_eq!(table.indexes.len(), 1);
}

#[test]
fn test_sample_query_results_small() {
    let (cols, rows) = sample_query_results_small();
    assert_eq!(cols.len(), 3);
    assert_eq!(rows.len(), 3);
}

#[test]
fn test_sample_errors_render_plain() {
    let plain = sample_syntax_error().render_plain();
    assert_golden("error_panel_plain.txt", &plain);
}

#[test]
fn test_sample_errors_render_rich() {
    let rich = sample_syntax_error().render_styled();
    assert_golden("error_panel_rich.txt", &rich);
}

#[test]
fn test_query_results_table_small_render_plain() {
    let (cols, rows) = sample_query_results_small();
    let table = QueryResultTable::from_data(cols, rows);
    let plain = table.render_plain();
    assert_golden("query_table_small.txt", &plain);
}

#[test]
fn test_query_results_table_large_render_plain() {
    let (cols, rows) = sample_query_results_large(20, 5);
    let table = QueryResultTable::from_data(cols, rows)
        .timing_ms(12.34)
        .max_rows(5);
    let plain = table.render_plain();
    assert_golden("query_table_large.txt", &plain);
}

#[test]
fn test_schema_tree_render_plain() {
    let users = table_info_to_table_data(user_table_info());
    let posts = table_info_to_table_data(posts_table_info());
    let tree = SchemaTree::new(&[users, posts]).ascii();
    let plain = tree.render_plain();
    assert_golden("schema_tree.txt", &plain);
}

#[test]
fn test_mock_connection_records_calls() {
    let conn = MockConnection::new();
    conn.emit_status("connecting");
    conn.emit_error("failed");
    assert_eq!(conn.status_calls.borrow().len(), 1);
    assert_eq!(conn.error_calls.borrow().len(), 1);
}

#[test]
fn test_mock_pool_stats_provider() {
    let stats = MockPoolStats::busy();
    assert_eq!(stats.active_connections(), 8);
    assert_eq!(stats.idle_connections(), 2);
    assert_eq!(stats.pending_requests(), 0);
    assert_eq!(stats.max_connections(), 10);
}

#[test]
fn test_golden_loader() {
    let content = load_golden("query_table_small.txt");
    assert!(content.contains("Alice"));
}
