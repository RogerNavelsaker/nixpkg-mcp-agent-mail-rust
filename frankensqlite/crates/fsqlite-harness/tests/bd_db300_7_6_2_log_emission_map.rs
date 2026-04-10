//! Contract tests for db300_log_emission_map.toml (bd-db300.7.6.2).
//!
//! The goal is to pin the exact structured-log emitter families for unit, e2e,
//! perf, failure, and decision-plane entrypoints so future operator suites do
//! not have to reconstruct observability coverage by hand.

#![allow(clippy::struct_field_names)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

const BEAD_ID: &str = "bd-db300.7.6.2";
const CONTRACT_PATH: &str = "db300_log_emission_map.toml";
const STRUCTURED_LOGGING_CONTRACT_PATH: &str = "db300_structured_logging_contract.toml";
const ENTRYPOINT_PATH: &str = "scripts/verify_g6_2_log_emission_map.sh";

const REQUIRED_EMITTER_FAMILIES: [&str; 7] = [
    "decision_plane_policy_snapshot_entrypoints",
    "e2e_mode_matrix_entrypoints",
    "failure_regime_atlas_entrypoints",
    "failure_shadow_oracle_entrypoints",
    "perf_persistent_phase_entrypoints",
    "unit_prepared_invalidation_helpers",
    "unit_publish_visibility_helpers",
];

const REQUIRED_SURFACE_CLASSES: [&str; 5] = ["decision_plane", "e2e", "failure", "perf", "unit"];

const REQUIRED_COVERAGE_LOG_FIELDS: [&str; 15] = [
    "trace_id",
    "scenario_id",
    "bead_id",
    "run_id",
    "emitter_family",
    "entrypoint_name",
    "required_event_family",
    "required_field_count",
    "missing_field_count",
    "unexpected_field_count",
    "artifact_manifest_key",
    "first_failure_summary",
    "first_failure_stage",
    "first_failure_artifact",
    "diagnostic_json_pointer",
];

const REQUIRED_ARTIFACT_LINKAGE_FIELDS: [&str; 3] =
    ["artifact_manifest_key", "bundle_kind", "replay_command"];

#[derive(Debug, Deserialize)]
struct EmissionMapDocument {
    meta: Meta,
    global_defaults: GlobalDefaults,
    coverage_log_fields: RequiredFieldSet,
    artifact_linkage_fields: RequiredFieldSet,
    surface_class_policy: SurfaceClassPolicy,
    #[serde(default, rename = "emitter_family")]
    emitter_families: Vec<EmitterFamily>,
}

#[derive(Debug, Deserialize)]
struct Meta {
    schema_version: String,
    bead_id: String,
    track_id: String,
    generated_at: String,
    contract_owner: String,
    structured_logging_contract_ref: String,
    verification_matrix_contract_ref: String,
    validation_matrix_contract_ref: String,
    artifact_bundle_contract_ref: String,
}

#[derive(Debug, Deserialize)]
struct GlobalDefaults {
    default_operator_manifest: String,
    missing_event_policy: String,
    missing_field_policy: String,
    gap_conversion_rule: String,
    failure_bundle_rule: String,
    concurrent_writer_requirement: String,
}

#[derive(Debug, Deserialize)]
struct RequiredFieldSet {
    required_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SurfaceClassPolicy {
    required_classes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct EmitterFamily {
    #[serde(rename = "emitter_family_id")]
    id: String,
    surface_class: String,
    entrypoint_name: String,
    source_path: String,
    source_symbol: String,
    artifact_manifest_key: String,
    bundle_kind: String,
    replay_command: String,
    mode_scope: Vec<String>,
    mandatory_when: Vec<String>,
    required_event_families: Vec<String>,
    minimum_required_fields: Vec<String>,
    expected_artifacts: Vec<String>,
    negative_path_expectation: String,
    gap_conversion_rule: String,
    notes: String,
}

#[derive(Debug, Deserialize)]
struct StructuredLoggingDocument {
    #[serde(default, rename = "event_family")]
    event_families: Vec<StructuredEventFamily>,
}

#[derive(Debug, Deserialize)]
struct StructuredEventFamily {
    family_id: String,
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .canonicalize()
        .expect("workspace root should canonicalize")
}

fn load_emission_map() -> EmissionMapDocument {
    let path = workspace_root().join(CONTRACT_PATH);
    let content = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "failed to read {} at {}: {error}",
            CONTRACT_PATH,
            path.display()
        )
    });
    toml::from_str::<EmissionMapDocument>(&content).unwrap_or_else(|error| {
        panic!(
            "failed to parse {} at {}: {error}",
            CONTRACT_PATH,
            path.display()
        )
    })
}

