//! Benchmark: read-heavy SELECT workload (WHERE/aggregates/ORDER BY/LIMIT).
//!
//! Bead: bd-72im
//!
//! Measures query performance across multiple SELECT patterns on both
//! FrankenSQLite and C SQLite.  Each benchmark group populates a table with
//! 1,000 rows, then repeatedly executes a specific query pattern.
//!
//! Patterns:
//! 1. Point lookup (`WHERE id = ?`)
//! 2. Range scan (`WHERE id BETWEEN ? AND ?`)
//! 3. Full-table aggregate (`SELECT COUNT(*)`)
//! 4. GROUP BY aggregate
//! 5. ORDER BY + LIMIT
//! 6. Correlated `EXISTS` subquery
//! 7. `IN (SELECT ...)` subquery
//! 8. Recursive CTE

use std::time::Duration;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use fsqlite_types::value::SqliteValue;

const SEED_ROWS: i64 = 1000;
const COUNT_SEED_ROWS: i64 = 10_000;
const SUBQUERY_ROWS: i64 = 10_000;
const RECURSIVE_CTE_LIMIT: i64 = 1_000;
const RECURSIVE_CTE_SUM: i64 = 500_500;

fn expected_score_sum(row_count: i64) -> i64 {
    7 * row_count * (row_count + 1) / 2
}

// ─── PRAGMA helpers ─────────────────────────────────────────────────────

fn apply_pragmas_csqlite(conn: &rusqlite::Connection) {
    conn.execute_batch(
        "PRAGMA page_size = 4096;\
         PRAGMA journal_mode = WAL;\
         PRAGMA synchronous = NORMAL;\
         PRAGMA cache_size = -64000;",
    )
    .ok();
}

fn apply_pragmas_fsqlite(conn: &fsqlite::Connection) {
    for pragma in [
        "PRAGMA page_size = 4096;",
        "PRAGMA journal_mode = WAL;",
        "PRAGMA synchronous = NORMAL;",
        "PRAGMA cache_size = -64000;",
    ] {
        let _ = conn.execute(pragma);
    }
}

// ─── Setup helpers ──────────────────────────────────────────────────────

fn setup_csqlite_with_rows(row_count: i64) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    apply_pragmas_csqlite(&conn);
    conn.execute_batch(
        "CREATE TABLE bench (\
             id INTEGER PRIMARY KEY,\
             name TEXT,\
             category TEXT,\
             score INTEGER\
         );",
    )
    .unwrap();
    conn.execute_batch("BEGIN").unwrap();
    {
        let mut stmt = conn
            .prepare(
                "INSERT INTO bench VALUES (?1, ('name_' || ?1), ('cat_' || (?1 % 10)), (?1 * 7))",
            )
            .unwrap();
        for i in 1..=row_count {
            stmt.execute(rusqlite::params![i]).unwrap();
        }
    }
    conn.execute_batch("COMMIT").unwrap();
    conn
}

fn setup_csqlite() -> rusqlite::Connection {
    setup_csqlite_with_rows(SEED_ROWS)
}

fn setup_fsqlite_with_rows(row_count: i64) -> fsqlite::Connection {
    let conn = fsqlite::Connection::open(":memory:").unwrap();
    apply_pragmas_fsqlite(&conn);
    conn.execute(
        "CREATE TABLE bench (\
             id INTEGER PRIMARY KEY,\
             name TEXT,\
             category TEXT,\
             score INTEGER\
         )",
    )
    .unwrap();
    conn.execute("BEGIN").unwrap();
    for i in 1..=row_count {
        conn.execute(&format!(
            "INSERT INTO bench VALUES ({i}, 'name_{i}', 'cat_{}', {})",
            i % 10,
            i * 7,
        ))
        .unwrap();
    }
    conn.execute("COMMIT").unwrap();
    conn
}

fn setup_fsqlite() -> fsqlite::Connection {
    setup_fsqlite_with_rows(SEED_ROWS)
}

fn setup_csqlite_subquery() -> rusqlite::Connection {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let category_count = (SUBQUERY_ROWS / 20).max(5);
    apply_pragmas_csqlite(&conn);
    conn.execute_batch(
        "CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT, price REAL, category_id INTEGER);\
         CREATE TABLE categories (id INTEGER PRIMARY KEY, name TEXT);",
    )
    .unwrap();
    conn.execute_batch("BEGIN").unwrap();
    {
        let mut category_stmt = conn
            .prepare("INSERT INTO categories VALUES (?1, ('cat_' || ?1))")
            .unwrap();
        for i in 1..=category_count {
            category_stmt.execute(rusqlite::params![i]).unwrap();
        }
        let mut product_stmt = conn
            .prepare(
                "INSERT INTO products VALUES (?1, ('prod_' || ?1), (?1 * 3.14), ((?1 % ?2) + 1))",
            )
            .unwrap();
        for i in 1..=SUBQUERY_ROWS {
            product_stmt
                .execute(rusqlite::params![i, category_count])
                .unwrap();
        }
    }
    conn.execute_batch("COMMIT").unwrap();
    conn.execute_batch("CREATE INDEX idx_prod_cat ON products(category_id);")
        .unwrap();
    conn
}

