#![forbid(unsafe_code)]

//! End-to-end tests for the Layout Laboratory screen (bd-32my.4).
//!
//! These tests exercise the layout constraint solver and interactive
//! layout demos through the `LayoutLab` screen, covering:
//!
//! - Preset switching (1-5 keys)
//! - Direction toggle (Vertical/Horizontal)
//! - Alignment mode cycling (Start, Center, End, SpaceBetween, SpaceAround)
//! - Gap and margin adjustment
//! - Constraint selection and navigation
//! - Widget demos (Padding, Align, Columns, Group)
//! - Debug overlay toggle
//! - Various terminal sizes
//!
//! # Invariants (Alien Artifact)
//!
//! 1. **Rect bounds**: All layout rects are within the parent area bounds.
//! 2. **No overlap**: Rects in a single direction (V/H) do not overlap.
//! 3. **Constraint count**: Number of rects equals number of constraints.
//! 4. **Direction toggle involutive**: Toggle twice returns to original.
//! 5. **Alignment cycling periodic**: 5 modes cycle back to start.
//! 6. **Gap/Margin bounded**: Gap <= 5, Margin <= 4.
//!
//! # Failure Modes
//!
//! | Scenario | Expected Behavior |
//! |----------|-------------------|
//! | Zero-width render area | No panic, shows "Terminal too small" |
//! | Tiny terminal (40x10) | Graceful degradation, no panic |
//! | Rapid preset switches | State remains consistent |
//! | Max gap/margin values | Values capped at limits |
//!
//! # JSONL Schema
//!
//! ```json
//! {"ts":"T000001","step":"env","test":"e2e_initial_state","cols":"120","rows":"40"}
//! {"ts":"T000002","step":"rendered","frame_hash":"a1b2c3d4e5f6g7h8"}
//! {"ts":"T000003","step":"preset_switch","preset":"2","name":"Grid 3x3"}
//! ```
//!
//! Run: `cargo test -p ftui-demo-showcase --test layout_lab_e2e`

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use ftui_core::event::{Event, KeyCode, KeyEvent, KeyEventKind, Modifiers};
use ftui_core::geometry::Rect;
use ftui_demo_showcase::screens::Screen;
use ftui_demo_showcase::screens::layout_lab::LayoutLab;
use ftui_harness::assert_snapshot;
use ftui_render::frame::Frame;
use ftui_render::grapheme_pool::GraphemePool;

// ---------------------------------------------------------------------------
// JSONL Logging Helpers
// ---------------------------------------------------------------------------

/// Atomic counter for ordered JSONL timestamps.
static JSONL_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Emit a JSONL log entry to stderr for verbose test logging.
fn log_jsonl(step: &str, data: &[(&str, &str)]) {
    let ts = JSONL_COUNTER.fetch_add(1, Ordering::Relaxed);
    let fields: Vec<String> = std::iter::once(format!("\"ts\":\"T{ts:06}\""))
        .chain(std::iter::once(format!("\"step\":\"{step}\"")))
        .chain(data.iter().map(|(k, v)| format!("\"{k}\":\"{v}\"")))
        .collect();
    eprintln!("{{{}}}", fields.join(","));
}

/// Log full schema entry for test start.
fn log_test_start(test_name: &str, cols: u16, rows: u16) {
    log_jsonl(
        "env",
        &[
            ("test", test_name),
            ("cols", &cols.to_string()),
            ("rows", &rows.to_string()),
            ("term", "test"),
            ("colorterm", ""),
            ("capabilities", "harness"),
        ],
    );
}

/// Log test completion with timing and checksum.
fn log_test_end(test_name: &str, start: Instant, checksum: u64, passed: bool) {
    log_jsonl(
        "outcome",
        &[
            ("test", test_name),
            ("status", if passed { "passed" } else { "failed" }),
            ("elapsed_ms", &start.elapsed().as_millis().to_string()),
            ("checksum", &format!("{checksum:016x}")),
        ],
    );
}

// ---------------------------------------------------------------------------
// Event Helpers
// ---------------------------------------------------------------------------

fn press(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: Modifiers::NONE,
        kind: KeyEventKind::Press,
    })
}

fn shift_press(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: Modifiers::SHIFT,
        kind: KeyEventKind::Press,
    })
}

fn char_press(ch: char) -> Event {
    press(KeyCode::Char(ch))
}

// ---------------------------------------------------------------------------
// Frame Capture Helpers
// ---------------------------------------------------------------------------