fn load_structured_logging_contract() -> StructuredLoggingDocument {
    let path = workspace_root().join(STRUCTURED_LOGGING_CONTRACT_PATH);
    let content = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "failed to read {} at {}: {error}",
            STRUCTURED_LOGGING_CONTRACT_PATH,
            path.display()
        )
    });
    toml::from_str::<StructuredLoggingDocument>(&content).unwrap_or_else(|error| {
        panic!(
            "failed to parse {} at {}: {error}",
            STRUCTURED_LOGGING_CONTRACT_PATH,
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

#[test]
fn meta_and_cross_contract_refs_are_pinned() {
    let document = load_emission_map();

    assert_eq!(document.meta.schema_version, "1.0.0");
    assert_eq!(document.meta.bead_id, BEAD_ID);
    assert_eq!(document.meta.track_id, "bd-db300.7.6");
    assert!(!document.meta.generated_at.trim().is_empty());
    assert!(!document.meta.contract_owner.trim().is_empty());
    assert_eq!(
        document.meta.structured_logging_contract_ref,
        STRUCTURED_LOGGING_CONTRACT_PATH
    );
    assert_eq!(
        document.meta.verification_matrix_contract_ref,
        "db300_verification_matrix.toml"
    );
    assert_eq!(
        document.meta.validation_matrix_contract_ref,
        "db300_validation_matrix.toml"
    );
    assert_eq!(
        document.meta.artifact_bundle_contract_ref,
        "scripts/verify_g6_3_artifact_bundle_shape.sh"
    );
    assert_eq!(
        document.global_defaults.default_operator_manifest,
        ENTRYPOINT_PATH
    );
    assert!(
        document
            .global_defaults
            .missing_event_policy
            .contains("observability_gap"),
        "missing-event policy must fail closed"
    );
    assert!(
        document
            .global_defaults
            .missing_field_policy
            .contains("first-failure diagnostics"),
        "missing-field policy must require diagnostics"
    );
    assert!(
        document
            .global_defaults
            .gap_conversion_rule
            .contains("add or revise a contract row"),
        "gap conversion rule must force tracked contract updates"
    );
    assert!(
        document
            .global_defaults
            .failure_bundle_rule
            .contains("failure_bundle_summary"),
        "failure bundle rule must keep failure_bundle_summary mandatory"
    );
    assert!(
        document
            .global_defaults
            .concurrent_writer_requirement
            .contains("ON by default"),
        "concurrent-writer default must remain explicit"
    );
}

#[test]
fn coverage_log_and_surface_class_contracts_are_exact() {
    let document = load_emission_map();

    assert_eq!(
        as_set(&document.coverage_log_fields.required_fields),
        expected(&REQUIRED_COVERAGE_LOG_FIELDS),
        "coverage log fields must stay exact and deterministic"
    );
    assert_eq!(
        as_set(&document.artifact_linkage_fields.required_fields),
        expected(&REQUIRED_ARTIFACT_LINKAGE_FIELDS),
        "artifact linkage fields must stay exact and deterministic"
    );
    assert_eq!(
        as_set(&document.surface_class_policy.required_classes),
        expected(&REQUIRED_SURFACE_CLASSES),
        "surface class coverage must stay exact"
    );
}

#[test]
fn emitter_family_set_and_surface_coverage_are_exact() {
    let document = load_emission_map();

    let emitter_ids = document
        .emitter_families
        .iter()
        .map(|family| family.id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        emitter_ids,
        expected(&REQUIRED_EMITTER_FAMILIES),
        "emitter family set must stay exact"
    );

    let surface_classes = document
        .emitter_families
        .iter()
        .map(|family| family.surface_class.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        surface_classes,
        expected(&REQUIRED_SURFACE_CLASSES),
        "unit/e2e/perf/failure/decision-plane coverage must remain explicit"
    );
}

#[test]
fn emitter_event_families_match_the_structured_logging_vocabulary() {
    let document = load_emission_map();
    let structured_logging = load_structured_logging_contract();

    let allowed_event_families = structured_logging
        .event_families
        .iter()
        .map(|family| family.family_id.as_str())
        .collect::<BTreeSet<_>>();

    let used_event_families = document
        .emitter_families
        .iter()
        .flat_map(|family| family.required_event_families.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();

    assert_eq!(
        used_event_families, allowed_event_families,
        "the emission map should cover every structured-log event family exactly through named emitters"
    );
}

#[test]
fn every_emitter_has_real_sources_and_complete_required_shape() {
    let document = load_emission_map();
    let workspace_root = workspace_root();
    let coverage_fields = as_set(&document.coverage_log_fields.required_fields);
    let linkage_fields = as_set(&document.artifact_linkage_fields.required_fields);

    for family in &document.emitter_families {
        let source_path = workspace_root.join(&family.source_path);
        assert!(
            source_path.exists(),
            "source path for {} should exist: {}",
            family.id,
            source_path.display()
        );
        assert!(
            !family.entrypoint_name.trim().is_empty(),
            "entrypoint_name must not be blank for {}",
            family.id
        );
        assert!(
            family.entrypoint_name.contains(&family.source_path)
                || family.entrypoint_name.contains(&family.source_symbol),
            "entrypoint_name for {} must point back to its source path or symbol",
            family.id
        );
        assert!(
            !family.source_symbol.trim().is_empty(),
            "source_symbol must not be blank for {}",
            family.id
        );
        assert!(
            !family.artifact_manifest_key.trim().is_empty(),
            "artifact_manifest_key must not be blank for {}",
            family.id
        );
        assert!(
            !family.bundle_kind.trim().is_empty(),
            "bundle_kind must not be blank for {}",
            family.id
        );
        assert!(
            !family.replay_command.trim().is_empty(),
            "replay_command must not be blank for {}",
            family.id
        );
        assert!(
            !family.mode_scope.is_empty(),
            "mode_scope must not be empty for {}",
            family.id
        );
        assert!(
            !family.mandatory_when.is_empty(),
            "mandatory_when must not be empty for {}",
            family.id
        );
        assert!(
            !family.required_event_families.is_empty(),
            "required_event_families must not be empty for {}",
            family.id
        );
        assert!(
            !family.expected_artifacts.is_empty(),
            "expected_artifacts must not be empty for {}",
            family.id
        );
        assert!(
            !family.negative_path_expectation.trim().is_empty(),
            "negative_path_expectation must not be blank for {}",
            family.id
        );
        assert!(
            !family.gap_conversion_rule.trim().is_empty(),
            "gap_conversion_rule must not be blank for {}",
            family.id
        );
        assert!(
            !family.notes.trim().is_empty(),
            "notes must not be blank for {}",
            family.id
        );

        let minimum_fields = as_set(&family.minimum_required_fields);
        assert!(
            coverage_fields.is_subset(&minimum_fields),
            "{} must include every coverage log field",
            family.id
        );
        assert!(
            linkage_fields.is_subset(&minimum_fields),
            "{} must include every artifact linkage field",
            family.id
        );
    }
}

#[test]
fn operator_entrypoint_exists_and_mentions_expected_outputs() {
    let workspace_root = workspace_root();
    let path = workspace_root.join(ENTRYPOINT_PATH);
    let script = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

    assert!(
        script.contains("emission_map_manifest.json"),
        "script should render the manifest artifact"
    );
    assert!(
        script.contains("emission_gap_ledger.json"),
        "script should render the gap ledger artifact"
    );
    assert!(
        script.contains("events.jsonl"),
        "script should render structured coverage events"
    );
    assert!(
        script.contains("summary.md"),
        "script should render a human-readable summary"
    );
    assert!(
        script.contains("SKIP_CONTRACT_TEST"),
        "script should support a fast path for operator review"
    );
}
