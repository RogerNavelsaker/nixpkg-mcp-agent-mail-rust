use ftui_text::wrap::{WrapMode, WrapOptions, wrap_text, wrap_text_optimal};

#[test]
fn test_wrap_optimal_indentation() {
    // "   foo" (3 spaces + 3 chars = 6 width)
    let text = "   foo";

    // Width 10: Should fit on one line
    let lines = wrap_text_optimal(text, 10);
    assert_eq!(lines, vec!["   foo"]);

    // Width 4: Should NOT split indentation if treated as space attached to phantom previous word
    // Current suspected behavior: splits into ["", "foo"] because "   " is treated as trailing space of line 1
    // Desired behavior: depends on definition. Usually we want indentation preserved attached to the word if possible,
    // or treated as content.
    let lines_narrow = wrap_text_optimal(text, 4);
    // If it treats "   " as width 0 content + 3 space:
    // Line 1: "   ", width 0. Fits.
    // Line 2: "foo", width 3. Fits.
    // Result: ["", "foo"] (trimmed).

    // If treated as content:
    // "   " (3) + "foo" (3) = 6. > 4.
    // Must break.
    // Line 1: "   " (3). Fits.
    // Line 2: "foo" (3). Fits.
    // Result: ["   ", "foo"].

    // Let's see what happens.
    println!("Width 4: {:?}", lines_narrow);
}

#[test]
fn test_wrap_word_indentation() {
    let text = "   foo";
    // Standard wrap
    let lines = wrap_text(text, 10, WrapMode::Word);
    assert_eq!(lines, vec!["foo"]); // Default preserve_indent is false in wrap_text.

    // Wait, `wrap_text` uses default options.
    // `WrapOptions::default()` has `preserve_indent: false`.
    // `finalize_line` with `preserve_indent: false` trims start.
    // So `wrap_text` should return ["foo"].

    // assert_eq!(wrap_text(text, 10, WrapMode::Word), vec!["foo"]);

    let opts = WrapOptions::new(10).preserve_indent(true);
    let lines_preserve = ftui_text::wrap::wrap_with_options(text, &opts);
    assert_eq!(lines_preserve, vec!["   foo"]);
}