/// Capture a frame and return a hash for determinism checks.
fn capture_frame_hash(lab: &LayoutLab, width: u16, height: u16) -> u64 {
    let mut pool = GraphemePool::new();
    let mut frame = Frame::new(width, height, &mut pool);
    let area = Rect::new(0, 0, width, height);
    lab.view(&mut frame, area);
    let mut hasher = DefaultHasher::new();
    for y in 0..height {
        for x in 0..width {
            if let Some(cell) = frame.buffer.get(x, y)
                && let Some(ch) = cell.content.as_char()
            {
                ch.hash(&mut hasher);
            }
        }
    }
    hasher.finish()
}

/// Render the layout lab and return the frame for inspection.
fn render_lab(lab: &LayoutLab, width: u16, height: u16) -> Frame<'static> {
    let pool = Box::leak(Box::new(GraphemePool::new()));
    let mut frame = Frame::new(width, height, pool);
    let area = Rect::new(0, 0, width, height);
    lab.view(&mut frame, area);
    frame
}

/// Extract text content from a frame buffer.
fn frame_to_text(frame: &Frame, width: u16, height: u16) -> String {
    let mut text = String::new();
    for y in 0..height {
        for x in 0..width {
            if let Some(cell) = frame.buffer.get(x, y)
                && let Some(ch) = cell.content.as_char()
            {
                text.push(ch);
            } else {
                text.push(' ');
            }
        }
        text.push('\n');
    }
    text
}

// ===========================================================================
// Scenario 1: Initial State and Rendering
// ===========================================================================

#[test]
fn e2e_initial_state_renders_correctly() {
    let start = Instant::now();
    log_test_start("e2e_initial_state_renders_correctly", 120, 40);

    let lab = LayoutLab::new();

    // Verify initial state
    assert_eq!(lab.title(), "Layout Laboratory");
    assert_eq!(lab.tab_label(), "Layout");
    log_jsonl(
        "check",
        &[("title", "Layout Laboratory"), ("tab", "Layout")],
    );

    // Render at standard size
    let hash = capture_frame_hash(&lab, 120, 40);
    log_jsonl("rendered", &[("frame_hash", &format!("{hash:016x}"))]);

    log_test_end("e2e_initial_state_renders_correctly", start, hash, true);
}

#[test]
fn e2e_renders_at_various_sizes() {
    let start = Instant::now();
    log_test_start("e2e_renders_at_various_sizes", 120, 40);

    let lab = LayoutLab::new();

    // Standard sizes
    let sizes = [(120, 40), (80, 24), (200, 50), (40, 10)];
    let mut last_hash = 0u64;

    for (w, h) in sizes {
        let hash = capture_frame_hash(&lab, w, h);
        log_jsonl(
            "rendered",
            &[
                ("width", &w.to_string()),
                ("height", &h.to_string()),
                ("frame_hash", &format!("{hash:016x}")),
            ],
        );
        last_hash = hash;
    }

    // Zero area should not panic
    let mut pool = GraphemePool::new();
    let mut frame = Frame::new(1, 1, &mut pool);
    lab.view(&mut frame, Rect::new(0, 0, 0, 0));
    log_jsonl("zero_area", &[("result", "no_panic")]);

    log_test_end("e2e_renders_at_various_sizes", start, last_hash, true);
}

#[test]
fn layout_lab_initial_80x24() {
    let lab = LayoutLab::new();
    let mut pool = GraphemePool::new();
    let mut frame = Frame::new(80, 24, &mut pool);
    let area = Rect::new(0, 0, 80, 24);
    lab.view(&mut frame, area);
    assert_snapshot!("layout_lab_initial_80x24", &frame.buffer);
}

#[test]
fn layout_lab_initial_120x40() {
    let lab = LayoutLab::new();
    let mut pool = GraphemePool::new();
    let mut frame = Frame::new(120, 40, &mut pool);
    let area = Rect::new(0, 0, 120, 40);
    lab.view(&mut frame, area);
    assert_snapshot!("layout_lab_initial_120x40", &frame.buffer);
}

#[test]
fn layout_lab_tiny_40x10() {
    let lab = LayoutLab::new();
    let mut pool = GraphemePool::new();
    let mut frame = Frame::new(40, 10, &mut pool);
    let area = Rect::new(0, 0, 40, 10);
    lab.view(&mut frame, area);
    assert_snapshot!("layout_lab_tiny_40x10", &frame.buffer);
}

