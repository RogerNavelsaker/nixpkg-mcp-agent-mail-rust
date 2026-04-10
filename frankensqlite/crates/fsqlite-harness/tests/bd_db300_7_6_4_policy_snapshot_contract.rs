//! Contract tests for db300_policy_snapshot_contract.toml (bd-db300.7.6.4).
//!
//! The goal is to pin the shared controller-artifact and runtime-snapshot
//! schema so downstream beads can consume decision records, operator packages,
//! and fallback semantics without inventing ad hoc metadata.

#![allow(clippy::struct_field_names)]

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use fsqlite_core::repair_symbols::policy_controller::{
    AutoTunePragmaConfig, CandidateAction, PolicyController, PolicyKnob, PolicySignals,
};
use serde::Deserialize;
use serde_json::Value;

const BEAD_ID: &str = "bd-db300.7.6.4";
const CONTRACT_PATH: &str = "db300_policy_snapshot_contract.toml";

const EXPECTED_ROLLOUT_STAGES: [&str; 5] = ["canary", "default", "fallback_only", "ramp", "shadow"];
const EXPECTED_ACTIVATION_STATES: [&str; 5] = [
    "operator_opt_in",
    "regime_gated_default",
    "rejected",
    "shadow_only",
    "universal_default",
];
const EXPECTED_CONTROL_MODES: [&str; 3] = [
    "conservative_baseline",
    "expected_loss_guarded_argmin",
    "shadow_compare",
];
const EXPECTED_KILL_SWITCH_STATES: [&str; 3] = ["armed", "disarmed", "tripped"];
const EXPECTED_DIVERGENCE_CLASSES: [&str; 6] = [
    "decision_budget_exceeded",
    "fallback_contract_breach",
    "observability_gap",
    "policy_version_mismatch",
    "provenance_mismatch",
    "stale_snapshot_schema",
];

#[derive(Debug, Deserialize)]
struct PolicySnapshotContractDocument {
    meta: Meta,
    global_defaults: GlobalDefaults,
    logging: Logging,
    artifact_contract: FieldContract,
    runtime_snapshot: FieldContract,
    counterexample_bundle: FieldContract,
    #[serde(default, rename = "rollout_stage")]
    rollout_stages: Vec<EnumeratedValue>,
    #[serde(default, rename = "activation_state")]
    activation_states: Vec<EnumeratedValue>,
    #[serde(default, rename = "control_mode")]
    control_modes: Vec<EnumeratedValue>,
    #[serde(default, rename = "kill_switch_state")]
    kill_switch_states: Vec<EnumeratedValue>,
    #[serde(default, rename = "divergence_class")]
    divergence_classes: Vec<EnumeratedValue>,
    #[serde(default, rename = "named_script")]
    named_scripts: Vec<NamedScript>,
    #[serde(default, rename = "consumer_surface")]
    consumer_surfaces: Vec<ConsumerSurface>,
}

#[derive(Debug, Deserialize)]
struct Meta {
    schema_version: String,
    bead_id: String,
    track_id: String,
    generated_at: String,
    contract_owner: String,
    shadow_oracle_contract_ref: String,
    regime_atlas_contract_ref: String,
}

#[derive(Debug, Deserialize)]
struct GlobalDefaults {
    default_rollout_stage: String,
    default_activation_state: String,
    default_control_mode: String,
    concurrent_writer_requirement: String,
    fallback_policy: String,
    negative_path_policy: String,
    counterexample_policy: String,
    promotion_rule: String,
}

#[derive(Debug, Deserialize)]
struct Logging {
    required_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FieldContract {
    schema_id: Option<String>,
    bundle_schema: Option<String>,
    required_fields: Vec<String>,
    #[serde(default)]
    optional_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct EnumeratedValue {
    #[serde(
        alias = "stage_id",
        alias = "state_id",
        alias = "mode_id",
        alias = "divergence_class_id"
    )]
    id: String,
    description: String,
}

