//! Contract tests for db300_structured_logging_contract.toml (bd-db300.7.6.1).
//!
//! The goal is to pin one canonical logging vocabulary around the concrete
//! proof and artifact surfaces already present in the tree so later emission
//! beads can wire fields mechanically instead of inventing names ad hoc.

#![allow(clippy::needless_lifetimes, clippy::struct_field_names)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

const BEAD_ID: &str = "bd-db300.7.6.1";
const CONTRACT_PATH: &str = "db300_structured_logging_contract.toml";

const REQUIRED_EVENT_FAMILIES: [&str; 11] = [
    "allocator_lease_delta",
    "cache_hit_ledger",
    "copy_alloc_churn",
    "failure_bundle_summary",
    "hold_wait_distribution",
    "page_touch_class",
    "parent_micro_publish",
    "retry_cause_histogram",
    "split_path_summary",
    "verification_bundle_summary",
    "write_tier_selection",
];

const REQUIRED_SURFACES: [&str; 5] = [
    "a3_benchmark_artifact_manifest",
    "d3_storage_cursor_scratch_decode",
    "g5_regime_atlas",
    "g5_shadow_oracle",
    "g6_policy_runtime_snapshot",
];

#[derive(Debug, Deserialize)]
struct ContractDocument {
    meta: Meta,
    global_defaults: GlobalDefaults,
    common_fields: RequiredFieldSet,
    run_identity: RequiredFieldSet,
    topology_fields: RequiredFieldSet,
    artifact_lineage: ArtifactLineage,
    decision_plane: DecisionPlane,
    failure_context: RequiredFieldSet,
    mechanical_comparison: MechanicalComparison,
    #[serde(default, rename = "event_family")]
    event_families: Vec<EventFamily>,
    #[serde(default, rename = "surface_contract")]
    surface_contracts: Vec<SurfaceContract>,
}

#[derive(Debug, Deserialize)]
struct Meta {
    schema_version: String,
    bead_id: String,
    track_id: String,
    generated_at: String,
    contract_owner: String,
    artifact_manifest_contract_ref: String,
    regime_atlas_contract_ref: String,
    shadow_oracle_contract_ref: String,
    policy_snapshot_contract_ref: String,
}

#[derive(Debug, Deserialize)]
struct GlobalDefaults {
    event_schema_id: String,
    run_identity_schema_id: String,
    mechanical_comparison_schema_id: String,
    id_pattern: String,
    concurrent_writer_requirement: String,
    missing_field_policy: String,
    serialization_policy: String,
    topology_policy: String,
    comparison_policy: String,
}