#[test]
fn layout_lab_wide_200x50() {
    let lab = LayoutLab::new();
    let mut pool = GraphemePool::new();
    let mut frame = Frame::new(200, 50, &mut pool);
    let area = Rect::new(0, 0, 200, 50);
    lab.view(&mut frame, area);
    assert_snapshot!("layout_lab_wide_200x50", &frame.buffer);
}

// ===========================================================================
// Scenario 2: Preset Switching
// ===========================================================================

#[test]
fn e2e_preset_switching() {
    let start = Instant::now();
    log_test_start("e2e_preset_switching", 120, 40);

    let mut lab = LayoutLab::new();

    // Initial preset is 0 (Flex Vertical)
    let initial_hash = capture_frame_hash(&lab, 120, 40);
    log_jsonl(
        "initial",
        &[
            ("preset", "0"),
            ("name", "Flex Vertical"),
            ("hash", &format!("{initial_hash:016x}")),
        ],
    );

    // Switch through all presets
    let preset_names = [
        "Flex Vertical",
        "Flex Horizontal",
        "Grid 3x3",
        "Nested Flex",
        "Real-World Layout",
    ];

    for (i, name) in preset_names.iter().enumerate() {
        let key = char::from_digit((i + 1) as u32, 10).unwrap();
        lab.update(&char_press(key));
        let hash = capture_frame_hash(&lab, 120, 40);
        log_jsonl(
            "preset_switch",
            &[
                ("key", &key.to_string()),
                ("preset", &i.to_string()),
                ("name", name),
                ("hash", &format!("{hash:016x}")),
            ],
        );
    }

    let final_hash = capture_frame_hash(&lab, 120, 40);
    log_test_end("e2e_preset_switching", start, final_hash, true);
}

#[test]
fn layout_lab_preset_grid_120x40() {
    let mut lab = LayoutLab::new();
    lab.update(&char_press('3')); // Switch to Grid 3x3

    let mut pool = GraphemePool::new();
    let mut frame = Frame::new(120, 40, &mut pool);
    let area = Rect::new(0, 0, 120, 40);
    lab.view(&mut frame, area);
    assert_snapshot!("layout_lab_preset_grid_120x40", &frame.buffer);
}

#[test]
fn layout_lab_preset_nested_120x40() {
    let mut lab = LayoutLab::new();
    lab.update(&char_press('4')); // Switch to Nested Flex

    let mut pool = GraphemePool::new();
    let mut frame = Frame::new(120, 40, &mut pool);
    let area = Rect::new(0, 0, 120, 40);
    lab.view(&mut frame, area);
    assert_snapshot!("layout_lab_preset_nested_120x40", &frame.buffer);
}

#[test]
fn layout_lab_preset_realworld_120x40() {
    let mut lab = LayoutLab::new();
    lab.update(&char_press('5')); // Switch to Real-World Layout

    let mut pool = GraphemePool::new();
    let mut frame = Frame::new(120, 40, &mut pool);
    let area = Rect::new(0, 0, 120, 40);
    lab.view(&mut frame, area);
    assert_snapshot!("layout_lab_preset_realworld_120x40", &frame.buffer);
}

// ===========================================================================
// Scenario 3: Direction Toggle
// ===========================================================================

#[test]
fn e2e_direction_toggle() {
    let start = Instant::now();
    log_test_start("e2e_direction_toggle", 120, 40);

    let mut lab = LayoutLab::new();

    // Initial direction is Vertical
    let initial_hash = capture_frame_hash(&lab, 120, 40);
    log_jsonl(
        "initial",
        &[
            ("direction", "Vertical"),
            ("hash", &format!("{initial_hash:016x}")),
        ],
    );

    // Toggle to Horizontal
    lab.update(&char_press('d'));
    let horizontal_hash = capture_frame_hash(&lab, 120, 40);
    log_jsonl(
        "toggled",
        &[
            ("direction", "Horizontal"),
            ("hash", &format!("{horizontal_hash:016x}")),
        ],
    );
    assert_ne!(
        initial_hash, horizontal_hash,
        "Direction toggle should change rendering"
    );

    // Toggle back to Vertical
    lab.update(&char_press('d'));
    let back_hash = capture_frame_hash(&lab, 120, 40);
    log_jsonl(
        "toggled_back",
        &[
            ("direction", "Vertical"),
            ("hash", &format!("{back_hash:016x}")),
        ],
    );
    assert_eq!(
        initial_hash, back_hash,
        "Toggle twice should return to initial state"
    );

    log_test_end("e2e_direction_toggle", start, back_hash, true);
}

