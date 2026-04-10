# Dependency Upgrade Log

**Date:** 2026-02-19  |  **Project:** sqlmodel_rust  |  **Language:** Rust

## Summary
- **Updated:** 3  |  **Skipped:** 1  |  **Failed:** 0  |  **Needs attention:** 0

## Updates

### syn: 2.0.114 → 2.0.116
- **Breaking:** None (patch release)
- **Change:** `cargo update -p syn` (version spec `"2"` already allows it)
- **Tests:** Pass (161 tests in sqlmodel-macros)

### md5: 0.7.0 → 0.8.0
- **Breaking:** `Context::compute()` deprecated in favor of `Context::finalize()`. Added `no_std` support.
- **Migration:** None needed — project only uses `md5::compute()` free function (unchanged API)
- **Change:** `md5 = "0.7"` → `md5 = "0.8"` in sqlmodel-postgres/Cargo.toml
- **Tests:** Pass (59 unit + 7 integration tests in sqlmodel-postgres)

### webpki-roots: 0.26.11 → 1.0.6
- **Breaking:** None (API identical; 0.26.11 already re-exported 1.0 via semver trick)
- **Migration:** None needed — `TLS_SERVER_ROOTS` constant unchanged
- **Change:** `webpki-roots = "0.26"` → `webpki-roots = "1"` in sqlmodel-postgres and sqlmodel-mysql
- **Tests:** Pass (all postgres + mysql tests)

## Skipped

### rand: 0.8.5 → 0.10.0
- **Reason:** `rsa` 0.9.x (stable) depends on `rand_core` 0.6, which is incompatible with `rand` 0.10 (`rand_core` 0.10). The only compatible `rsa` version is 0.10.0-rc.15 (pre-release). Per policy, we do not upgrade to pre-release versions.
- **Action:** Revisit when `rsa` 0.10.0 stable is released.
- **Affected crates:** sqlmodel-postgres (auth/scram.rs), sqlmodel-mysql (auth.rs)

## Pre-Existing Issues

### sqlmodel-schema test failure (unrelated)
- `create::tests::test_create_table_sql_type_override` fails on both old and new dependency versions
- Not caused by any dependency update
