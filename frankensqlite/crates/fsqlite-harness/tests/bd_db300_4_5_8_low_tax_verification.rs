//! Contract tests for db300_low_tax_verification_contract.toml (bd-db300.4.5.8).
//!
//! The goal is to keep the c1 low-tax rewrite lane tied to explicit reusable
//! proof surfaces instead of relying on benchmark deltas alone.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

const BEAD_ID: &str = "bd-db300.4.5.8";
const CONTRACT_PATH: &str = "db300_low_tax_verification_contract.toml";
const ENTRYPOINT_PATH: &str = "scripts/verify_d5_8_low_tax_verification.sh";

const EXPECTED_COVERED_BEADS: [&str; 5] = [
    "bd-db300.4.5.1",
    "bd-db300.4.5.2",
    "bd-db300.4.5.3",
    "bd-db300.4.5.4",
    "bd-db300.4.5.5",
];

const EXPECTED_DEFERRED_EXTENSIONS: [&str; 1] = ["bd-db300.4.5.6"];

const EXPECTED_FAMILIES: [&str; 4] = [
    "boundary_duplication_census",
    "prepared_reuse_and_refresh",
    "record_decode_scratch",
    "result_row_register_reuse",
];

#[derive(Debug, Deserialize)]
struct VerificationDocument {
    meta: Meta,
    global_defaults: GlobalDefaults,
    #[serde(default, rename = "covered_bead")]
    covered_beads: Vec<CoveredBead>,
    #[serde(default, rename = "deferred_extension")]
    deferred_extensions: Vec<DeferredExtension>,
    #[serde(default, rename = "test_family")]
    test_families: Vec<TestFamily>,
}

#[derive(Debug, Deserialize)]
struct Meta {
    schema_version: String,
    bead_id: String,
    track_id: String,
    generated_at: String,
    contract_owner: String,
    verification_matrix_contract_ref: String,
    validation_matrix_contract_ref: String,
    provenance_contract_ref: String,
}

#[derive(Debug, Deserialize)]
struct GlobalDefaults {
    default_operator_manifest: String,
    behavior_preservation_rule: String,
    gap_conversion_rule: String,
    optional_specialization_rule: String,
    structured_diagnostics_rule: String,
}

#[derive(Debug, Deserialize)]
struct CoveredBead {
    bead_id: String,
    title: String,
}