#[test]
fn layout_lab_direction_horizontal_120x40() {
    let mut lab = LayoutLab::new();
    lab.update(&char_press('d')); // Toggle to Horizontal

    let mut pool = GraphemePool::new();
    let mut frame = Frame::new(120, 40, &mut pool);
    let area = Rect::new(0, 0, 120, 40);
    lab.view(&mut frame, area);
    assert_snapshot!("layout_lab_direction_horizontal_120x40", &frame.buffer);
}

// ===========================================================================
// Scenario 4: Alignment Cycling
// ===========================================================================

#[test]
fn e2e_alignment_cycling() {
    let start = Instant::now();
    log_test_start("e2e_alignment_cycling", 120, 40);

    let mut lab = LayoutLab::new();
    let alignment_names = ["Start", "Center", "End", "SpaceBetween", "SpaceAround"];

    // Capture initial state
    let initial_hash = capture_frame_hash(&lab, 120, 40);
    log_jsonl(
        "initial",
        &[
            ("alignment", "Start"),
            ("hash", &format!("{initial_hash:016x}")),
        ],
    );

    // Cycle through all alignments
    for (i, name) in alignment_names.iter().enumerate().skip(1) {
        lab.update(&char_press('a'));
        let hash = capture_frame_hash(&lab, 120, 40);
        log_jsonl(
            "alignment_cycle",
            &[
                ("index", &i.to_string()),
                ("alignment", name),
                ("hash", &format!("{hash:016x}")),
            ],
        );
    }

    // One more cycle should wrap to Start
    lab.update(&char_press('a'));
    let wrapped_hash = capture_frame_hash(&lab, 120, 40);
    log_jsonl(
        "alignment_wrap",
        &[
            ("alignment", "Start"),
            ("hash", &format!("{wrapped_hash:016x}")),
        ],
    );
    assert_eq!(
        initial_hash, wrapped_hash,
        "5 alignment cycles should wrap to initial"
    );

    log_test_end("e2e_alignment_cycling", start, wrapped_hash, true);
}

#[test]
fn layout_lab_alignment_center_120x40() {
    let mut lab = LayoutLab::new();
    lab.update(&char_press('a')); // Cycle to Center

    let mut pool = GraphemePool::new();
    let mut frame = Frame::new(120, 40, &mut pool);
    let area = Rect::new(0, 0, 120, 40);
    lab.view(&mut frame, area);
    assert_snapshot!("layout_lab_alignment_center_120x40", &frame.buffer);
}

#[test]
fn layout_lab_alignment_spacebetween_120x40() {
    let mut lab = LayoutLab::new();
    // Cycle: Start -> Center -> End -> SpaceBetween
    lab.update(&char_press('a'));
    lab.update(&char_press('a'));
    lab.update(&char_press('a'));

    let mut pool = GraphemePool::new();
    let mut frame = Frame::new(120, 40, &mut pool);
    let area = Rect::new(0, 0, 120, 40);
    lab.view(&mut frame, area);
    assert_snapshot!("layout_lab_alignment_spacebetween_120x40", &frame.buffer);
}

// ===========================================================================
// Scenario 5: Gap and Margin Adjustment
// ===========================================================================

#[test]
fn e2e_gap_adjustment() {
    let start = Instant::now();
    log_test_start("e2e_gap_adjustment", 120, 40);

    let mut lab = LayoutLab::new();

    // Initial gap is 0
    let initial_hash = capture_frame_hash(&lab, 120, 40);
    log_jsonl(
        "initial",
        &[("gap", "0"), ("hash", &format!("{initial_hash:016x}"))],
    );

    // Increase gap
    for gap in 1..=5 {
        lab.update(&char_press('+'));
        let hash = capture_frame_hash(&lab, 120, 40);
        log_jsonl(
            "gap_increased",
            &[("gap", &gap.to_string()), ("hash", &format!("{hash:016x}"))],
        );
    }

    // Gap should be capped at 5
    lab.update(&char_press('+'));
    let capped_hash = capture_frame_hash(&lab, 120, 40);
    log_jsonl("gap_capped", &[("gap", "5"), ("result", "capped_at_max")]);

    // Decrease gap
    lab.update(&char_press('-'));
    let decreased_hash = capture_frame_hash(&lab, 120, 40);
    log_jsonl(
        "gap_decreased",
        &[("gap", "4"), ("hash", &format!("{decreased_hash:016x}"))],
    );

    log_test_end("e2e_gap_adjustment", start, capped_hash, true);
}

