use ftui_layout::{
    PaneId, PaneInteractionTimeline, PaneInteractionTimelineCheckpointDecision,
    PaneInteractionTimelineReplayDiagnostics, PaneLeaf, PaneNodeKind, PaneOperation, PanePlacement,
    PaneSplitRatio, PaneTree, SplitAxis,
};
use serde::Serialize;
use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
struct ScenarioSpec {
    name: &'static str,
    leaf_count: usize,
    operations_per_iteration: usize,
    iterations: usize,
    warmup_iterations: usize,
}

#[derive(Debug, Serialize)]
struct HarnessManifest {
    scenario: String,
    benchmark_binary: String,
    leaf_count: usize,
    operations_per_iteration: usize,
    iterations: usize,
    warmup_iterations: usize,
    baseline_hash: u64,
    final_hash: u64,
    aggregate_hash: u64,
    elapsed_ns: u128,
    ns_per_iteration: u128,
    checkpoint_interval: usize,
    checkpoint_count: usize,
    checkpoint_hit: bool,
    replay_start_idx: usize,
    replay_depth: usize,
    estimated_snapshot_cost_ns: u128,
    estimated_replay_step_cost_ns: u128,
    checkpoint_decision: PaneInteractionTimelineCheckpointDecision,
    baseline_snapshot_path: String,
    final_snapshot_path: String,
    log_path: String,
}