#[derive(Debug, Deserialize)]
struct DeferredExtension {
    bead_id: String,
    title: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct TestFamily {
    family_id: String,
    owner_crate: String,
    source_path: String,
    runner: String,
    supports_beads: Vec<String>,
    test_names: Vec<String>,
    diagnostic_keys: Vec<String>,
    behavior_preservation_scope: String,
    proof_note_requirement: String,
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .canonicalize()
        .expect("workspace root should canonicalize")
}

fn load_contract() -> VerificationDocument {
    let path = workspace_root().join(CONTRACT_PATH);
    let content = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "failed to read {} at {}: {error}",
            CONTRACT_PATH,
            path.display()
        )
    });
    toml::from_str::<VerificationDocument>(&content).unwrap_or_else(|error| {
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

fn load_source(path: &str) -> String {
    let full = workspace_root().join(path);
    fs::read_to_string(&full)
        .unwrap_or_else(|error| panic!("failed to read source {}: {error}", full.display()))
}

#[test]
fn meta_and_contract_refs_are_pinned() {
    let document = load_contract();

    assert_eq!(document.meta.schema_version, "1.0.0");
    assert_eq!(document.meta.bead_id, BEAD_ID);
    assert_eq!(document.meta.track_id, "bd-db300.4.5");
    assert!(!document.meta.generated_at.trim().is_empty());
    assert!(!document.meta.contract_owner.trim().is_empty());
    assert_eq!(
        document.meta.verification_matrix_contract_ref,
        "db300_verification_matrix.toml"
    );
    assert_eq!(
        document.meta.validation_matrix_contract_ref,
        "db300_validation_matrix.toml"
    );
    assert_eq!(
        document.meta.provenance_contract_ref,
        "db300_log_emission_map.toml"
    );
    assert_eq!(
        document.global_defaults.default_operator_manifest,
        ENTRYPOINT_PATH
    );
    assert!(
        document
            .global_defaults
            .behavior_preservation_rule
            .contains("benchmark wins alone never satisfy"),
        "behavior-preservation rule must reject benchmark-only justification"
    );
    assert!(
        document
            .global_defaults
            .gap_conversion_rule
            .contains("extend this contract"),
        "gap conversion rule must force contract extension"
    );
    assert!(
        document
            .global_defaults
            .optional_specialization_rule
            .contains("D5.6"),
        "optional specialization rule must keep D5.6 explicit"
    );
    assert!(
        document
            .global_defaults
            .structured_diagnostics_rule
            .contains("prepared reuse"),
        "diagnostics rule must name the low-tax failure buckets"
    );
}

#[test]
fn covered_beads_and_optional_extension_sets_are_exact() {
    let document = load_contract();

    let covered = document
        .covered_beads
        .iter()
        .map(|row| row.bead_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(covered, expected(&EXPECTED_COVERED_BEADS));
    for bead in &document.covered_beads {
        assert!(
            !bead.title.trim().is_empty(),
            "covered bead title must not be blank"
        );
    }

    let deferred = document
        .deferred_extensions
        .iter()
        .map(|row| row.bead_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(deferred, expected(&EXPECTED_DEFERRED_EXTENSIONS));
    for row in &document.deferred_extensions {
        assert!(!row.title.trim().is_empty());
        assert!(!row.reason.trim().is_empty());
    }
}

#[test]
fn family_set_and_supported_bead_coverage_are_exact() {
    let document = load_contract();

    let family_ids = document
        .test_families
        .iter()
        .map(|row| row.family_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(family_ids, expected(&EXPECTED_FAMILIES));

    let supported = document
        .test_families
        .iter()
        .flat_map(|row| row.supports_beads.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();
    assert_eq!(supported, expected(&EXPECTED_COVERED_BEADS));
}

#[test]
fn every_family_has_real_tests_runners_and_proof_notes() {
    let document = load_contract();

    for family in &document.test_families {
        assert!(
            !family.owner_crate.trim().is_empty(),
            "owner_crate must not be blank for {}",
            family.family_id
        );
        assert!(
            !family.runner.trim().is_empty(),
            "runner must not be blank for {}",
            family.family_id
        );
        assert!(
            !family.behavior_preservation_scope.trim().is_empty(),
            "behavior scope must not be blank for {}",
            family.family_id
        );
        assert!(
            !family.proof_note_requirement.trim().is_empty(),
            "proof note requirement must not be blank for {}",
            family.family_id
        );
        assert!(
            !family.diagnostic_keys.is_empty(),
            "diagnostic keys must not be empty for {}",
            family.family_id
        );
        assert!(
            !family.supports_beads.is_empty(),
            "supports_beads must not be empty for {}",
            family.family_id
        );

        let source = load_source(&family.source_path);
        for test_name in &family.test_names {
            assert!(
                source.contains(&format!("fn {test_name}")),
                "{} should contain test {}",
                family.source_path,
                test_name
            );
        }
    }
}

#[test]
fn operator_entrypoint_exists_and_mentions_expected_artifacts() {
    let path = workspace_root().join(ENTRYPOINT_PATH);
    let script = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

    assert!(
        script.contains("low_tax_verification_manifest.json"),
        "script should render a manifest artifact"
    );
    assert!(
        script.contains("low_tax_family_ledger.json"),
        "script should render a family ledger artifact"
    );
    assert!(
        script.contains("events.jsonl"),
        "script should render structured events"
    );
    assert!(
        script.contains("summary.md"),
        "script should render a human-readable summary"
    );
    assert!(
        script.contains("SKIP_TEST_RUN"),
        "script should support a fast path for artifact review"
    );
}
