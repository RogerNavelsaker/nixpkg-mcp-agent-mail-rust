use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use fsqlite_types::opcode::{Opcode, P4};
use fsqlite_types::record::{PrecomputedRecordHeader, PrecomputedSerialTypeKind, serialize_record};
use fsqlite_types::value::SqliteValue;
use fsqlite_types::{Cx, PageSize};
use fsqlite_vdbe::ProgramBuilder;
use fsqlite_vdbe::engine::{VdbeEngine, set_vdbe_jit_enabled};

const MAKE_RECORD_REPEATS: usize = 256;

fn fixed_schema_row(column_count: usize) -> Vec<SqliteValue> {
    (0..column_count)
        .map(|idx| {
            let idx_i64 = i64::try_from(idx).expect("benchmark column count should fit into i64");
            let value = (idx_i64 * 97_531) - 49_152;
            SqliteValue::Integer(value)
        })
        .collect()
}

fn integer_header(column_count: usize) -> PrecomputedRecordHeader {
    let kinds = vec![PrecomputedSerialTypeKind::IntegerOrNull; column_count];
    PrecomputedRecordHeader::new(&kinds)
}

fn build_make_record_program(column_count: usize, p4: P4) -> fsqlite_vdbe::VdbeProgram {
    let mut builder = ProgramBuilder::new();
    let first_reg = builder.alloc_regs(i32::try_from(column_count).expect("column count fits"));
    let r_record = builder.alloc_reg();

    for idx in 0..column_count {
        let reg = first_reg + i32::try_from(idx).expect("column index fits");
        let value = match idx % 4 {
            0 => i32::try_from(idx).expect("small integer literal fits"),
            1 => 200 + i32::try_from(idx).expect("small integer literal fits"),
            2 => 70_000 + i32::try_from(idx).expect("small integer literal fits"),
            _ => 5_000_000 + i32::try_from(idx).expect("small integer literal fits"),
        };
        builder.emit_op(Opcode::Integer, value, reg, 0, P4::None, 0);
    }

    for _ in 0..MAKE_RECORD_REPEATS {
        builder.emit_op(
            Opcode::MakeRecord,
            first_reg,
            i32::try_from(column_count).expect("column count fits"),
            r_record,
            p4.clone(),
            0,
        );
    }
    builder.emit_op(Opcode::Halt, 0, 0, 0, P4::None, 0);

    builder.finish().expect("benchmark program should build")
}

fn bench_make_record_fixed_schema(c: &mut Criterion) {
    set_vdbe_jit_enabled(false);
    let mut group = c.benchmark_group("make_record_fixed_schema");

    for column_count in [4_usize, 8, 16, 32] {
        let row = fixed_schema_row(column_count);
        let header = integer_header(column_count);
        let record_bytes = u64::try_from(serialize_record(&row).len()).unwrap_or(u64::MAX);
        let total_bytes =
            record_bytes.saturating_mul(u64::try_from(MAKE_RECORD_REPEATS).unwrap_or(0));
        group.throughput(Throughput::Bytes(total_bytes));

        let generic_program =
            build_make_record_program(column_count, P4::Affinity("D".repeat(column_count)));
        group.bench_with_input(
            BenchmarkId::new("generic", column_count),
            &generic_program,
            |b, program| {
                let execution_cx = Cx::new();
                let mut engine = VdbeEngine::new_with_execution_cx(
                    program.register_count(),
                    &execution_cx,
                    PageSize::DEFAULT,
                );
                b.iter(|| {
                    let outcome = engine
                        .execute(program)
                        .expect("generic MakeRecord benchmark should execute");
                    criterion::black_box(outcome);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("precomputed_header", column_count),
            &build_make_record_program(column_count, P4::PrecomputedHeader(header)),
            |b, program| {
                let execution_cx = Cx::new();
                let mut engine = VdbeEngine::new_with_execution_cx(
                    program.register_count(),
                    &execution_cx,
                    PageSize::DEFAULT,
                );
                b.iter(|| {
                    let outcome = engine
                        .execute(program)
                        .expect("precomputed MakeRecord benchmark should execute");
                    criterion::black_box(outcome);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_make_record_fixed_schema);
criterion_main!(benches);