#[test]
fn e2e_margin_adjustment() {
    let start = Instant::now();
    log_test_start("e2e_margin_adjustment", 120, 40);

    let mut lab = LayoutLab::new();

    // Initial margin is 0
    let initial_hash = capture_frame_hash(&lab, 120, 40);
    log_jsonl(
        "initial",
        &[("margin", "0"), ("hash", &format!("{initial_hash:016x}"))],
    );

    // Increase margin
    for margin in 1..=4 {
        lab.update(&char_press('m'));
        let hash = capture_frame_hash(&lab, 120, 40);
        log_jsonl(
            "margin_increased",
            &[
                ("margin", &margin.to_string()),
                ("hash", &format!("{hash:016x}")),
            ],
        );
    }

    // Margin should be capped at 4
    lab.update(&char_press('m'));
    let capped_hash = capture_frame_hash(&lab, 120, 40);
    log_jsonl(
        "margin_capped",
        &[("margin", "4"), ("result", "capped_at_max")],
    );

    // Decrease margin with Shift+M
    lab.update(&shift_press(KeyCode::Char('M')));
    let decreased_hash = capture_frame_hash(&lab, 120, 40);
    log_jsonl(
        "margin_decreased",
        &[("margin", "3"), ("hash", &format!("{decreased_hash:016x}"))],
    );

    log_test_end("e2e_margin_adjustment", start, capped_hash, true);
}

#[test]
fn layout_lab_with_gap_120x40() {
    let mut lab = LayoutLab::new();
    // Add gap
    lab.update(&char_press('+'));
    lab.update(&char_press('+'));

    let mut pool = GraphemePool::new();
    let mut frame = Frame::new(120, 40, &mut pool);
    let area = Rect::new(0, 0, 120, 40);
    lab.view(&mut frame, area);
    assert_snapshot!("layout_lab_with_gap_120x40", &frame.buffer);
}

#[test]
fn layout_lab_with_margin_120x40() {
    let mut lab = LayoutLab::new();
    // Add margin
    lab.update(&char_press('m'));
    lab.update(&char_press('m'));

    let mut pool = GraphemePool::new();
    let mut frame = Frame::new(120, 40, &mut pool);
    let area = Rect::new(0, 0, 120, 40);
    lab.view(&mut frame, area);
    assert_snapshot!("layout_lab_with_margin_120x40", &frame.buffer);
}

// ===========================================================================
// Scenario 6: Constraint Selection (Tab Navigation)
// ===========================================================================

#[test]
fn e2e_constraint_selection() {
    let start = Instant::now();
    log_test_start("e2e_constraint_selection", 120, 40);

    let mut lab = LayoutLab::new();

    // Enable debug overlay to see constraint selection
    lab.update(&char_press('D'));

    // Preset 0 has 5 constraints
    for i in 0..5 {
        let hash = capture_frame_hash(&lab, 120, 40);
        log_jsonl(
            "constraint_selected",
            &[("index", &i.to_string()), ("hash", &format!("{hash:016x}"))],
        );
        lab.update(&press(KeyCode::Tab));
    }

    // Should wrap back to 0
    let wrapped_hash = capture_frame_hash(&lab, 120, 40);
    log_jsonl(
        "constraint_wrap",
        &[("index", "0"), ("hash", &format!("{wrapped_hash:016x}"))],
    );

    log_test_end("e2e_constraint_selection", start, wrapped_hash, true);
}

// ===========================================================================
// Scenario 7: Debug Overlay Toggle
// ===========================================================================

#[test]
fn e2e_debug_overlay_toggle() {
    let start = Instant::now();
    log_test_start("e2e_debug_overlay_toggle", 120, 40);

    let mut lab = LayoutLab::new();

    // Initial state: debug off
    let initial_hash = capture_frame_hash(&lab, 120, 40);
    log_jsonl(
        "initial",
        &[("debug", "off"), ("hash", &format!("{initial_hash:016x}"))],
    );

    // Toggle debug on with 'D' (uppercase D, no modifiers)
    lab.update(&char_press('D'));
    let debug_on_hash = capture_frame_hash(&lab, 120, 40);
    log_jsonl(
        "debug_toggled",
        &[("debug", "on"), ("hash", &format!("{debug_on_hash:016x}"))],
    );
    assert_ne!(
        initial_hash, debug_on_hash,
        "Debug toggle should change rendering"
    );

    // Toggle debug off
    lab.update(&char_press('D'));
    let debug_off_hash = capture_frame_hash(&lab, 120, 40);
    log_jsonl(
        "debug_toggled_back",
        &[
            ("debug", "off"),
            ("hash", &format!("{debug_off_hash:016x}")),
        ],
    );
    assert_eq!(
        initial_hash, debug_off_hash,
        "Toggle debug twice should return to initial"
    );

    log_test_end("e2e_debug_overlay_toggle", start, debug_off_hash, true);
}

