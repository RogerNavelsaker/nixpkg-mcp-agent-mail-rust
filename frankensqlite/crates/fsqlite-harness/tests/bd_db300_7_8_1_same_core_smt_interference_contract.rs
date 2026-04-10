//! Contract tests for db300_topology_interference_contract.toml
//! (bd-db300.7.8.1).
//!
//! This slice pins the topology-case resolution entrypoint so later
//! measurement beads inherit one stable case matrix, log vocabulary, and
//! artifact layout instead of inventing new selectors or file names.

#![allow(clippy::needless_lifetimes, clippy::struct_field_names)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use tempfile::tempdir;

const BEAD_ID: &str = "bd-db300.7.8.1";
const CONTRACT_PATH: &str = "db300_topology_interference_contract.toml";
const ENTRYPOINT_PATH: &str = "scripts/verify_g8_1_same_core_smt_interference.sh";
const TOPOLOGY_BUNDLE_SCRIPT: &str = "scripts/verify_bd_db300_1_6_1_topology_bundle.sh";
const REQUIRED_CASE_IDS: [&str; 3] = [
    "same_core_serialized",
    "same_llc_diff_core_pair",
    "smt_sibling_pair",
];
const REQUIRED_EVENT_FAMILIES: [&str; 3] = [
    "topology_case_resolution",
    "topology_interference_measurement",
    "verification_bundle_summary",
];
const REQUIRED_ARTIFACTS: [&str; 7] = [
    "manifest.json",
    "case_matrix.json",
    "structured_logs.ndjson",
    "summary.md",
    "rerun_entrypoint.sh",
    "topology_bundle/hardware_discovery_bundle.json",
    "topology_bundle/hardware_discovery_summary.md",
];

#[derive(Debug, Deserialize)]
struct ContractDocument {
    meta: Meta,
    global_defaults: GlobalDefaults,
    structured_log_common_fields: RequiredFieldSet,
    structured_log_case_fields: RequiredFieldSet,
    structured_log_measurement_fields: RequiredFieldSet,
    structured_log_failure_fields: RequiredFieldSet,
    provenance: Provenance,
    mechanical_comparison: MechanicalComparison,
    artifact_layout: ArtifactLayout,
    first_slice: FirstSlice,
    #[serde(default, rename = "event_family")]
    event_families: Vec<EventFamily>,
    #[serde(default, rename = "case")]
    cases: Vec<CaseContract>,
}

#[derive(Debug, Deserialize)]
struct Meta {
    schema_version: String,
    bead_id: String,
    track_id: String,
    generated_at: String,
    contract_owner: String,
    operator_entrypoint: String,
    topology_bundle_contract_ref: String,
    artifact_manifest_contract_ref: String,
    structured_logging_contract_ref: String,
}

#[derive(Debug, Deserialize)]
struct GlobalDefaults {
    event_schema_id: String,
    case_matrix_schema_id: String,
    artifact_manifest_schema_id: String,
    run_identity_schema_id: String,
    default_surface_id: String,
    default_pillar_id: String,
    default_primitive_class: String,
    default_placement_profile_id: String,
    default_hardware_class_id: String,
    concurrent_writer_requirement: String,
    missing_case_policy: String,
    replay_policy: String,
    mechanical_comparison_policy: String,
    first_slice_id: String,
    first_slice_goal: String,
}