fn setup_fsqlite_subquery() -> fsqlite::Connection {
    let conn = fsqlite::Connection::open(":memory:").unwrap();
    let category_count = (SUBQUERY_ROWS / 20).max(5);
    apply_pragmas_fsqlite(&conn);
    conn.execute(
        "CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT, price REAL, category_id INTEGER)",
    )
    .unwrap();
    conn.execute("CREATE TABLE categories (id INTEGER PRIMARY KEY, name TEXT)")
        .unwrap();
    conn.execute("BEGIN").unwrap();
    for i in 1..=category_count {
        conn.execute(&format!("INSERT INTO categories VALUES ({i}, 'cat_{i}')"))
            .unwrap();
    }
    for i in 1..=SUBQUERY_ROWS {
        let category_id = (i % category_count) + 1;
        let price = i as f64 * 3.14;
        conn.execute(&format!(
            "INSERT INTO products VALUES ({i}, 'prod_{i}', {price}, {category_id})"
        ))
        .unwrap();
    }
    conn.execute("COMMIT").unwrap();
    conn.execute("CREATE INDEX idx_prod_cat ON products(category_id)")
        .unwrap();
    conn
}

// ─── Criterion config ───────────────────────────────────────────────────

fn criterion_config() -> Criterion {
    Criterion::default().configure_from_args()
}

// ─── 1. Point lookup ────────────────────────────────────────────────────

fn bench_point_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_point_lookup");
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(10));
    group.throughput(Throughput::Elements(1));

    group.bench_function("csqlite", |b| {
        let conn = setup_csqlite();
        let mut stmt = conn.prepare("SELECT * FROM bench WHERE id = ?1").unwrap();
        let mut id = 1_i64;
        b.iter(|| {
            let rows: Vec<(i64, String)> = stmt
                .query_map(rusqlite::params![id], |row| {
                    Ok((row.get(0).unwrap(), row.get(1).unwrap()))
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(rows.len(), 1);
            id = (id % SEED_ROWS) + 1;
        });
    });

    group.bench_function("frankensqlite", |b| {
        let conn = setup_fsqlite();
        let stmt = conn.prepare("SELECT * FROM bench WHERE id = ?1").unwrap();
        let mut id = 1_i64;
        b.iter(|| {
            let _row = stmt
                .query_row_with_params(&[SqliteValue::Integer(id)])
                .unwrap();
            id = (id % SEED_ROWS) + 1;
        });
    });

    group.finish();
}

// ─── 2. Range scan ──────────────────────────────────────────────────────

fn bench_range_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_range_scan_50");
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(10));
    group.throughput(Throughput::Elements(50));

    group.bench_function("csqlite", |b| {
        let conn = setup_csqlite();
        let mut stmt = conn
            .prepare("SELECT * FROM bench WHERE id >= ?1 AND id < ?2")
            .unwrap();
        b.iter(|| {
            let rows: Vec<(i64,)> = stmt
                .query_map(rusqlite::params![200, 250], |row| {
                    Ok((row.get(0).unwrap(),))
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(rows.len(), 50);
        });
    });

    group.bench_function("frankensqlite", |b| {
        let conn = setup_fsqlite();
        let stmt = conn
            .prepare("SELECT * FROM bench WHERE id >= ?1 AND id < ?2")
            .unwrap();
        b.iter(|| {
            let rows = stmt
                .query_with_params(&[SqliteValue::Integer(200), SqliteValue::Integer(250)])
                .unwrap();
            assert_eq!(rows.len(), 50);
        });
    });

    group.finish();
}

// ─── 3. Full-table aggregate ────────────────────────────────────────────

fn bench_full_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_count_star");
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(10));
    group.throughput(Throughput::Elements(1));

    group.bench_function("csqlite", |b| {
        let conn = setup_csqlite();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM bench").unwrap();
        b.iter(|| {
            let count: i64 = stmt.query_row([], |r| r.get(0)).unwrap();
            assert_eq!(count, SEED_ROWS);
        });
    });

    group.bench_function("frankensqlite", |b| {
        let conn = setup_fsqlite();
        let stmt = conn.prepare("SELECT COUNT(*) FROM bench").unwrap();
        b.iter(|| {
            let row = stmt.query_row().unwrap();
            assert_eq!(row.values()[0], SqliteValue::Integer(SEED_ROWS));
        });
    });

    group.finish();
}