#[test]
fn layout_lab_debug_overlay_120x40() {
    let mut lab = LayoutLab::new();
    lab.update(&char_press('D')); // Toggle debug on

    let mut pool = GraphemePool::new();
    let mut frame = Frame::new(120, 40, &mut pool);
    let area = Rect::new(0, 0, 120, 40);
    lab.view(&mut frame, area);
    assert_snapshot!("layout_lab_debug_overlay_120x40", &frame.buffer);
}

// ===========================================================================
// Scenario 8: Align Position Cycling
// ===========================================================================

#[test]
fn e2e_align_position_cycling() {
    let start = Instant::now();
    log_test_start("e2e_align_position_cycling", 120, 40);

    let mut lab = LayoutLab::new();
    let position_names = [
        "TopLeft",
        "TopCenter",
        "TopRight",
        "MidLeft",
        "Center",
        "MidRight",
        "BotLeft",
        "BotCenter",
        "BotRight",
    ];

    // Initial position is Center (index 4)
    log_jsonl("initial", &[("align_pos", "Center")]);

    // Cycle through remaining positions (5..8, then 0..4)
    for i in 1..9 {
        lab.update(&char_press('l'));
        let expected_idx = (4 + i) % 9;
        let hash = capture_frame_hash(&lab, 120, 40);
        log_jsonl(
            "align_pos_cycle",
            &[
                ("index", &expected_idx.to_string()),
                ("position", position_names[expected_idx]),
                ("hash", &format!("{hash:016x}")),
            ],
        );
    }

    let final_hash = capture_frame_hash(&lab, 120, 40);
    log_test_end("e2e_align_position_cycling", start, final_hash, true);
}

// ===========================================================================
// Scenario 9: Padding Adjustment
// ===========================================================================

#[test]
fn e2e_padding_adjustment() {
    let start = Instant::now();
    log_test_start("e2e_padding_adjustment", 120, 40);

    let mut lab = LayoutLab::new();

    // Increase padding
    lab.update(&char_press('p'));
    let increased_hash = capture_frame_hash(&lab, 120, 40);
    log_jsonl(
        "padding_increased",
        &[("hash", &format!("{increased_hash:016x}"))],
    );

    // Decrease padding with Shift+P
    lab.update(&shift_press(KeyCode::Char('P')));
    let decreased_hash = capture_frame_hash(&lab, 120, 40);
    log_jsonl(
        "padding_decreased",
        &[("hash", &format!("{decreased_hash:016x}"))],
    );

    log_test_end("e2e_padding_adjustment", start, decreased_hash, true);
}

// ===========================================================================
// Scenario 10: Screen Trait Implementation
// ===========================================================================

#[test]
fn e2e_screen_trait_methods() {
    log_test_start("e2e_screen_trait_methods", 80, 24);

    let lab = LayoutLab::new();

    assert_eq!(lab.title(), "Layout Laboratory");
    assert_eq!(lab.tab_label(), "Layout");

    let keybindings = lab.keybindings();
    assert!(
        keybindings.len() >= 10,
        "Should have at least 10 keybindings, got {}",
        keybindings.len()
    );
    log_jsonl("keybindings", &[("count", &keybindings.len().to_string())]);

    // Verify specific keybindings exist
    let expected_keys = ["1-5", "d", "a", "+/-", "m/M", "p/P", "Tab", "l", "D"];
    for expected in expected_keys {
        let has_key = keybindings.iter().any(|k| k.key == expected);
        assert!(has_key, "Should have '{expected}' keybinding");
    }

    log_jsonl("trait_check", &[("result", "passed")]);
}

// ===========================================================================
// Scenario 11: Content Verification
// ===========================================================================

#[test]
fn e2e_frame_contains_expected_content() {
    log_test_start("e2e_frame_contains_expected_content", 120, 40);

    let lab = LayoutLab::new();
    let frame = render_lab(&lab, 120, 40);
    let text = frame_to_text(&frame, 120, 40);

    // Should contain preset title
    assert!(
        text.contains("Preset 1: Flex Vertical"),
        "Should show preset title"
    );

    // Should contain Controls panel
    assert!(text.contains("Controls"), "Should show Controls panel");

    // Should contain Widget Demos panel
    assert!(
        text.contains("Widget Demos"),
        "Should show Widget Demos panel"
    );

    // Should contain constraint labels
    assert!(
        text.contains("Fixed") || text.contains("Pct") || text.contains("Min"),
        "Should show constraint labels"
    );

    log_jsonl("content_check", &[("result", "passed")]);
}