#[derive(Debug, Deserialize)]
struct RequiredFieldSet {
    required_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Provenance {
    required_ids: Vec<String>,
    required_selector_bindings: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MechanicalComparison {
    join_key_fields: Vec<String>,
    required_match_fields: Vec<String>,
    ignored_ephemeral_fields: Vec<String>,
    comparison_status_on_missing_field: String,
    comparison_status_on_selector_mismatch: String,
}

#[derive(Debug, Deserialize)]
struct ArtifactLayout {
    bundle_dir_template: String,
    required_files: Vec<String>,
    optional_files: Vec<String>,
    required_directories: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FirstSlice {
    slice_id: String,
    entrypoint: String,
    topology_bundle_dependency: String,
    contract_test: String,
    required_event_families: Vec<String>,
    required_artifacts: Vec<String>,
    success_rule: String,
}

#[derive(Debug, Deserialize)]
struct EventFamily {
    family_id: String,
    required_fields: Vec<String>,
    mechanical_compare_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CaseContract {
    case_id: String,
    case_kind: String,
    placement_profile_id: String,
    primitive_class: String,
    selector_strategy: String,
    availability_guard: String,
    cpu_pair_contract: String,
    smt_relationship: String,
}

#[derive(Debug, Deserialize)]
struct CaseMatrix {
    schema_version: String,
    bead_id: String,
    run_id: String,
    trace_id: String,
    scenario_id: String,
    placement_profile_id: String,
    hardware_class_id: String,
    hardware_signature: String,
    primitive_class: String,
    #[serde(default)]
    cases: Vec<RenderedCase>,
}

#[derive(Debug, Deserialize)]
struct RenderedCase {
    case_id: String,
    case_kind: String,
    availability: String,
    #[serde(default)]
    cpu_pair: Vec<u32>,
    cpu_pair_signature: Option<String>,
    llc_domain: Option<String>,
    smt_relationship: String,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    schema_version: String,
    bead_id: String,
    run_id: String,
    trace_id: String,
    scenario_id: String,
    operator_entrypoint: String,
    contract_path: String,
    topology_bundle_script: String,
    case_matrix_path: String,
    structured_logs_path: String,
    summary_path: String,
    rerun_entrypoint_path: String,
    required_log_families: Vec<String>,
    required_log_fields: BTreeMap<String, Vec<String>>,
    artifact_names: Vec<String>,
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .canonicalize()
        .expect("workspace root should canonicalize")
}

fn load_contract() -> ContractDocument {
    let path = workspace_root().join(CONTRACT_PATH);
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    toml::from_str::<ContractDocument>(&content)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}

fn as_set(values: &[String]) -> BTreeSet<&str> {
    values.iter().map(String::as_str).collect()
}

fn expected<'a>(values: &'a [&'a str]) -> BTreeSet<&'a str> {
    values.iter().copied().collect()
}

fn by_family(document: &ContractDocument) -> BTreeMap<&str, &EventFamily> {
    document
        .event_families
        .iter()
        .map(|family| (family.family_id.as_str(), family))
        .collect()
}

fn by_case(document: &ContractDocument) -> BTreeMap<&str, &CaseContract> {
    document
        .cases
        .iter()
        .map(|case| (case.case_id.as_str(), case))
        .collect()
}

#[test]
fn meta_and_operator_entrypoint_are_pinned() {
    let document = load_contract();
    let workspace = workspace_root();

    assert_eq!(document.meta.schema_version, "1.0.0");
    assert_eq!(document.meta.bead_id, BEAD_ID);
    assert_eq!(document.meta.track_id, "bd-db300.7.8");
    assert!(!document.meta.generated_at.trim().is_empty());
    assert!(!document.meta.contract_owner.trim().is_empty());
    assert_eq!(document.meta.operator_entrypoint, ENTRYPOINT_PATH);
    assert_eq!(
        document.meta.topology_bundle_contract_ref,
        TOPOLOGY_BUNDLE_SCRIPT
    );
    assert_eq!(
        document.meta.artifact_manifest_contract_ref,
        "crates/fsqlite-e2e/src/fixture_select.rs"
    );
    assert_eq!(
        document.meta.structured_logging_contract_ref,
        "db300_structured_logging_contract.toml"
    );
    assert!(
        workspace.join(ENTRYPOINT_PATH).is_file(),
        "operator entrypoint must exist"
    );
    assert!(
        workspace.join(TOPOLOGY_BUNDLE_SCRIPT).is_file(),
        "topology bundle script must exist"
    );

    assert_eq!(
        document.global_defaults.event_schema_id,
        "fsqlite.db300.topology_interference_event.v1"
    );
    assert_eq!(
        document.global_defaults.case_matrix_schema_id,
        "fsqlite.db300.topology_interference_case_matrix.v1"
    );
    assert_eq!(
        document.global_defaults.artifact_manifest_schema_id,
        "fsqlite.db300.topology_interference_manifest.v1"
    );
    assert_eq!(
        document.global_defaults.run_identity_schema_id,
        "fsqlite.db300.run_identity.v1"
    );
    assert_eq!(
        document.global_defaults.default_surface_id,
        "g8_1_topology_interference_suite"
    );
    assert_eq!(document.global_defaults.default_pillar_id, "G8");
    assert_eq!(
        document.global_defaults.default_primitive_class,
        "topology_interference_smoke"
    );
    assert_eq!(
        document.global_defaults.default_placement_profile_id,
        "recommended_pinned"
    );
    assert_eq!(
        document.global_defaults.default_hardware_class_id,
        "linux_x86_64_many_core_numa"
    );
    assert!(
        document
            .global_defaults
            .concurrent_writer_requirement
            .contains("ON by default")
    );
    assert!(
        document
            .global_defaults
            .missing_case_policy
            .contains("availability=unavailable")
    );
    assert!(
        document
            .global_defaults
            .replay_policy
            .contains("rerun_entrypoint.sh")
    );
    assert!(
        document
            .global_defaults
            .mechanical_comparison_policy
            .contains("cpu_pair_signature")
    );
}

#[test]
fn case_matrix_and_log_schema_are_complete() {
    let document = load_contract();
    let family_map = by_family(&document);
    let case_map = by_case(&document);

    assert_eq!(
        expected(&REQUIRED_EVENT_FAMILIES),
        family_map.keys().copied().collect()
    );
    assert_eq!(
        expected(&REQUIRED_CASE_IDS),
        case_map.keys().copied().collect()
    );

    assert_eq!(
        as_set(&document.structured_log_common_fields.required_fields),
        expected(&[
            "schema_version",
            "trace_id",
            "scenario_id",
            "bead_id",
            "run_id",
            "phase",
            "event_type",
            "outcome",
            "timestamp",
            "message",
            "surface_id",
            "pillar_id",
            "event_family",
            "claim_id",
            "evidence_id",
            "evidence_root",
            "placement_profile_id",
            "hardware_class_id",
            "hardware_signature",
            "source_revision",
            "beads_data_hash",
        ])
    );
    assert_eq!(
        as_set(&document.structured_log_case_fields.required_fields),
        expected(&[
            "case_id",
            "case_kind",
            "primitive_class",
            "cpu_pair",
            "cpu_pair_signature",
            "llc_domain",
            "smt_relationship",
            "availability",
        ])
    );
    assert_eq!(
        as_set(&document.structured_log_measurement_fields.required_fields),
        expected(&[
            "throughput",
            "p50_ns",
            "p95_ns",
            "p99_ns",
            "fairness_score",
            "wakeups",
            "migrations",
            "ownership_events",
        ])
    );
    assert_eq!(
        as_set(&document.structured_log_failure_fields.required_fields),
        expected(&[
            "first_failure_summary",
            "first_failure_stage",
            "first_failure_artifact",
            "diagnostic_json_pointer",
            "replay_command",
        ])
    );
    assert!(
        document
            .provenance
            .required_ids
            .iter()
            .any(|field| field == "artifact_bundle_relpath")
    );
    assert!(
        document
            .provenance
            .required_selector_bindings
            .iter()
            .any(|field| field == "PRIMITIVE_CLASS")
    );
    assert!(
        document
            .mechanical_comparison
            .join_key_fields
            .iter()
            .any(|field| field == "cpu_pair_signature")
    );
    assert!(
        document
            .mechanical_comparison
            .required_match_fields
            .iter()
            .any(|field| field == "llc_domain")
    );
    assert!(
        document
            .mechanical_comparison
            .ignored_ephemeral_fields
            .iter()
            .any(|field| field == "timestamp")
    );
    assert_eq!(
        document
            .mechanical_comparison
            .comparison_status_on_missing_field,
        "observability_gap"
    );
    assert_eq!(
        document
            .mechanical_comparison
            .comparison_status_on_selector_mismatch,
        "not_comparable"
    );

    let same_core = case_map["same_core_serialized"];
    assert_eq!(same_core.case_kind, "same_core");
    assert_eq!(same_core.smt_relationship, "same_logical_cpu");
    assert!(same_core.cpu_pair_contract.contains("=="));

    let smt = case_map["smt_sibling_pair"];
    assert_eq!(smt.case_kind, "smt_sibling");
    assert_eq!(smt.smt_relationship, "siblings_same_core");
    assert!(smt.availability_guard.contains("threads_per_core"));

    let llc = case_map["same_llc_diff_core_pair"];
    assert_eq!(llc.case_kind, "same_llc");
    assert_eq!(llc.smt_relationship, "not_siblings_same_llc");
    assert!(llc.cpu_pair_contract.contains("shared_cpu_list"));

    for case in case_map.values() {
        assert_eq!(case.placement_profile_id, "recommended_pinned");
        assert_eq!(case.primitive_class, "topology_interference_smoke");
        assert!(!case.selector_strategy.trim().is_empty());
    }

    let resolution = family_map["topology_case_resolution"];
    assert!(
        resolution
            .required_fields
            .iter()
            .any(|field| field == "availability")
    );
    assert!(
        resolution
            .mechanical_compare_fields
            .iter()
            .any(|field| field == "cpu_pair_signature")
    );

    let measurement = family_map["topology_interference_measurement"];
    assert!(
        measurement
            .required_fields
            .iter()
            .any(|field| field == "throughput")
    );
    assert!(
        measurement
            .mechanical_compare_fields
            .iter()
            .any(|field| field == "fairness_score")
    );
}

#[test]
fn artifact_layout_and_first_slice_are_mechanical() {
    let document = load_contract();

    assert_eq!(
        document.artifact_layout.bundle_dir_template,
        "artifacts/bd-db300.7.8.1/{run_id}"
    );
    assert_eq!(
        as_set(&document.artifact_layout.required_files),
        expected(&REQUIRED_ARTIFACTS)
    );
    assert!(
        document
            .artifact_layout
            .optional_files
            .iter()
            .any(|entry| entry == "first_failure.json")
    );
    assert!(
        document
            .artifact_layout
            .required_directories
            .iter()
            .any(|entry| entry == "topology_bundle")
    );

    assert_eq!(document.first_slice.slice_id, "g8_1_case_resolution_smoke");
    assert_eq!(document.first_slice.entrypoint, ENTRYPOINT_PATH);
    assert_eq!(
        document.first_slice.topology_bundle_dependency,
        TOPOLOGY_BUNDLE_SCRIPT
    );
    assert_eq!(
        document.global_defaults.first_slice_id,
        document.first_slice.slice_id
    );
    assert!(
        document
            .global_defaults
            .first_slice_goal
            .contains("same_core_serialized")
    );
    assert!(
        document
            .first_slice
            .contract_test
            .contains("bd_db300_7_8_1_same_core_smt_interference_contract")
    );
    assert_eq!(
        as_set(&document.first_slice.required_event_families),
        expected(&["topology_case_resolution", "verification_bundle_summary"])
    );
    assert_eq!(
        as_set(&document.first_slice.required_artifacts),
        expected(&REQUIRED_ARTIFACTS)
    );
    assert!(
        document
            .first_slice
            .success_rule
            .contains("available or unavailable")
    );
}

#[test]
fn operator_entrypoint_smoke_renders_replayable_artifacts() {
    let workspace = workspace_root();
    let temp = tempdir().expect("tempdir");
    let artifact_dir = temp.path().join("g8_1_smoke");
    let output = Command::new("bash")
        .arg(workspace.join(ENTRYPOINT_PATH))
        .current_dir(&workspace)
        .env("SKIP_CONTRACT_TEST", "1")
        .env("RUN_ID", "bd-db300.7.8.1-smoke")
        .env("TRACE_ID", "trace-bd-db300.7.8.1-smoke")
        .env("SCENARIO_ID", "G8-1-SMOKE")
        .env("ARTIFACT_DIR", &artifact_dir)
        .output()
        .expect("run operator entrypoint");

    if !output.status.success() {
        panic!(
            "entrypoint failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    for artifact in REQUIRED_ARTIFACTS {
        assert!(
            artifact_dir.join(artifact).exists(),
            "missing artifact {}",
            artifact
        );
    }

    let case_matrix: CaseMatrix = serde_json::from_str(
        &fs::read_to_string(artifact_dir.join("case_matrix.json")).expect("case_matrix.json"),
    )
    .expect("parse case_matrix.json");
    assert_eq!(
        case_matrix.schema_version,
        "fsqlite.db300.topology_interference_case_matrix.v1"
    );
    assert_eq!(case_matrix.bead_id, BEAD_ID);
    assert_eq!(case_matrix.run_id, "bd-db300.7.8.1-smoke");
    assert_eq!(case_matrix.trace_id, "trace-bd-db300.7.8.1-smoke");
    assert_eq!(case_matrix.scenario_id, "G8-1-SMOKE");
    assert_eq!(case_matrix.primitive_class, "topology_interference_smoke");
    assert_eq!(case_matrix.placement_profile_id, "recommended_pinned");
    assert_eq!(case_matrix.hardware_class_id, "linux_x86_64_many_core_numa");
    assert!(!case_matrix.hardware_signature.is_empty());
    assert_eq!(case_matrix.cases.len(), REQUIRED_CASE_IDS.len());

    let case_map: BTreeMap<&str, &RenderedCase> = case_matrix
        .cases
        .iter()
        .map(|case| (case.case_id.as_str(), case))
        .collect();
    assert_eq!(
        case_map.keys().copied().collect::<BTreeSet<_>>(),
        expected(&REQUIRED_CASE_IDS)
    );

    let same_core = case_map["same_core_serialized"];
    assert_eq!(same_core.case_kind, "same_core");
    assert_eq!(same_core.availability, "available");
    assert_eq!(same_core.cpu_pair.len(), 2);
    assert_eq!(same_core.cpu_pair[0], same_core.cpu_pair[1]);
    assert_eq!(same_core.smt_relationship, "same_logical_cpu");
    assert!(same_core.cpu_pair_signature.is_some());

    let smt = case_map["smt_sibling_pair"];
    assert_eq!(smt.case_kind, "smt_sibling");
    assert_eq!(smt.smt_relationship, "siblings_same_core");
    match smt.availability.as_str() {
        "available" => {
            assert_eq!(smt.cpu_pair.len(), 2);
            assert_ne!(smt.cpu_pair[0], smt.cpu_pair[1]);
            assert!(smt.cpu_pair_signature.is_some());
        }
        "unavailable" => {
            assert!(smt.cpu_pair.is_empty());
            assert!(smt.cpu_pair_signature.is_none());
        }
        other => panic!("unexpected smt availability {other}"),
    }

    let llc = case_map["same_llc_diff_core_pair"];
    assert_eq!(llc.case_kind, "same_llc");
    assert_eq!(llc.smt_relationship, "not_siblings_same_llc");
    match llc.availability.as_str() {
        "available" => {
            assert_eq!(llc.cpu_pair.len(), 2);
            assert_ne!(llc.cpu_pair[0], llc.cpu_pair[1]);
            assert!(llc.llc_domain.is_some());
            assert!(llc.cpu_pair_signature.is_some());
        }
        "unavailable" => {
            assert!(llc.cpu_pair.is_empty());
        }
        other => panic!("unexpected llc availability {other}"),
    }

    let manifest: Manifest = serde_json::from_str(
        &fs::read_to_string(artifact_dir.join("manifest.json")).expect("manifest.json"),
    )
    .expect("parse manifest.json");
    assert_eq!(
        manifest.schema_version,
        "fsqlite.db300.topology_interference_manifest.v1"
    );
    assert_eq!(manifest.bead_id, BEAD_ID);
    assert_eq!(manifest.run_id, "bd-db300.7.8.1-smoke");
    assert_eq!(manifest.trace_id, "trace-bd-db300.7.8.1-smoke");
    assert_eq!(manifest.scenario_id, "G8-1-SMOKE");
    assert_eq!(manifest.operator_entrypoint, ENTRYPOINT_PATH);
    assert_eq!(manifest.contract_path, CONTRACT_PATH);
    assert_eq!(manifest.topology_bundle_script, TOPOLOGY_BUNDLE_SCRIPT);
    assert_eq!(manifest.case_matrix_path, "case_matrix.json");
    assert_eq!(manifest.structured_logs_path, "structured_logs.ndjson");
    assert_eq!(manifest.summary_path, "summary.md");
    assert_eq!(manifest.rerun_entrypoint_path, "rerun_entrypoint.sh");
    assert_eq!(
        manifest
            .required_log_families
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        expected(&["topology_case_resolution", "verification_bundle_summary"])
    );
    assert_eq!(
        manifest
            .artifact_names
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        expected(&REQUIRED_ARTIFACTS)
    );
    assert!(
        manifest
            .required_log_fields
            .get("topology_case_resolution")
            .expect("resolution fields")
            .iter()
            .any(|field| field == "cpu_pair_signature")
    );

    let log_lines = fs::read_to_string(artifact_dir.join("structured_logs.ndjson"))
        .expect("structured_logs.ndjson");
    let log_events: Vec<serde_json::Value> = log_lines
        .lines()
        .map(|line| serde_json::from_str(line).expect("structured log line should parse"))
        .collect();
    assert!(log_events.iter().any(|event| {
        event
            .get("event_family")
            .and_then(serde_json::Value::as_str)
            == Some("topology_case_resolution")
    }));
    assert!(log_events.iter().any(|event| {
        event
            .get("event_family")
            .and_then(serde_json::Value::as_str)
            == Some("verification_bundle_summary")
    }));
    assert!(log_events.iter().any(|event| {
        event.get("case_id").and_then(serde_json::Value::as_str) == Some("same_core_serialized")
    }));

    let rerun =
        fs::read_to_string(artifact_dir.join("rerun_entrypoint.sh")).expect("rerun_entrypoint.sh");
    assert!(rerun.contains("verify_g8_1_same_core_smt_interference.sh"));
    assert!(rerun.contains("PLACEMENT_PROFILE_ID"));
    assert!(rerun.contains("PRIMITIVE_CLASS"));
}