#[derive(Debug, Deserialize)]
struct RequiredFieldSet {
    required_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ArtifactLineage {
    #[serde(rename = "always_required")]
    always: Vec<String>,
    #[serde(rename = "decision_plane_required")]
    decision_plane: Vec<String>,
    #[serde(rename = "shadow_required")]
    shadow: Vec<String>,
    #[serde(rename = "artifact_required")]
    artifact: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DecisionPlane {
    required_fields: Vec<String>,
    one_of_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MechanicalComparison {
    join_key_fields: Vec<String>,
    required_match_fields: Vec<String>,
    ignored_ephemeral_fields: Vec<String>,
    numeric_counter_fields: Vec<String>,
    id_fields: Vec<String>,
    not_comparable_on_mismatch: Vec<String>,
    require_sorted_keys: bool,
    comparison_status_on_missing_field: String,
    comparison_status_on_topology_mismatch: String,
}

#[derive(Debug, Deserialize)]
struct EventFamily {
    family_id: String,
    purpose: String,
    required_fields: Vec<String>,
    compare_fields: Vec<String>,
    #[serde(default)]
    live_consumers: Vec<String>,
    #[serde(default)]
    future_consumers: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SurfaceContract {
    surface_id: String,
    pillar_id: String,
    source_path: String,
    source_symbol: String,
    artifact_outputs: Vec<String>,
    #[serde(default)]
    observed_log_fields: Vec<String>,
    #[serde(default)]
    observed_artifact_fields: Vec<String>,
    canonical_event_families: Vec<String>,
    #[serde(default)]
    required_upgrade_fields: Vec<String>,
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .canonicalize()
        .expect("workspace root should canonicalize")
}

fn load_contract() -> ContractDocument {
    let path = workspace_root().join(CONTRACT_PATH);
    let content = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "failed to read {} at {}: {error}",
            CONTRACT_PATH,
            path.display()
        )
    });
    toml::from_str::<ContractDocument>(&content).unwrap_or_else(|error| {
        panic!(
            "failed to parse {} at {}: {error}",
            CONTRACT_PATH,
            path.display()
        )
    })
}

fn expected<'a>(values: &'a [&'a str]) -> BTreeSet<&'a str> {
    values.iter().copied().collect()
}

fn as_set(values: &[String]) -> BTreeSet<&str> {
    values.iter().map(String::as_str).collect()
}

fn by_family(document: &ContractDocument) -> BTreeMap<&str, &EventFamily> {
    document
        .event_families
        .iter()
        .map(|family| (family.family_id.as_str(), family))
        .collect()
}

fn by_surface(document: &ContractDocument) -> BTreeMap<&str, &SurfaceContract> {
    document
        .surface_contracts
        .iter()
        .map(|surface| (surface.surface_id.as_str(), surface))
        .collect()
}

fn load_source(path: &str) -> String {
    let full = workspace_root().join(path);
    fs::read_to_string(&full)
        .unwrap_or_else(|error| panic!("failed to read source {}: {error}", full.display()))
}

fn source_mentions_field(source: &str, field: &str) -> bool {
    source.contains(&format!("\"{field}\"")) || source.contains(field)
}

#[test]
fn meta_and_cross_contract_refs_are_pinned() {
    let document = load_contract();

    assert_eq!(document.meta.schema_version, "1.0.0");
    assert_eq!(document.meta.bead_id, BEAD_ID);
    assert_eq!(document.meta.track_id, "bd-db300.7.6");
    assert!(!document.meta.generated_at.trim().is_empty());
    assert!(!document.meta.contract_owner.trim().is_empty());
    assert_eq!(
        document.meta.artifact_manifest_contract_ref,
        "crates/fsqlite-e2e/src/fixture_select.rs"
    );
    assert_eq!(
        document.meta.regime_atlas_contract_ref,
        "db300_regime_atlas_contract.toml"
    );
    assert_eq!(
        document.meta.shadow_oracle_contract_ref,
        "db300_shadow_oracle_contract.toml"
    );
    assert_eq!(
        document.meta.policy_snapshot_contract_ref,
        "db300_policy_snapshot_contract.toml"
    );

    assert_eq!(
        document.global_defaults.event_schema_id,
        "fsqlite.db300.structured_log_event.v1"
    );
    assert_eq!(
        document.global_defaults.run_identity_schema_id,
        "fsqlite.db300.run_identity.v1"
    );
    assert_eq!(
        document.global_defaults.mechanical_comparison_schema_id,
        "fsqlite.db300.mechanical_comparison.v1"
    );
    assert_eq!(
        document.global_defaults.id_pattern,
        "^[A-Za-z0-9][A-Za-z0-9._:-]*$"
    );
    assert!(
        document
            .global_defaults
            .concurrent_writer_requirement
            .contains("ON by default"),
        "concurrent-writer default must remain explicit"
    );
    assert!(
        document
            .global_defaults
            .missing_field_policy
            .contains("observability_gap"),
        "missing-field policy must fail closed"
    );
    assert!(
        document
            .global_defaults
            .serialization_policy
            .contains("sorted keys"),
        "serialization policy must pin sorted-key rendering"
    );
    assert!(
        document
            .global_defaults
            .topology_policy
            .contains("hardware_signature"),
        "topology policy must require hardware identity"
    );
    assert!(
        document
            .global_defaults
            .comparison_policy
            .contains("not_comparable"),
        "comparison policy must pin non-comparable topology mismatches"
    );
}

#[test]
fn field_ledgers_cover_run_identity_topology_lineage_and_decision_plane() {
    let document = load_contract();

    let common = as_set(&document.common_fields.required_fields);
    for required in [
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
    ] {
        assert!(
            common.contains(required),
            "common field missing: {required}"
        );
    }

    let run_identity = as_set(&document.run_identity.required_fields);
    for required in [
        "fixture_id",
        "workload",
        "concurrency",
        "mode_id",
        "build_profile_id",
        "placement_profile_id",
        "hardware_class_id",
        "hardware_signature",
        "source_revision",
        "beads_data_hash",
        "seed",
        "seed_policy_id",
        "retry_policy_id",
        "primitive_version",
        "concurrent_writer_mode",
    ] {
        assert!(
            run_identity.contains(required),
            "run_identity missing: {required}"
        );
    }

    let topology = as_set(&document.topology_fields.required_fields);
    for required in [
        "placement_profile_id",
        "hardware_class_id",
        "hardware_signature",
        "cpu_affinity_mask",
        "smt_policy_state",
        "memory_policy",
        "helper_lane_cpu_set",
        "numa_balancing_state",
    ] {
        assert!(
            topology.contains(required),
            "topology field missing: {required}"
        );
    }

    let always = as_set(&document.artifact_lineage.always);
    for required in [
        "trace_id",
        "scenario_id",
        "bead_id",
        "run_id",
        "claim_id",
        "evidence_id",
        "evidence_root",
        "baseline_comparator",
    ] {
        assert!(
            always.contains(required),
            "artifact lineage missing: {required}"
        );
    }
    assert_eq!(
        as_set(&document.artifact_lineage.decision_plane),
        expected(&["budget_id", "decision_id", "policy_id", "slo_id"])
    );
    assert_eq!(
        as_set(&document.artifact_lineage.shadow),
        expected(&["shadow_run_id"])
    );
    assert_eq!(
        as_set(&document.artifact_lineage.artifact),
        expected(&["artifact_graph_id", "provenance_root"])
    );

    let decision_fields = as_set(&document.decision_plane.required_fields);
    for required in [
        "policy_id",
        "decision_id",
        "action",
        "expected_loss",
        "top_evidence_terms",
        "counterfactual_action",
        "regret_delta",
        "fallback_active",
        "fallback_reason",
        "controller_calibration",
        "policy_version",
    ] {
        assert!(
            decision_fields.contains(required),
            "decision-plane field missing: {required}"
        );
    }
    assert_eq!(
        as_set(&document.decision_plane.one_of_fields),
        expected(&["confidence", "posterior"])
    );

    let failure_fields = as_set(&document.failure_context.required_fields);
    for required in [
        "first_failure_summary",
        "first_failure_stage",
        "first_failure_artifact",
        "diagnostic_json_pointer",
        "replay_command",
        "fallback_active",
        "fallback_reason",
    ] {
        assert!(
            failure_fields.contains(required),
            "failure-context field missing: {required}"
        );
    }
}

#[test]
fn mechanical_comparison_rules_are_explicit_and_stable() {
    let document = load_contract();
    let comparison = &document.mechanical_comparison;

    let join = as_set(&comparison.join_key_fields);
    for required in [
        "surface_id",
        "event_family",
        "fixture_id",
        "workload",
        "concurrency",
        "mode_id",
        "build_profile_id",
        "placement_profile_id",
        "hardware_class_id",
        "hardware_signature",
        "source_revision",
        "beads_data_hash",
        "primitive_version",
        "seed",
    ] {
        assert!(join.contains(required), "join key missing: {required}");
    }

    let required_match = as_set(&comparison.required_match_fields);
    for required in [
        "surface_id",
        "event_family",
        "placement_profile_id",
        "hardware_class_id",
        "hardware_signature",
        "source_revision",
        "beads_data_hash",
        "primitive_version",
    ] {
        assert!(
            required_match.contains(required),
            "required match field missing: {required}"
        );
    }

    let ignored = as_set(&comparison.ignored_ephemeral_fields);
    for required in [
        "timestamp",
        "elapsed_ms",
        "log_path",
        "artifact_path",
        "first_failure_json_pointer",
        "diagnostic_json_pointer",
        "minimal_reproduction_json_pointer",
    ] {
        assert!(
            ignored.contains(required),
            "ignored ephemeral field missing: {required}"
        );
    }

    let mismatch = as_set(&comparison.not_comparable_on_mismatch);
    for required in [
        "placement_profile_id",
        "hardware_class_id",
        "hardware_signature",
        "cpu_affinity_mask",
        "smt_policy_state",
        "memory_policy",
        "helper_lane_cpu_set",
        "numa_balancing_state",
        "source_revision",
        "beads_data_hash",
        "primitive_version",
    ] {
        assert!(
            mismatch.contains(required),
            "not_comparable mismatch field missing: {required}"
        );
    }

    assert!(comparison.require_sorted_keys);
    assert_eq!(
        comparison.comparison_status_on_missing_field,
        "observability_gap"
    );
    assert_eq!(
        comparison.comparison_status_on_topology_mismatch,
        "not_comparable"
    );
    assert!(
        as_set(&comparison.numeric_counter_fields).contains("expected_loss")
            && as_set(&comparison.numeric_counter_fields).contains("regret_delta"),
        "numeric comparison fields must include decision-loss metrics"
    );
    assert!(
        as_set(&comparison.id_fields).contains("claim_id")
            && as_set(&comparison.id_fields).contains("shadow_run_id"),
        "id fields must cover lineage and shadow identifiers"
    );
}

#[test]
fn event_family_ledger_covers_required_db300_logging_families() {
    let document = load_contract();
    let by_id = by_family(&document);
    let actual = by_id.keys().copied().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected(&REQUIRED_EVENT_FAMILIES));

    let copy_alloc = by_id
        .get("copy_alloc_churn")
        .expect("copy_alloc_churn family should exist");
    for required in [
        "hot_path",
        "iterations",
        "legacy_parse_record_calls",
        "scratch_parse_record_calls",
        "legacy_owned_payload_materialization_calls",
        "scratch_owned_payload_materialization_calls",
        "scratch_local_payload_copy_calls",
        "payload_capacity",
        "target_capacity",
        "current_capacity",
    ] {
        assert!(
            copy_alloc
                .required_fields
                .iter()
                .any(|field| field == required),
            "copy_alloc_churn must require {required}"
        );
    }
    assert!(
        copy_alloc
            .live_consumers
            .iter()
            .any(|path| path == "scripts/verify_bd_db300_4_3_3_storage_cursor_reset_safety.sh"),
        "copy_alloc_churn must be anchored to the D3.3 proof surface"
    );

    let verification_bundle = by_id
        .get("verification_bundle_summary")
        .expect("verification_bundle_summary should exist");
    for required in [
        "claim_id",
        "evidence_id",
        "evidence_root",
        "artifact_graph_id",
        "provenance_root",
        "baseline_comparator",
        "replay_command",
        "source_revision",
        "beads_data_hash",
    ] {
        assert!(
            verification_bundle
                .required_fields
                .iter()
                .any(|field| field == required),
            "verification_bundle_summary must require {required}"
        );
    }
    assert!(
        !verification_bundle.live_consumers.is_empty()
            && !verification_bundle.future_consumers.is_empty(),
        "verification bundle summary must name both live and future consumers"
    );

    let failure_bundle = by_id
        .get("failure_bundle_summary")
        .expect("failure_bundle_summary should exist");
    for required in [
        "first_failure_summary",
        "first_failure_stage",
        "first_failure_artifact",
        "diagnostic_json_pointer",
        "replay_command",
        "fallback_active",
        "fallback_reason",
    ] {
        assert!(
            failure_bundle
                .required_fields
                .iter()
                .any(|field| field == required),
            "failure_bundle_summary must require {required}"
        );
    }

    let write_tier = by_id
        .get("write_tier_selection")
        .expect("write_tier_selection should exist");
    for required in [
        "policy_id",
        "decision_id",
        "action",
        "expected_loss",
        "counterfactual_action",
        "regret_delta",
        "fallback_active",
        "fallback_reason",
    ] {
        assert!(
            write_tier
                .required_fields
                .iter()
                .any(|field| field == required),
            "write_tier_selection must require {required}"
        );
    }
    assert!(
        write_tier
            .live_consumers
            .iter()
            .any(|path| path == "crates/fsqlite-core/src/policy_controller.rs"),
        "write_tier_selection must be anchored to policy_controller.rs"
    );

    for family in document.event_families {
        assert!(
            !family.purpose.trim().is_empty(),
            "event family {} must describe its purpose",
            family.family_id
        );
        assert!(
            !family.required_fields.is_empty(),
            "event family {} must name required fields",
            family.family_id
        );
        assert!(
            !family.compare_fields.is_empty(),
            "event family {} must name compare fields",
            family.family_id
        );
        assert!(
            !family.live_consumers.is_empty() || !family.future_consumers.is_empty(),
            "event family {} must name live or future consumers",
            family.family_id
        );
    }
}

#[test]
fn surface_contracts_pin_live_surfaces_and_upgrade_gaps() {
    let document = load_contract();
    let by_id = by_surface(&document);
    let actual = by_id.keys().copied().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected(&REQUIRED_SURFACES));