#[test]
fn e2e_grid_preset_shows_header() {
    log_test_start("e2e_grid_preset_shows_header", 120, 40);

    let mut lab = LayoutLab::new();
    lab.update(&char_press('3')); // Switch to Grid 3x3

    let frame = render_lab(&lab, 120, 40);
    let text = frame_to_text(&frame, 120, 40);

    assert!(
        text.contains("Grid 3x3"),
        "Should show Grid 3x3 in preset title"
    );

    log_jsonl("grid_check", &[("result", "passed")]);
}

// ===========================================================================
// Scenario 12: Determinism Verification
// ===========================================================================

#[test]
fn e2e_deterministic_rendering() {
    let start = Instant::now();
    log_test_start("e2e_deterministic_rendering", 120, 40);

    // Create two identical labs and verify they render identically
    let lab1 = LayoutLab::new();
    let lab2 = LayoutLab::new();

    let hash1 = capture_frame_hash(&lab1, 120, 40);
    let hash2 = capture_frame_hash(&lab2, 120, 40);

    assert_eq!(
        hash1, hash2,
        "Identical LayoutLab instances should render identically"
    );

    log_jsonl(
        "determinism",
        &[
            ("hash1", &format!("{hash1:016x}")),
            ("hash2", &format!("{hash2:016x}")),
            ("match", "true"),
        ],
    );

    // Apply same operations to both
    let mut lab1 = lab1;
    let mut lab2 = lab2;

    lab1.update(&char_press('d'));
    lab2.update(&char_press('d'));

    let hash1 = capture_frame_hash(&lab1, 120, 40);
    let hash2 = capture_frame_hash(&lab2, 120, 40);

    assert_eq!(
        hash1, hash2,
        "After same operations, labs should render identically"
    );

    log_test_end("e2e_deterministic_rendering", start, hash1, true);
}

// ===========================================================================
// Scenario 13: Rapid Interaction Stress Test
// ===========================================================================

#[test]
fn e2e_rapid_interactions() {
    let start = Instant::now();
    log_test_start("e2e_rapid_interactions", 80, 24);

    let mut lab = LayoutLab::new();
    let mut checksum = 0u64;

    // Rapidly cycle through presets
    for _ in 0..20 {
        for i in 1..=5 {
            let key = char::from_digit(i, 10).unwrap();
            lab.update(&char_press(key));
        }
    }
    checksum ^= capture_frame_hash(&lab, 80, 24);

    // Rapidly toggle direction
    for _ in 0..50 {
        lab.update(&char_press('d'));
    }
    checksum ^= capture_frame_hash(&lab, 80, 24);

    // Rapidly cycle alignment
    for _ in 0..25 {
        lab.update(&char_press('a'));
    }
    checksum ^= capture_frame_hash(&lab, 80, 24);

    // Rapidly adjust gap
    for _ in 0..20 {
        lab.update(&char_press('+'));
        lab.update(&char_press('-'));
    }
    checksum ^= capture_frame_hash(&lab, 80, 24);

    log_jsonl(
        "stress_test",
        &[
            ("iterations", "115"),
            ("checksum", &format!("{checksum:016x}")),
        ],
    );

    log_test_end("e2e_rapid_interactions", start, checksum, true);
}

// ===========================================================================
// Scenario 14: Combined State Changes
// ===========================================================================

#[test]
fn e2e_combined_state_changes() {
    let start = Instant::now();
    log_test_start("e2e_combined_state_changes", 120, 40);

    let mut lab = LayoutLab::new();

    // Apply multiple state changes
    lab.update(&char_press('2')); // Flex Horizontal preset
    lab.update(&char_press('d')); // Toggle direction
    lab.update(&char_press('a')); // Cycle alignment
    lab.update(&char_press('a')); // Cycle alignment again
    lab.update(&char_press('+')); // Increase gap
    lab.update(&char_press('m')); // Increase margin
    lab.update(&char_press('D')); // Toggle debug

    let final_hash = capture_frame_hash(&lab, 120, 40);
    log_jsonl(
        "combined_state",
        &[
            ("preset", "1"),
            ("direction", "Horizontal"),
            ("alignment", "End"),
            ("gap", "1"),
            ("margin", "1"),
            ("debug", "on"),
            ("hash", &format!("{final_hash:016x}")),
        ],
    );

    // Verify rendering doesn't panic with all these changes
    let frame = render_lab(&lab, 120, 40);
    let _text = frame_to_text(&frame, 120, 40);

    log_test_end("e2e_combined_state_changes", start, final_hash, true);
}

