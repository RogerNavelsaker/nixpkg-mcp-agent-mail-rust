//! Contract tests for db300_regime_atlas_contract.toml (bd-db300.7.5.5).
//!
//! This contract pins the shared regime-atlas vocabulary so activation-state,
//! frontier, fallback, and gap-conversion semantics stay explicit across the
//! benchmark matrix and the three structural pillars.

#![allow(clippy::struct_field_names)]

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::Deserialize;

const BEAD_ID: &str = "bd-db300.7.5.5";
const CONTRACT_PATH: &str = "db300_regime_atlas_contract.toml";

const EXPECTED_AXIS_IDS: [&str; 13] = [
    "concurrency_level",
    "conflict_topology",
    "control_mode_override",
    "durability_shape",
    "engine_mode",
    "hot_page_pressure",
    "key_shape_class",
    "placement_profile",
    "process_thread_split",
    "read_write_mix",
    "topology_class",
    "transaction_shape",
    "workload_family",
];
const EXPECTED_ACTIVATION_IDS: [&str; 5] = [
    "operator_opt_in",
    "regime_gated_default",
    "rejected",
    "shadow_only",
    "universal_default",
];
const EXPECTED_FRONTIER_RULE_IDS: [&str; 5] = [
    "hot_page_conflict_breakpoint",
    "shadow_oracle_clearance",
    "tail_latency_guardrail",
    "throughput_break_even",
    "topology_stability",
];
const EXPECTED_GAP_RULE_IDS: [&str; 4] = [
    "missing_matrix_cell",
    "missing_structured_logs",
    "unknown_topology_class",
    "unstable_breakpoint",
];
const EXPECTED_SCRIPT_IDS: [&str; 4] = [
    "d1_parallel_wal_suite",
    "e2_fused_entry_suite",
    "e3_metadata_publication_suite",
    "g5_5_regime_atlas_suite",
];
const EXPECTED_SURFACE_IDS: [&str; 3] = [
    "d1_parallel_wal",
    "e2_fused_entry",
    "e3_metadata_publication",
];

#[derive(Debug, Deserialize)]
struct RegimeAtlasContractDocument {
    meta: Meta,
    global_defaults: GlobalDefaults,
    logging: LoggingContract,
    #[serde(default, rename = "regime_axis")]
    regime_axes: Vec<RegimeAxis>,
    #[serde(default, rename = "activation_state")]
    activation_states: Vec<ActivationState>,
    #[serde(default, rename = "frontier_rule")]
    frontier_rules: Vec<FrontierRule>,
    #[serde(default, rename = "gap_conversion_rule")]
    gap_conversion_rules: Vec<GapConversionRule>,
    #[serde(default, rename = "named_script")]
    named_scripts: Vec<NamedScript>,
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
}

#[derive(Debug, Deserialize)]
struct GlobalDefaults {
    default_activation_state: String,
    unclassified_regime_action: String,
    hostile_regime_action: String,
    concurrent_writer_requirement: String,
    evidence_promotion_rule: String,
    baseline_comparator_rule: String,
    gap_conversion_rule: String,
    safe_mode_policy: String,
}

#[derive(Debug, Deserialize)]
struct LoggingContract {
    common_fields: Vec<String>,
    first_failure_fields: Vec<String>,
    rendered_artifacts: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RegimeAxis {
    axis_id: String,
    description: String,
    examples: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ActivationState {
    state_id: String,
    promotion_rule: String,
    fallback_behavior: String,
    operator_story: String,
}

#[derive(Debug, Deserialize)]
struct FrontierRule {
    rule_id: String,
    target_metric: String,
    decision_rule: String,
    stability_requirement: String,
    fallback_on_failure: String,
}

#[derive(Debug, Deserialize)]
struct GapConversionRule {
    gap_id: String,
    trigger: String,
    required_action: String,
}

#[derive(Debug, Deserialize)]
struct NamedScript {
    script_id: String,
    path: String,
    surfaces: Vec<String>,
    artifact_outputs: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SurfaceContract {
    surface_id: String,
    pillar_id: String,
    title: String,
    baseline_comparator: String,
    candidate_lever: String,
    regime_axes: Vec<String>,
    supported_activation_states: Vec<String>,
    frontier_rules: Vec<String>,
    named_scripts: Vec<String>,
    required_log_fields: Vec<String>,
    deterministic_fallback: String,
    operator_story: String,
    unit_test_obligations: Vec<String>,
    e2e_obligations: Vec<String>,
}

fn load_contract() -> RegimeAtlasContractDocument {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .join(CONTRACT_PATH);
    let content = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "failed to read {} at {}: {error}",
            CONTRACT_PATH,
            path.display()
        )
    });
    toml::from_str::<RegimeAtlasContractDocument>(&content).unwrap_or_else(|error| {
        panic!(
            "failed to parse {} at {}: {error}",
            CONTRACT_PATH,
            path.display()
        )
    })
}