    let d3 = by_id
        .get("d3_storage_cursor_scratch_decode")
        .expect("D3 surface should exist");
    assert_eq!(d3.pillar_id, "D3");
    assert!(
        d3.canonical_event_families
            .iter()
            .any(|family| family == "copy_alloc_churn"),
        "D3 surface must map to copy_alloc_churn"
    );
    for required in [
        "surface_id",
        "claim_id",
        "evidence_id",
        "source_revision",
        "beads_data_hash",
        "primitive_version",
    ] {
        assert!(
            d3.required_upgrade_fields
                .iter()
                .any(|field| field == required),
            "D3 surface must declare upgrade field {required}"
        );
    }

    let g5 = by_id
        .get("g5_shadow_oracle")
        .expect("G5 shadow surface should exist");
    assert_eq!(g5.pillar_id, "G5");
    assert!(
        g5.canonical_event_families
            .iter()
            .any(|family| family == "failure_bundle_summary"),
        "shadow oracle surface must map to failure_bundle_summary"
    );
    for required in [
        "run_id",
        "event_family",
        "claim_id",
        "evidence_id",
        "policy_id",
    ] {
        assert!(
            g5.required_upgrade_fields
                .iter()
                .any(|field| field == required),
            "shadow oracle surface must declare upgrade field {required}"
        );
    }