fn bench_full_count_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_count_star_10000");
    group.sample_size(30);
    group.measurement_time(Duration::from_secs(15));
    group.throughput(Throughput::Elements(1));

    group.bench_function("csqlite", |b| {
        let conn = setup_csqlite_with_rows(COUNT_SEED_ROWS);
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM bench").unwrap();
        b.iter(|| {
            let count: i64 = stmt.query_row([], |r| r.get(0)).unwrap();
            assert_eq!(count, COUNT_SEED_ROWS);
        });
    });

    group.bench_function("frankensqlite", |b| {
        let conn = setup_fsqlite_with_rows(COUNT_SEED_ROWS);
        let stmt = conn.prepare("SELECT COUNT(*) FROM bench").unwrap();
        b.iter(|| {
            let row = stmt.query_row().unwrap();
            assert_eq!(row.values()[0], SqliteValue::Integer(COUNT_SEED_ROWS));
        });
    });

    group.finish();
}

fn bench_count_range(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_count_range_50");
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(10));
    group.throughput(Throughput::Elements(50));

    group.bench_function("csqlite", |b| {
        let conn = setup_csqlite();
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM bench WHERE id >= ?1 AND id < ?2")
            .unwrap();
        b.iter(|| {
            let count: i64 = stmt
                .query_row(rusqlite::params![200, 250], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 50);
        });
    });

    group.bench_function("frankensqlite", |b| {
        let conn = setup_fsqlite();
        let stmt = conn
            .prepare("SELECT COUNT(*) FROM bench WHERE id >= ?1 AND id < ?2")
            .unwrap();
        b.iter(|| {
            let row = stmt
                .query_row_with_params(&[SqliteValue::Integer(200), SqliteValue::Integer(250)])
                .unwrap();
            assert_eq!(row.values()[0], SqliteValue::Integer(50));
        });
    });

    group.finish();
}

fn bench_count_sum_aggregate(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_count_sum_aggregate");
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(10));
    group.throughput(Throughput::Elements(1));

    group.bench_function("csqlite", |b| {
        let conn = setup_csqlite();
        let mut stmt = conn
            .prepare("SELECT COUNT(*), SUM(score) FROM bench")
            .unwrap();
        b.iter(|| {
            let (count, sum): (i64, i64) = stmt
                .query_row([], |r| Ok((r.get(0).unwrap(), r.get(1).unwrap())))
                .unwrap();
            assert_eq!(count, SEED_ROWS);
            assert_eq!(sum, expected_score_sum(SEED_ROWS));
        });
    });

    group.bench_function("frankensqlite", |b| {
        let conn = setup_fsqlite();
        let stmt = conn
            .prepare("SELECT COUNT(*), SUM(score) FROM bench")
            .unwrap();
        b.iter(|| {
            let row = stmt.query_row().unwrap();
            assert_eq!(row.values()[0], SqliteValue::Integer(SEED_ROWS));
            assert_eq!(
                row.values()[1],
                SqliteValue::Integer(expected_score_sum(SEED_ROWS))
            );
        });
    });

    group.finish();
}

// ─── 4. GROUP BY aggregate ──────────────────────────────────────────────

fn bench_group_by(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_group_by");
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(10));
    group.throughput(Throughput::Elements(1));

    group.bench_function("csqlite", |b| {
        let conn = setup_csqlite();
        let mut stmt = conn
            .prepare("SELECT category, COUNT(*), SUM(score) FROM bench GROUP BY category")
            .unwrap();
        b.iter(|| {
            let rows: Vec<(String, i64, i64)> = stmt
                .query_map([], |row| {
                    Ok((
                        row.get(0).unwrap(),
                        row.get(1).unwrap(),
                        row.get(2).unwrap(),
                    ))
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(rows.len(), 10);
        });
    });

    group.bench_function("frankensqlite", |b| {
        let conn = setup_fsqlite();
        let stmt = conn
            .prepare("SELECT category, COUNT(*), SUM(score) FROM bench GROUP BY category")
            .unwrap();
        b.iter(|| {
            let rows = stmt.query().unwrap();
            assert_eq!(rows.len(), 10);
        });
    });

    group.finish();
}

// ─── 5. ORDER BY + LIMIT ───────────────────────────────────────────────

fn bench_order_limit(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_order_limit_10");
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(10));
    group.throughput(Throughput::Elements(10));

    group.bench_function("csqlite", |b| {
        let conn = setup_csqlite();
        let mut stmt = conn
            .prepare("SELECT * FROM bench ORDER BY score DESC LIMIT 10")
            .unwrap();
        b.iter(|| {
            let rows: Vec<(i64,)> = stmt
                .query_map([], |row| Ok((row.get(0).unwrap(),)))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(rows.len(), 10);
        });
    });

    group.bench_function("frankensqlite", |b| {
        let conn = setup_fsqlite();
        let stmt = conn
            .prepare("SELECT * FROM bench ORDER BY score DESC LIMIT 10")
            .unwrap();
        b.iter(|| {
            let rows = stmt.query().unwrap();
            assert_eq!(rows.len(), 10);
        });
    });

    group.finish();
}