fn expected<'a>(values: &'a [&'a str]) -> BTreeSet<&'a str> {
    values.iter().copied().collect::<BTreeSet<_>>()
}

#[test]
fn manifest_meta_and_fail_closed_defaults_are_pinned() {
    let document = load_contract();
    assert_eq!(document.meta.schema_version, "1.0.0");
    assert_eq!(document.meta.bead_id, BEAD_ID);
    assert_eq!(document.meta.track_id, "bd-db300.7.5");
    assert!(!document.meta.generated_at.trim().is_empty());
    assert!(!document.meta.contract_owner.trim().is_empty());

    assert_eq!(
        document.global_defaults.default_activation_state,
        "shadow_only"
    );
    assert_eq!(
        document.global_defaults.unclassified_regime_action,
        "fallback_to_conservative"
    );
    assert_eq!(
        document.global_defaults.hostile_regime_action,
        "fallback_to_conservative"
    );
    assert!(
        document
            .global_defaults
            .concurrent_writer_requirement
            .contains("ON by default"),
        "contract must explicitly preserve concurrent-writer defaults"
    );
    assert!(
        document
            .global_defaults
            .evidence_promotion_rule
            .contains("shadow-oracle"),
        "promotion rule must name shadow-oracle evidence"
    );
    assert!(
        document
            .global_defaults
            .baseline_comparator_rule
            .contains("baseline_comparator")
            && document
                .global_defaults
                .baseline_comparator_rule
                .contains("breakpoint_metric"),
        "baseline comparator rule must pin both comparator and breakpoint evidence"
    );
    assert!(
        document
            .global_defaults
            .gap_conversion_rule
            .contains("tracked work"),
        "gap conversion must create tracked work instead of guessing"
    );
    assert!(
        document
            .global_defaults
            .safe_mode_policy
            .contains("deterministic conservative"),
        "safe mode must fail closed"
    );
}

#[test]
fn axis_activation_frontier_and_script_vocabularies_are_exact() {
    let document = load_contract();

    let axis_ids = document
        .regime_axes
        .iter()
        .map(|axis| axis.axis_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(axis_ids, expected(&EXPECTED_AXIS_IDS));
    for axis in &document.regime_axes {
        assert!(
            !axis.description.trim().is_empty(),
            "axis {} must have a description",
            axis.axis_id
        );
        assert!(
            !axis.examples.is_empty(),
            "axis {} must provide example values",
            axis.axis_id
        );
    }

    let activation_ids = document
        .activation_states
        .iter()
        .map(|state| state.state_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(activation_ids, expected(&EXPECTED_ACTIVATION_IDS));
    for state in &document.activation_states {
        assert!(
            !state.promotion_rule.trim().is_empty()
                && !state.fallback_behavior.trim().is_empty()
                && !state.operator_story.trim().is_empty(),
            "activation state {} must be fully specified",
            state.state_id
        );
    }

    let frontier_rule_ids = document
        .frontier_rules
        .iter()
        .map(|rule| rule.rule_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(frontier_rule_ids, expected(&EXPECTED_FRONTIER_RULE_IDS));
    for rule in &document.frontier_rules {
        assert!(
            !rule.target_metric.trim().is_empty()
                && !rule.decision_rule.trim().is_empty()
                && !rule.stability_requirement.trim().is_empty()
                && !rule.fallback_on_failure.trim().is_empty(),
            "frontier rule {} must be fully specified",
            rule.rule_id
        );
    }

    let gap_rule_ids = document
        .gap_conversion_rules
        .iter()
        .map(|rule| rule.gap_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(gap_rule_ids, expected(&EXPECTED_GAP_RULE_IDS));
    for rule in &document.gap_conversion_rules {
        assert!(
            !rule.trigger.trim().is_empty() && !rule.required_action.trim().is_empty(),
            "gap conversion rule {} must be fully specified",
            rule.gap_id
        );
    }

    let script_ids = document
        .named_scripts
        .iter()
        .map(|script| script.script_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(script_ids, expected(&EXPECTED_SCRIPT_IDS));
    for script in &document.named_scripts {
        assert!(
            script.path.starts_with("scripts/verify_"),
            "script {} must use named verify entrypoint conventions",
            script.script_id
        );
        assert!(
            !script.surfaces.is_empty() && !script.artifact_outputs.is_empty(),
            "script {} must name surfaces and artifact outputs",
            script.script_id
        );
    }
}

#[test]
fn logging_and_surface_contracts_pin_the_three_pillars() {
    let document = load_contract();

    let common_fields = document
        .logging
        .common_fields
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for required in [
        "trace_id",
        "scenario_id",
        "bead_id",
        "surface_id",
        "pillar_id",
        "regime_id",
        "activation_state",
        "frontier_reason",
        "breakpoint_metric",
        "placement_profile",
        "topology_class",
        "fallback_state",
        "baseline_comparator",
    ] {
        assert!(
            common_fields.contains(required),
            "logging contract must include {}",
            required
        );
    }

    let first_failure_fields = document
        .logging
        .first_failure_fields
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for required in [
        "first_failure_summary",
        "first_failure_stage",
        "first_failure_artifact",
        "replay_command",
        "diagnostic_json_pointer",
    ] {
        assert!(
            first_failure_fields.contains(required),
            "logging contract must include failure field {}",
            required
        );
    }
    let rendered_artifacts = document
        .logging
        .rendered_artifacts
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for required in [
        "regime_atlas_manifest.json",
        "activation_frontiers.json",
        "structured_logs.ndjson",
        "summary.md",
    ] {
        assert!(
            rendered_artifacts.contains(required),
            "rendered artifact list must include {}",
            required
        );
    }

    let surface_ids = document
        .surface_contracts
        .iter()
        .map(|surface| surface.surface_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(surface_ids, expected(&EXPECTED_SURFACE_IDS));

    let axis_ids = document
        .regime_axes
        .iter()
        .map(|axis| axis.axis_id.as_str())
        .collect::<BTreeSet<_>>();
    let activation_ids = document
        .activation_states
        .iter()
        .map(|state| state.state_id.as_str())
        .collect::<BTreeSet<_>>();
    let frontier_rule_ids = document
        .frontier_rules
        .iter()
        .map(|rule| rule.rule_id.as_str())
        .collect::<BTreeSet<_>>();
    let script_ids = document
        .named_scripts
        .iter()
        .map(|script| script.script_id.as_str())
        .collect::<BTreeSet<_>>();

    for surface in &document.surface_contracts {
        assert!(
            matches!(surface.pillar_id.as_str(), "E2" | "E3" | "D1"),
            "surface {} must map to one of the three structural pillars",
            surface.surface_id
        );
        assert!(
            !surface.title.trim().is_empty()
                && !surface.baseline_comparator.trim().is_empty()
                && !surface.candidate_lever.trim().is_empty()
                && !surface.deterministic_fallback.trim().is_empty()
                && !surface.operator_story.trim().is_empty(),
            "surface {} must fully specify activation and fallback semantics",
            surface.surface_id
        );
        assert!(
            !surface.unit_test_obligations.is_empty() && !surface.e2e_obligations.is_empty(),
            "surface {} must name both unit and e2e proof obligations",
            surface.surface_id
        );
        assert!(
            !surface.required_log_fields.is_empty(),
            "surface {} must pin required log fields",
            surface.surface_id
        );
        for axis in &surface.regime_axes {
            assert!(
                axis_ids.contains(axis.as_str()),
                "surface {} references unknown axis {}",
                surface.surface_id,
                axis
            );
        }
        for state in &surface.supported_activation_states {
            assert!(
                activation_ids.contains(state.as_str()),
                "surface {} references unknown activation state {}",
                surface.surface_id,
                state
            );
        }
        for rule in &surface.frontier_rules {
            assert!(
                frontier_rule_ids.contains(rule.as_str()),
                "surface {} references unknown frontier rule {}",
                surface.surface_id,
                rule
            );
        }
        for script in &surface.named_scripts {
            assert!(
                script_ids.contains(script.as_str()),
                "surface {} references unknown script {}",
                surface.surface_id,
                script
            );
        }
        for field in &surface.required_log_fields {
            assert!(
                common_fields.contains(field.as_str()),
                "surface {} requires unknown log field {}",
                surface.surface_id,
                field
            );
        }
    }
}