#[derive(Debug)]
struct IterationResult {
    final_hash: u64,
    applied_len: usize,
    replay_diagnostics: PaneInteractionTimelineReplayDiagnostics,
    replay_elapsed_ns: u128,
    snapshot: ftui_layout::PaneTreeSnapshot,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut spec = ScenarioSpec {
        name: "timeline-ratios-32x2000",
        leaf_count: 32,
        operations_per_iteration: 32,
        iterations: 2_000,
        warmup_iterations: 200,
    };
    let mut out_dir = default_out_dir()?;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bench" => {}
            "--out-dir" => {
                let value = args.next().ok_or("missing value for --out-dir")?;
                out_dir = PathBuf::from(value);
            }
            "--iterations" => {
                let value = args.next().ok_or("missing value for --iterations")?;
                spec.iterations = value.parse()?;
            }
            "--warmup-iterations" => {
                let value = args.next().ok_or("missing value for --warmup-iterations")?;
                spec.warmup_iterations = value.parse()?;
            }
            "--operations" => {
                let value = args.next().ok_or("missing value for --operations")?;
                spec.operations_per_iteration = value.parse()?;
            }
            "--leaf-count" => {
                let value = args.next().ok_or("missing value for --leaf-count")?;
                spec.leaf_count = value.parse()?;
            }
            "--scenario-name" => {
                let value = args.next().ok_or("missing value for --scenario-name")?;
                spec.name = Box::leak(value.into_boxed_str());
            }
            "-h" | "--help" => {
                print_help();
                return Ok(());
            }
            other => return Err(format!("unknown option: {other}").into()),
        }
    }

    if spec.leaf_count < 2 {
        return Err("leaf_count must be at least 2".into());
    }
    if spec.operations_per_iteration == 0 || spec.iterations == 0 {
        return Err("operations and iterations must be > 0".into());
    }

    fs::create_dir_all(&out_dir)?;

    let baseline = build_pane_tree(spec.leaf_count);
    let baseline_hash = baseline.state_hash();
    let split_ids = pane_split_ids(&baseline);
    if split_ids.is_empty() {
        return Err("scenario produced no split ids".into());
    }
    let ratios = [
        PaneSplitRatio::new(3, 2).expect("valid ratio"),
        PaneSplitRatio::new(2, 3).expect("valid ratio"),
        PaneSplitRatio::new(5, 4).expect("valid ratio"),
        PaneSplitRatio::new(4, 5).expect("valid ratio"),
    ];

    let mut log_lines = Vec::new();
    log_lines.push(format!(
        "scenario={} leaf_count={} operations_per_iteration={} iterations={} warmup_iterations={}",
        spec.name,
        spec.leaf_count,
        spec.operations_per_iteration,
        spec.iterations,
        spec.warmup_iterations
    ));
    log_lines.push(format!("baseline_hash={baseline_hash}"));

    let (estimated_snapshot_cost_ns, estimated_replay_step_cost_ns, checkpoint_decision) =
        measure_checkpoint_decision_inputs(
            &baseline,
            &split_ids,
            &ratios,
            spec.operations_per_iteration,
        )?;
    log_lines.push(format!(
        "checkpoint_decision interval={} snapshot_cost_ns={} replay_step_cost_ns={} estimated_replay_depth_ns={}",
        checkpoint_decision.checkpoint_interval,
        checkpoint_decision.estimated_snapshot_cost_ns,
        checkpoint_decision.estimated_replay_step_cost_ns,
        checkpoint_decision.estimated_replay_depth_ns
    ));

    for warmup_idx in 0..spec.warmup_iterations {
        let result = execute_iteration(
            &baseline,
            &split_ids,
            &ratios,
            spec.operations_per_iteration,
        )?;
        if warmup_idx == 0 {
            log_lines.push(format!(
                "warmup_sample final_hash={} applied_len={}",
                result.final_hash, result.applied_len
            ));
        }
    }

    let start = Instant::now();
    let mut aggregate_hash = 0u64;
    let mut sample_snapshot = None;
    let mut sample_hash = 0u64;
    let mut sample_replay_diagnostics = None;
    for iter_idx in 0..spec.iterations {
        let result = execute_iteration(
            &baseline,
            &split_ids,
            &ratios,
            spec.operations_per_iteration,
        )?;
        aggregate_hash ^= result.final_hash.rotate_left((iter_idx % 63) as u32);
        if iter_idx == 0 || iter_idx + 1 == spec.iterations {
            log_lines.push(format!(
                "iteration={} final_hash={} applied_len={} snapshot_nodes={}",
                iter_idx,
                result.final_hash,
                result.applied_len,
                result.snapshot.nodes.len()
            ));
            log_lines.push(format!(
                "iteration={} checkpoint_interval={} checkpoint_count={} checkpoint_hit={} replay_start_idx={} replay_depth={} replay_elapsed_ns={}",
                iter_idx,
                result.replay_diagnostics.checkpoint_interval,
                result.replay_diagnostics.checkpoint_count,
                result.replay_diagnostics.checkpoint_hit,
                result.replay_diagnostics.replay_start_idx,
                result.replay_diagnostics.replay_depth,
                result.replay_elapsed_ns
            ));
        }
        sample_hash = result.final_hash;
        sample_replay_diagnostics = Some(result.replay_diagnostics);
        sample_snapshot = Some(result.snapshot);
    }
    let elapsed = start.elapsed();

    let baseline_snapshot_path = out_dir.join("baseline_snapshot.json");
    let final_snapshot_path = out_dir.join("final_snapshot.json");
    let manifest_path = out_dir.join("manifest.json");
    let log_path = out_dir.join("run.log");

    let baseline_snapshot = baseline.to_snapshot();
    let final_snapshot = sample_snapshot.ok_or("missing sample snapshot after execution")?;
    let replay_diagnostics =
        sample_replay_diagnostics.ok_or("missing replay diagnostics after execution")?;

    write_json(&baseline_snapshot_path, &baseline_snapshot)?;
    write_json(&final_snapshot_path, &final_snapshot)?;
    write_lines(&log_path, &log_lines)?;

    let manifest = HarnessManifest {
        scenario: spec.name.to_string(),
        benchmark_binary: env::current_exe()?.display().to_string(),
        leaf_count: spec.leaf_count,
        operations_per_iteration: spec.operations_per_iteration,
        iterations: spec.iterations,
        warmup_iterations: spec.warmup_iterations,
        baseline_hash,
        final_hash: sample_hash,
        aggregate_hash,
        elapsed_ns: elapsed.as_nanos(),
        ns_per_iteration: elapsed.as_nanos() / u128::from(spec.iterations as u64),
        checkpoint_interval: replay_diagnostics.checkpoint_interval,
        checkpoint_count: replay_diagnostics.checkpoint_count,
        checkpoint_hit: replay_diagnostics.checkpoint_hit,
        replay_start_idx: replay_diagnostics.replay_start_idx,
        replay_depth: replay_diagnostics.replay_depth,
        estimated_snapshot_cost_ns,
        estimated_replay_step_cost_ns,
        checkpoint_decision,
        baseline_snapshot_path: baseline_snapshot_path.display().to_string(),
        final_snapshot_path: final_snapshot_path.display().to_string(),
        log_path: log_path.display().to_string(),
    };
    write_json(&manifest_path, &manifest)?;

    println!("pane_profile_harness scenario={}", spec.name);
    println!("benchmark_binary={}", manifest.benchmark_binary);
    println!("baseline_hash={}", manifest.baseline_hash);
    println!("final_hash={}", manifest.final_hash);
    println!("aggregate_hash={}", manifest.aggregate_hash);
    println!("elapsed_ns={}", manifest.elapsed_ns);
    println!("ns_per_iteration={}", manifest.ns_per_iteration);
    println!("manifest={}", manifest_path.display());
    println!("baseline_snapshot={}", baseline_snapshot_path.display());
    println!("final_snapshot={}", final_snapshot_path.display());
    println!("log_path={}", log_path.display());
    println!(
        "HARNESS_MANIFEST_JSON={}",
        serde_json::to_string(&manifest)?
    );
    println!(
        "HARNESS_BASELINE_SNAPSHOT_JSON={}",
        serde_json::to_string(&baseline_snapshot)?
    );
    println!(
        "HARNESS_FINAL_SNAPSHOT_JSON={}",
        serde_json::to_string(&final_snapshot)?
    );
    println!(
        "HARNESS_RUN_LOG_JSON={}",
        serde_json::to_string(&log_lines)?
    );

    Ok(())
}

