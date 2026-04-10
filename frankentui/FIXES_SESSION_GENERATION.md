# Session Fixes - Generation-Tracked Grapheme Pooling

## Critical Fix: Grapheme Pool ABA Protection
**Component:** `crates/ftui-render/src/cell.rs`, `crates/ftui-render/src/grapheme_pool.rs`
**Issue:** `GraphemeId` used a 24-bit slot index with no generation counter. If a slot was freed and reused (ABA problem), dangling `GraphemeId`s held by user code (outside the `gc` root set) would silently point to incorrect graphemes (e.g., an old "🚀" ID pointing to a new "💩" emoji).
**Fix:**
- Refactored `GraphemeId` layout from `[width: 7][slot: 24]` to `[width: 7][gen: 8][slot: 16]`.
- Reduced max slots from 16M to 64K (sufficient for TUI frames) to gain 8 bits for a generation counter.
- Updated `GraphemePool` to maintain a parallel `generations: Vec<u8>` array.
- Updated `intern` to increment generation on slot reuse.
- Updated `get`, `retain`, `release`, `refcount` to strictly validate `id.generation() == slot.generation`. Stale access now returns `None` or is ignored safely.
- Updated unit tests and property tests to reflect the new ID structure and verify generation isolation.

This change ensures that stale references are detected rather than causing visual corruption, significantly improving the robustness of the grapheme pooling system.