fn bench_exists_subquery(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_exists_subquery_count");
    group.sample_size(30);
    group.measurement_time(Duration::from_secs(10));
    group.throughput(Throughput::Elements(1));

    let category_count = (SUBQUERY_ROWS / 20).max(5);
    let half = category_count / 2;
    let expected_count = SUBQUERY_ROWS / 2;
    let sql = format!(
        "SELECT COUNT(*) FROM products p WHERE EXISTS (SELECT 1 FROM categories c WHERE c.id = p.category_id AND c.id <= {half})"
    );

    group.bench_function("csqlite", |b| {
        let conn = setup_csqlite_subquery();
        let mut stmt = conn.prepare(&sql).unwrap();
        b.iter(|| {
            let count: i64 = stmt.query_row([], |r| r.get(0)).unwrap();
            assert_eq!(count, expected_count);
        });
    });

    group.bench_function("frankensqlite", |b| {
        let conn = setup_fsqlite_subquery();
        let stmt = conn.prepare(&sql).unwrap();
        b.iter(|| {
            let row = stmt.query_row().unwrap();
            assert_eq!(row.values()[0], SqliteValue::Integer(expected_count));
        });
    });

    group.finish();
}

fn bench_in_subquery(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_in_subquery_count");
    group.sample_size(30);
    group.measurement_time(Duration::from_secs(10));
    group.throughput(Throughput::Elements(1));

    let expected_count = 100_i64;
    let sql = "SELECT COUNT(*) FROM products WHERE category_id IN (SELECT id FROM categories WHERE id <= 5)";

    group.bench_function("csqlite", |b| {
        let conn = setup_csqlite_subquery();
        let mut stmt = conn.prepare(sql).unwrap();
        b.iter(|| {
            let count: i64 = stmt.query_row([], |r| r.get(0)).unwrap();
            assert_eq!(count, expected_count);
        });
    });

    group.bench_function("frankensqlite", |b| {
        let conn = setup_fsqlite_subquery();
        let stmt = conn.prepare(sql).unwrap();
        b.iter(|| {
            let row = stmt.query_row().unwrap();
            assert_eq!(row.values()[0], SqliteValue::Integer(expected_count));
        });
    });

    group.finish();
}

fn bench_recursive_cte(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_recursive_cte_sum_1000");
    group.sample_size(30);
    group.measurement_time(Duration::from_secs(10));
    group.throughput(Throughput::Elements(RECURSIVE_CTE_LIMIT as u64));

    let sql = "WITH RECURSIVE cnt(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM cnt WHERE x < 1000) SELECT SUM(x) FROM cnt";

    group.bench_function("csqlite", |b| {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let mut stmt = conn.prepare(sql).unwrap();
        b.iter(|| {
            let sum: i64 = stmt.query_row([], |r| r.get(0)).unwrap();
            assert_eq!(sum, RECURSIVE_CTE_SUM);
        });
    });

    group.bench_function("frankensqlite", |b| {
        let conn = fsqlite::Connection::open(":memory:").unwrap();
        let stmt = conn.prepare(sql).unwrap();
        b.iter(|| {
            let row = stmt.query_row().unwrap();
            assert_eq!(row.values()[0], SqliteValue::Integer(RECURSIVE_CTE_SUM));
        });
    });

    group.finish();
}

criterion_group!(
    name = read_heavy;
    config = criterion_config();
    targets =
        bench_point_lookup,
        bench_range_scan,
        bench_full_count,
        bench_full_count_large,
        bench_count_range,
        bench_count_sum_aggregate,
        bench_group_by,
        bench_order_limit,
        bench_exists_subquery,
        bench_in_subquery,
        bench_recursive_cte
);
criterion_main!(read_heavy);
