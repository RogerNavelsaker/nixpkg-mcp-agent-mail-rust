//! Compile-fail tests for terminal mode typestate transitions (bd-3rrzt.3).
//!
//! Uses trybuild to verify that invalid mode transitions produce
//! compile-time errors, enforcing the typestate safety guarantees.

#[test]
fn compile_fail_invalid_transitions() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