fn execute_iteration(
    baseline: &PaneTree,
    split_ids: &[PaneId],
    ratios: &[PaneSplitRatio],
    operations_per_iteration: usize,
) -> Result<IterationResult, Box<dyn std::error::Error>> {
    let mut tree = baseline.clone();
    let mut timeline = PaneInteractionTimeline::with_baseline(baseline);
    for idx in 0..operations_per_iteration {
        let split = split_ids[idx % split_ids.len()];
        let ratio = ratios[idx % ratios.len()];
        timeline.apply_and_record(
            &mut tree,
            idx as u64,
            80_000 + idx as u64,
            PaneOperation::SetSplitRatio { split, ratio },
        )?;
    }
    let replay_diagnostics = timeline.replay_diagnostics();
    let replay_start = Instant::now();
    let replayed = timeline.replay()?;
    let replay_elapsed_ns = replay_start.elapsed().as_nanos();
    let replay_hash = replayed.state_hash();
    if replay_hash != tree.state_hash() {
        return Err(format!(
            "replay hash mismatch: replayed={replay_hash} tree={}",
            tree.state_hash()
        )
        .into());
    }
    Ok(IterationResult {
        final_hash: replay_hash,
        applied_len: timeline.applied_len(),
        replay_diagnostics,
        replay_elapsed_ns,
        snapshot: replayed.to_snapshot(),
    })
}

fn measure_checkpoint_decision_inputs(
    baseline: &PaneTree,
    split_ids: &[PaneId],
    ratios: &[PaneSplitRatio],
    operations_per_iteration: usize,
) -> Result<(u128, u128, PaneInteractionTimelineCheckpointDecision), Box<dyn std::error::Error>> {
    let snapshot_start = Instant::now();
    let _snapshot = baseline.to_snapshot();
    let snapshot_cost_ns = snapshot_start.elapsed().as_nanos();

    let result = execute_iteration(baseline, split_ids, ratios, operations_per_iteration)?;
    let replay_steps = result.replay_diagnostics.replay_depth.max(1) as u128;
    let replay_step_cost_ns = (result.replay_elapsed_ns / replay_steps).max(1);
    let decision =
        PaneInteractionTimeline::checkpoint_decision(snapshot_cost_ns, replay_step_cost_ns);
    Ok((snapshot_cost_ns.max(1), replay_step_cost_ns, decision))
}

fn pane_split_ids(tree: &PaneTree) -> Vec<PaneId> {
    tree.nodes()
        .filter_map(|node| matches!(node.kind, PaneNodeKind::Split(_)).then_some(node.id))
        .collect()
}

fn build_pane_tree(leaf_count: usize) -> PaneTree {
    assert!(leaf_count >= 1);
    let mut tree = PaneTree::singleton("leaf-0");
    if leaf_count == 1 {
        return tree;
    }

    let ratio = PaneSplitRatio::new(1, 1).expect("valid ratio");
    let mut split_queue = std::collections::VecDeque::from([tree.root()]);
    for idx in 1..leaf_count {
        let target = split_queue
            .pop_front()
            .expect("split queue must yield leaf");
        let axis = if idx % 2 == 0 {
            SplitAxis::Horizontal
        } else {
            SplitAxis::Vertical
        };
        let outcome = tree
            .apply_operation(
                idx as u64,
                PaneOperation::SplitLeaf {
                    target,
                    axis,
                    ratio,
                    placement: PanePlacement::ExistingFirst,
                    new_leaf: PaneLeaf::new(format!("leaf-{idx}")),
                },
            )
            .expect("deterministic split should succeed");
        let new_leaf_id = outcome
            .touched_nodes
            .into_iter()
            .find(|node_id| {
                *node_id != target
                    && matches!(
                        tree.node(*node_id),
                        Some(node) if matches!(node.kind, PaneNodeKind::Leaf(_))
                    )
            })
            .expect("split should create new leaf");
        split_queue.push_back(target);
        split_queue.push_back(new_leaf_id);
    }
    tree
}

fn default_out_dir() -> io::Result<PathBuf> {
    Ok(env::current_dir()?.join("target/pane-profiling/bd-1k7ek.1/pane_core_profile_harness"))
}

fn print_help() {
    println!("Usage: cargo bench -p ftui-layout --bench pane_profile_harness -- [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --out-dir PATH             write artifacts under PATH");
    println!("  --iterations N             measured iterations (default: 2000)");
    println!("  --warmup-iterations N      warmup iterations before timing (default: 200)");
    println!("  --operations N             operations per iteration (default: 32)");
    println!("  --leaf-count N             pane leaf count (default: 32)");
    println!("  --scenario-name NAME       label to embed in the manifest");
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, value)?;
    Ok(())
}

fn write_lines(path: &Path, lines: &[String]) -> io::Result<()> {
    let mut file = File::create(path)?;
    for line in lines {
        writeln!(file, "{line}")?;
    }
    Ok(())
}