    let g6 = by_id
        .get("g6_policy_runtime_snapshot")
        .expect("G6 policy runtime surface should exist");
    assert_eq!(g6.pillar_id, "G6");
    assert!(
        g6.canonical_event_families
            .iter()
            .any(|family| family == "write_tier_selection"),
        "policy runtime surface must map to write_tier_selection"
    );
    for required in ["action", "top_evidence_terms", "claim_id", "evidence_id"] {
        assert!(
            g6.required_upgrade_fields
                .iter()
                .any(|field| field == required),
            "policy runtime surface must declare upgrade field {required}"
        );
    }

    let a3 = by_id
        .get("a3_benchmark_artifact_manifest")
        .expect("A3 benchmark manifest surface should exist");
    assert_eq!(a3.pillar_id, "A3");
    assert!(a3.observed_log_fields.is_empty());
    for required in [
        "trace_id",
        "scenario_id",
        "bead_id",
        "surface_id",
        "event_family",
        "hardware_signature",
        "claim_id",
        "evidence_id",
        "primitive_version",
    ] {
        assert!(
            a3.required_upgrade_fields
                .iter()
                .any(|field| field == required),
            "A3 surface must declare upgrade field {required}"
        );
    }

    for surface in document.surface_contracts {
        let full = workspace_root().join(&surface.source_path);
        assert!(
            full.exists(),
            "surface {} points at missing source {}",
            surface.surface_id,
            full.display()
        );
        assert!(
            !surface.source_symbol.trim().is_empty(),
            "surface {} must name a source symbol",
            surface.surface_id
        );
        assert!(
            !surface.artifact_outputs.is_empty(),
            "surface {} must name artifact outputs",
            surface.surface_id
        );
        assert!(
            !surface.canonical_event_families.is_empty(),
            "surface {} must map to at least one canonical event family",
            surface.surface_id
        );
        assert!(
            !surface.observed_log_fields.is_empty() || !surface.observed_artifact_fields.is_empty(),
            "surface {} must document observed live fields",
            surface.surface_id
        );
    }
}

#[test]
fn live_sources_contain_the_observed_fields_named_by_the_contract() {
    let document = load_contract();

    for surface in &document.surface_contracts {
        let source = load_source(&surface.source_path);
        for field in &surface.observed_log_fields {
            assert!(
                source_mentions_field(&source, field),
                "surface {} source {} does not mention observed log field {}",
                surface.surface_id,
                surface.source_path,
                field
            );
        }
        for field in &surface.observed_artifact_fields {
            assert!(
                source_mentions_field(&source, field),
                "surface {} source {} does not mention observed artifact field {}",
                surface.surface_id,
                surface.source_path,
                field
            );
        }
    }
}