#[derive(Debug, Deserialize)]
struct NamedScript {
    script_id: String,
    path: String,
    surfaces: Vec<String>,
    artifact_outputs: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ConsumerSurface {
    surface_id: String,
    required_fields: Vec<String>,
}

fn load_contract() -> PolicySnapshotContractDocument {
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
    toml::from_str::<PolicySnapshotContractDocument>(&content).unwrap_or_else(|error| {
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
fn manifest_meta_and_cross_contract_refs_are_pinned() {
    let document = load_contract();
    assert_eq!(document.meta.schema_version, "1.0.0");
    assert_eq!(document.meta.bead_id, BEAD_ID);
    assert_eq!(document.meta.track_id, "bd-db300.7.6");
    assert!(!document.meta.generated_at.trim().is_empty());
    assert!(!document.meta.contract_owner.trim().is_empty());
    assert_eq!(
        document.meta.shadow_oracle_contract_ref,
        "db300_shadow_oracle_contract.toml"
    );
    assert_eq!(
        document.meta.regime_atlas_contract_ref,
        "db300_regime_atlas_contract.toml"
    );
    assert_eq!(document.global_defaults.default_rollout_stage, "default");
    assert_eq!(
        document.global_defaults.default_activation_state,
        "regime_gated_default"
    );
    assert_eq!(
        document.global_defaults.default_control_mode,
        "expected_loss_guarded_argmin"
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
            .counterexample_policy
            .contains("counterexample bundle"),
        "counterexample capture must stay explicit"
    );
    assert!(
        document
            .global_defaults
            .fallback_policy
            .contains("conservative baseline"),
        "fallback policy must name the conservative baseline"
    );
    assert!(
        document
            .global_defaults
            .negative_path_policy
            .contains("stale_snapshot_schema"),
        "negative path policy must include stale schema handling"
    );
    assert!(
        document
            .global_defaults
            .promotion_rule
            .contains("shadow -> canary -> ramp -> default"),
        "promotion rule must pin progressive rollout semantics"
    );
}

#[test]
fn artifact_runtime_and_logging_field_sets_cover_required_contract() {
    let document = load_contract();
    assert_eq!(
        document.artifact_contract.schema_id.as_deref(),
        Some("fsqlite.policy_artifact_contract.v1")
    );
    assert_eq!(
        document.runtime_snapshot.schema_id.as_deref(),
        Some("fsqlite.policy_runtime_snapshot.v1")
    );
    assert_eq!(
        document.counterexample_bundle.bundle_schema.as_deref(),
        Some("fsqlite.policy_controller.counterexample.v1")
    );
    assert!(
        document
            .artifact_contract
            .required_fields
            .contains(&"shadow_contract_ref".to_owned())
    );
    assert!(
        document
            .runtime_snapshot
            .required_fields
            .contains(&"divergence_class".to_owned())
    );
    assert!(
        document
            .runtime_snapshot
            .required_fields
            .contains(&"counterexample_bundle".to_owned())
    );
    assert!(
        document
            .counterexample_bundle
            .required_fields
            .contains(&"kill_switch_state".to_owned())
    );
    assert!(
        document
            .logging
            .required_fields
            .contains(&"first_failure_diagnostics".to_owned())
    );
    assert!(
        document
            .runtime_snapshot
            .optional_fields
            .contains(&"safety_certificate_id".to_owned())
    );
}

#[test]
fn enumerations_named_script_and_consumer_surfaces_are_explicit() {
    let document = load_contract();
    let rollout_stages = document
        .rollout_stages
        .iter()
        .map(|stage| stage.id.as_str())
        .collect::<BTreeSet<_>>();
    let activation_states = document
        .activation_states
        .iter()
        .map(|state| state.id.as_str())
        .collect::<BTreeSet<_>>();
    let control_modes = document
        .control_modes
        .iter()
        .map(|mode| mode.id.as_str())
        .collect::<BTreeSet<_>>();
    let kill_switch_states = document
        .kill_switch_states
        .iter()
        .map(|state| state.id.as_str())
        .collect::<BTreeSet<_>>();
    let divergence_classes = document
        .divergence_classes
        .iter()
        .map(|class| class.id.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(rollout_stages, expected(&EXPECTED_ROLLOUT_STAGES));
    assert_eq!(activation_states, expected(&EXPECTED_ACTIVATION_STATES));
    assert_eq!(control_modes, expected(&EXPECTED_CONTROL_MODES));
    assert_eq!(kill_switch_states, expected(&EXPECTED_KILL_SWITCH_STATES));
    assert_eq!(divergence_classes, expected(&EXPECTED_DIVERGENCE_CLASSES));
    assert!(
        document
            .rollout_stages
            .iter()
            .all(|stage| !stage.description.trim().is_empty())
    );

    let script = document
        .named_scripts
        .iter()
        .find(|script| script.script_id == "g6_4_policy_snapshot_contract")
        .expect("named validation entrypoint must exist");
    assert_eq!(
        script.path,
        "scripts/verify_bd_db300_7_6_4_policy_snapshot_contract.sh"
    );
    assert!(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../")
            .join(&script.path)
            .exists()
    );
    assert!(
        script
            .surfaces
            .iter()
            .any(|surface| surface == "g7_verify_suite_package")
    );
    assert!(
        script
            .artifact_outputs
            .iter()
            .any(|artifact| artifact.ends_with("suite_package.json"))
    );

    let operator_surface = document
        .consumer_surfaces
        .iter()
        .find(|surface| surface.surface_id == "g7_verify_suite_package")
        .expect("g7 operator package surface must exist");
    for field in [
        "shadow_mode",
        "shadow_verdict",
        "kill_switch_state",
        "divergence_class",
        "counterexample_bundle",
    ] {
        assert!(
            operator_surface
                .required_fields
                .iter()
                .any(|candidate| candidate == field),
            "g7 operator surface must require {field}"
        );
    }
}

#[test]
fn policy_controller_snapshot_serializes_contract_required_fields() {
    let document = load_contract();
    let mut controller = PolicyController::new(AutoTunePragmaConfig::default(), 32, 2, 16);
    let _ = controller.evaluate_knob(
        PolicyKnob::BgCpuMax,
        controller.effective_limits().bg_cpu_max,
        &[CandidateAction::new(
            1,
            controller.effective_limits().bg_cpu_max + 1,
            0.0,
            "ignored_without_telemetry",
        )],
        PolicySignals {
            symbol_loss_rejects_h0: false,
            bocpd_regime_shift: false,
            regime_id: 11,
        },
        false,
        10,
    );

    let entry = controller
        .ledger()
        .latest()
        .expect("telemetry-gap fallback must record a decision");
    let artifact_json =
        serde_json::to_value(&entry.artifact_contract).expect("artifact contract should serialize");
    let snapshot_json =
        serde_json::to_value(&entry.runtime_snapshot).expect("runtime snapshot should serialize");

    for field in &document.artifact_contract.required_fields {
        assert!(
            artifact_json.get(field).is_some(),
            "artifact contract missing {field}"
        );
    }
    for field in &document.runtime_snapshot.required_fields {
        assert!(
            snapshot_json.get(field).is_some(),
            "runtime snapshot missing {field}"
        );
    }

    assert_eq!(
        snapshot_json
            .get("kill_switch_state")
            .and_then(Value::as_str),
        Some("armed")
    );
    assert_eq!(
        snapshot_json
            .get("divergence_class")
            .and_then(Value::as_str),
        Some("observability_gap")
    );
    assert_eq!(
        snapshot_json
            .get("activation_state")
            .and_then(Value::as_str),
        Some("shadow_only")
    );
    assert!(
        snapshot_json
            .get("counterexample_bundle")
            .and_then(Value::as_str)
            .is_some(),
        "observability gaps must reference a counterexample bundle path"
    );
}