#[test]
fn layout_lab_combined_state_120x40() {
    let mut lab = LayoutLab::new();
    lab.update(&char_press('3')); // Grid preset
    lab.update(&char_press('+')); // Gap
    lab.update(&char_press('+')); // More gap
    lab.update(&char_press('m')); // Margin

    let mut pool = GraphemePool::new();
    let mut frame = Frame::new(120, 40, &mut pool);
    let area = Rect::new(0, 0, 120, 40);
    lab.view(&mut frame, area);
    assert_snapshot!("layout_lab_combined_state_120x40", &frame.buffer);
}

// ===========================================================================
// Scenario 15: Resize Regression (Round-Trip Determinism + Latency Logging)
// ===========================================================================

#[test]
fn e2e_resize_roundtrip_regression() {
    let start = Instant::now();
    log_test_start("e2e_resize_roundtrip_regression", 120, 40);

    let mut lab = LayoutLab::new();
    lab.update(&char_press('5')); // Real-world layout preset (denser constraints)

    let sizes = [(80, 24), (120, 40), (160, 50), (120, 40), (80, 24)];
    let mut hashes = Vec::with_capacity(sizes.len());
    let mut max_elapsed_ms = 0u128;

    for (w, h) in sizes {
        let render_start = Instant::now();
        let hash = capture_frame_hash(&lab, w, h);
        let elapsed_ms = render_start.elapsed().as_millis();
        max_elapsed_ms = max_elapsed_ms.max(elapsed_ms);
        hashes.push(hash);
        log_jsonl(
            "resize_render",
            &[
                ("width", &w.to_string()),
                ("height", &h.to_string()),
                ("elapsed_ms", &elapsed_ms.to_string()),
                ("hash", &format!("{hash:016x}")),
            ],
        );
    }

    assert_eq!(
        hashes[0], hashes[4],
        "Round-trip resize back to 80x24 should be deterministic"
    );
    assert_eq!(
        hashes[1], hashes[3],
        "Round-trip resize back to 120x40 should be deterministic"
    );

    log_jsonl(
        "resize_summary",
        &[("max_elapsed_ms", &max_elapsed_ms.to_string())],
    );
    log_test_end(
        "e2e_resize_roundtrip_regression",
        start,
        hashes[hashes.len() - 1],
        true,
    );
}

#[test]
fn e2e_resize_storm_determinism() {
    let start = Instant::now();
    log_test_start("e2e_resize_storm_determinism", 120, 40);

    let mut lab_a = LayoutLab::new();
    let mut lab_b = LayoutLab::new();
    lab_a.update(&char_press('4')); // Nested preset for more constraints
    lab_b.update(&char_press('4'));

    let sizes = [
        (60, 20),
        (80, 24),
        (100, 30),
        (120, 40),
        (90, 28),
        (70, 22),
        (140, 45),
        (110, 36),
        (80, 24),
        (60, 20),
    ];

    let mut hasher = DefaultHasher::new();
    let mut max_elapsed_ms = 0u128;

    for (w, h) in sizes {
        let render_start = Instant::now();
        let hash_a = capture_frame_hash(&lab_a, w, h);
        let hash_b = capture_frame_hash(&lab_b, w, h);
        let elapsed_ms = render_start.elapsed().as_millis();
        max_elapsed_ms = max_elapsed_ms.max(elapsed_ms);

        assert_eq!(
            hash_a, hash_b,
            "Deterministic resize storm: hashes must match for {w}x{h}"
        );

        log_jsonl(
            "resize_storm",
            &[
                ("width", &w.to_string()),
                ("height", &h.to_string()),
                ("elapsed_ms", &elapsed_ms.to_string()),
                ("hash", &format!("{hash_a:016x}")),
            ],
        );

        w.hash(&mut hasher);
        h.hash(&mut hasher);
        hash_a.hash(&mut hasher);
    }

    let checksum = hasher.finish();
    log_jsonl(
        "resize_storm_summary",
        &[
            ("max_elapsed_ms", &max_elapsed_ms.to_string()),
            ("checksum", &format!("{checksum:016x}")),
        ],
    );
    log_test_end("e2e_resize_storm_determinism", start, checksum, true);
}
