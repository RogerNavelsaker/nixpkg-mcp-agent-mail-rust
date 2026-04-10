//! JIT closure compilation for hot prepared DML programs (V3.1).
//!
//! Instead of runtime code generation, this "JIT" compiles recognized VDBE
//! program patterns into specialized Rust closures at prepare time. The
//! closures call B-tree operations directly, bypassing the VDBE interpreter
//! dispatch loop entirely.
//!
//! Currently supports: simple sequential INSERT with no indexes/triggers/FKs.

use fsqlite_types::opcode::{Opcode, VdbeOp};

/// A compiled DML closure that bypasses the VDBE interpreter.
pub enum CompiledDml {
    /// Compiled simple sequential INSERT.
    /// Params: column values to insert (bound parameters).
    /// Returns: (affected_rows, last_insert_rowid).
    SimpleInsert(SimpleInsertTemplate),
}

/// Template for a compiled simple INSERT program.
///
/// Captures the static properties of the INSERT at compile time:
/// cursor ID, root page, column count, parameter mapping.
#[derive(Debug)]
pub struct SimpleInsertTemplate {
    /// Cursor ID for the target table.
    pub cursor_id: i32,
    /// Root page number of the target table.
    pub root_page: i32,
    /// Number of columns in the record.
    pub num_cols: i32,
    /// Register index of the first column value.
    /// Parameters map 1:1 from the params slice to consecutive registers.
    pub first_col_reg: i32,
    /// Insert flags (P5 from the Insert opcode).
    pub insert_flags: u16,
}

/// Attempt to compile a VDBE program into a specialized closure.
///
/// Returns `None` if the program doesn't match any known compilable pattern.
/// This is called after the hot-threshold is reached (N executions).
pub fn try_compile_insert(ops: &[VdbeOp]) -> Option<SimpleInsertTemplate> {
    // Scan for the pattern:
    //   ... (setup opcodes: Init, Transaction, OpenWrite, etc.)
    //   NewRowid(cursor, r_rowid)  OR  FusedAppendInsert(cursor, r_start, n_cols)
    //   MakeRecord(r_start, n_cols, r_record)  [if not fused]
    //   Insert(cursor, r_record, r_rowid)      [if not fused]
    //   ... (Close, Halt)
    //
    // Guard: no IdxInsert (= no secondary indexes), no triggers, no FKs.

    // Check guards first
    let has_idx_insert = ops.iter().any(|op| op.opcode == Opcode::IdxInsert);
    if has_idx_insert {
        return None; // Table has secondary indexes
    }

    // Look for FusedAppendInsert (already optimized by peephole)
    if let Some(fused) = ops.iter().find(|op| op.opcode == Opcode::FusedAppendInsert) {
        return Some(SimpleInsertTemplate {
            cursor_id: fused.p1,
            root_page: find_root_page(ops, fused.p1)?,
            num_cols: fused.p3,
            first_col_reg: fused.p2,
            insert_flags: fused.p5,
        });
    }

    // Look for unfused NewRowid + MakeRecord + Insert
    let new_rowid = ops.iter().find(|op| op.opcode == Opcode::NewRowid)?;
    let make_record = ops.iter().find(|op| op.opcode == Opcode::MakeRecord)?;
    let insert = ops.iter().find(|op| op.opcode == Opcode::Insert)?;

    // Verify consistency
    if new_rowid.p1 != insert.p1 {
        return None; // Different cursors
    }
    if make_record.p3 != insert.p2 {
        return None; // Record register mismatch
    }
    let oe_flag = insert.p5 & 0x0F;
    if oe_flag != 2 {
        return None; // Not ABORT mode
    }

    Some(SimpleInsertTemplate {
        cursor_id: new_rowid.p1,
        root_page: find_root_page(ops, new_rowid.p1)?,
        num_cols: make_record.p2,
        first_col_reg: make_record.p1,
        insert_flags: insert.p5,
    })
}

/// Find the root page for a cursor by scanning OpenWrite opcodes.
fn find_root_page(ops: &[VdbeOp], cursor_id: i32) -> Option<i32> {
    ops.iter()
        .find(|op| {
            (op.opcode == Opcode::OpenWrite || op.opcode == Opcode::FusedOpenWriteLast)
                && op.p1 == cursor_id
        })
        .map(|op| op.p2)
}
