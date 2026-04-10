//! Contract tests for db300_validation_matrix.toml (bd-db300.7.5.2).
//!
//! The goal is to keep the overlay implementation beads tied to explicit
//! crash/fault/interference/e2e obligations so another agent can tell exactly
//! which reruns are mandatory after a code change lands.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::Deserialize;

const BEAD_ID: &str = "bd-db300.7.5.2";
const CONTRACT_PATH: &str = "db300_validation_matrix.toml";
const ENTRYPOINT_PATH: &str = "scripts/verify_g5_2_validation_matrix.sh";

const INCLUDED_BEADS: [&str; 12] = [
    "bd-db300.3.8.1",
    "bd-db300.3.8.2",
    "bd-db300.3.8.4",
    "bd-db300.3.8.5",
    "bd-db300.3.8.6",
    "bd-db300.3.8.7",
    "bd-db300.4.5.1",
    "bd-db300.4.5.2",
    "bd-db300.4.5.3",
    "bd-db300.4.5.4",
    "bd-db300.4.5.5",
    "bd-db300.4.5.6",
];

const EXCLUDED_BEADS: [&str; 23] = [
    "bd-db300.1.7.1",
    "bd-db300.1.7.2",
    "bd-db300.1.7.3",
    "bd-db300.1.7.4",
    "bd-db300.3.8.3",
    "bd-db300.3.8.8",
    "bd-db300.3.8.9",
    "bd-db300.3.8.10",
    "bd-db300.3.8.11",
    "bd-db300.3.8.12",
    "bd-db300.4.5.7",
    "bd-db300.4.5.8",
    "bd-db300.4.5.9",
    "bd-db300.5.8.1",
    "bd-db300.5.8.2",
    "bd-db300.5.8.3",
    "bd-db300.5.8.4",
    "bd-db300.5.8.5",
    "bd-db300.7.9.1",
    "bd-db300.7.9.2",
    "bd-db300.7.9.3",
    "bd-db300.7.9.4",
    "bd-db300.7.9.5",
];

const EXPECTED_VALIDATION_CLASSES: [&str; 5] = [
    "canonical_matrix_e2e",
    "negative_path_proof",
    "persistent_phase_e2e",
    "topology_interference",
    "crash_fault_recovery",
];

#[derive(Debug, Deserialize)]
struct ValidationMatrixDocument {
    meta: MatrixMeta,
    global_defaults: GlobalDefaults,
    #[serde(default, rename = "validation_class")]
    validation_classes: Vec<ValidationClass>,
    #[serde(default, rename = "obligation_row")]
    obligation_rows: Vec<ObligationRow>,
    #[serde(default, rename = "excluded_bead")]
    excluded_beads: Vec<ExcludedBead>,
}

#[derive(Debug, Deserialize)]
struct MatrixMeta {
    schema_version: String,
    bead_id: String,
    track_id: String,
    generated_at: String,
    contract_owner: String,
}

#[derive(Debug, Deserialize)]
struct GlobalDefaults {
    default_seed_policy: String,
    default_stop_on_failure: bool,
    default_operator_manifest: String,
    tracked_exception_rule: String,
}

#[derive(Debug, Deserialize)]
struct ValidationClass {
    id: String,
    description: String,
    applicability_rule: String,
}

#[derive(Debug, Deserialize)]
struct ObligationRow {
    row_id: String,
    bead_id: String,
    title: String,
    validation_owner: String,
    validation_class: String,
    entrypoint: String,
    placement_profile: String,
    workload_row: String,
    failure_mode: String,
    expected_artifacts: Vec<String>,
    log_family: String,
    seed_policy: String,
    stop_on_failure: bool,
    negative_expectation: String,
    notes: String,
}

#[derive(Debug, Deserialize)]
struct ExcludedBead {
    bead_id: String,
    title: String,
    reason: String,
    gap_rule: String,
}

fn load_contract() -> ValidationMatrixDocument {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(CONTRACT_PATH);
    let source = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "failed to read {} at {}: {error}",
            CONTRACT_PATH,
            path.display()
        )
    });
    toml::from_str(&source).unwrap_or_else(|error| {
        panic!(
            "failed to parse {} at {}: {error}",
            CONTRACT_PATH,
            path.display()
        )
    })
}

fn expected_included_beads() -> BTreeSet<&'static str> {
    INCLUDED_BEADS.into_iter().collect()
}

fn expected_excluded_beads() -> BTreeSet<&'static str> {
    EXCLUDED_BEADS.into_iter().collect()
}

fn is_commit_path_bead(bead_id: &str) -> bool {
    bead_id.starts_with("bd-db300.3.8.")
}

fn is_low_tax_bead(bead_id: &str) -> bool {
    bead_id.starts_with("bd-db300.4.5.")
}

#[test]
fn manifest_meta_is_pinned_to_g5_2_contract() {
    let document = load_contract();
    assert_eq!(document.meta.schema_version, "1.0.0");
    assert_eq!(document.meta.bead_id, BEAD_ID);
    assert_eq!(document.meta.track_id, "bd-db300.7.5");
    assert!(
        !document.meta.generated_at.trim().is_empty(),
        "generated_at must not be blank"
    );
    assert!(
        !document.meta.contract_owner.trim().is_empty(),
        "contract_owner must not be blank"
    );
    assert_eq!(
        document.global_defaults.default_operator_manifest,
        ENTRYPOINT_PATH
    );
    assert!(
        document.global_defaults.default_stop_on_failure,
        "default stop_on_failure should remain fail-closed"
    );
    assert!(
        !document
            .global_defaults
            .default_seed_policy
            .trim()
            .is_empty(),
        "default seed policy must be named"
    );
    assert!(
        !document
            .global_defaults
            .tracked_exception_rule
            .trim()
            .is_empty(),
        "tracked_exception_rule must not be blank"
    );
}

#[test]
fn included_and_excluded_sets_cover_the_overlay_leaf_surface_exactly_once() {
    let document = load_contract();

    let included = document
        .obligation_rows
        .iter()
        .map(|row| row.bead_id.as_str())
        .collect::<BTreeSet<_>>();
    let excluded = document
        .excluded_beads
        .iter()
        .map(|row| row.bead_id.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        included,
        expected_included_beads(),
        "included code-changing bead set must stay exact and deterministic"
    );
    assert_eq!(
        excluded,
        expected_excluded_beads(),
        "excluded non-code-changing bead set must stay exact and deterministic"
    );
    assert!(
        included.is_disjoint(&excluded),
        "included and excluded bead sets must not overlap"
    );

    let combined = included.union(&excluded).copied().collect::<BTreeSet<_>>();
    let expected_all = expected_included_beads()
        .union(&expected_excluded_beads())
        .copied()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        combined, expected_all,
        "every overlay leaf bead must be either covered or explicitly excluded"
    );
}

#[test]
fn validation_classes_and_rows_have_required_shape() {
    let document = load_contract();
    let class_ids = document
        .validation_classes
        .iter()
        .map(|class| class.id.as_str())
        .collect::<BTreeSet<_>>();
    let expected_class_ids = EXPECTED_VALIDATION_CLASSES
        .into_iter()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        class_ids, expected_class_ids,
        "validation classes are part of the contract and must stay exact"
    );

    for class in &document.validation_classes {
        assert!(
            !class.description.trim().is_empty(),
            "blank description for validation class {}",
            class.id
        );
        assert!(
            !class.applicability_rule.trim().is_empty(),
            "blank applicability_rule for validation class {}",
            class.id
        );
    }

    for row in &document.obligation_rows {
        assert!(
            !row.row_id.trim().is_empty(),
            "blank row_id for {}",
            row.bead_id
        );
        assert!(
            !row.title.trim().is_empty(),
            "blank title for {}",
            row.row_id
        );
        assert!(
            !row.validation_owner.trim().is_empty(),
            "blank validation_owner for {}",
            row.row_id
        );
        assert!(
            expected_class_ids.contains(row.validation_class.as_str()),
            "unknown validation_class for {}: {}",
            row.row_id,
            row.validation_class
        );
        assert!(
            row.entrypoint.starts_with("scripts/")
                || row.entrypoint.starts_with("cargo ")
                || row.entrypoint.starts_with("cargo test "),
            "entrypoint must be a script or explicit cargo command surface: {} => {}",
            row.row_id,
            row.entrypoint
        );
        assert!(
            !row.placement_profile.trim().is_empty(),
            "blank placement_profile for {}",
            row.row_id
        );
        assert!(
            !row.workload_row.trim().is_empty(),
            "blank workload_row for {}",
            row.row_id
        );
        assert!(
            !row.failure_mode.trim().is_empty(),
            "blank failure_mode for {}",
            row.row_id
        );
        assert!(
            !row.expected_artifacts.is_empty(),
            "missing expected_artifacts for {}",
            row.row_id
        );
        for artifact in &row.expected_artifacts {
            assert!(
                !artifact.trim().is_empty(),
                "blank expected artifact in {}",
                row.row_id
            );
        }
        assert!(
            !row.log_family.trim().is_empty(),
            "blank log_family for {}",
            row.row_id
        );
        assert!(
            !row.seed_policy.trim().is_empty(),
            "blank seed_policy for {}",
            row.row_id
        );
        assert!(
            row.stop_on_failure,
            "all obligation rows should remain fail-closed for {}",
            row.row_id
        );
        assert!(
            !row.negative_expectation.trim().is_empty(),
            "negative expectation required for {}",
            row.row_id
        );
        assert!(
            !row.notes.trim().is_empty(),
            "notes must explain why this obligation exists for {}",
            row.row_id
        );
    }

    for excluded in &document.excluded_beads {
        assert!(
            !excluded.title.trim().is_empty(),
            "blank title for excluded bead {}",
            excluded.bead_id
        );
        assert!(
            !excluded.reason.trim().is_empty(),
            "blank reason for excluded bead {}",
            excluded.bead_id
        );
        assert!(
            !excluded.gap_rule.trim().is_empty(),
            "blank gap_rule for excluded bead {}",
            excluded.bead_id
        );
    }
}

#[test]
fn included_beads_have_the_required_obligation_mix() {
    let document = load_contract();
    let mut rows_by_bead: BTreeMap<&str, Vec<&ObligationRow>> = BTreeMap::new();
    for row in &document.obligation_rows {
        rows_by_bead
            .entry(row.bead_id.as_str())
            .or_default()
            .push(row);
    }

    for bead_id in expected_included_beads() {
        let rows = rows_by_bead
            .get(bead_id)
            .unwrap_or_else(|| panic!("missing obligation rows for {bead_id}"));
        let classes = rows
            .iter()
            .map(|row| row.validation_class.as_str())
            .collect::<BTreeSet<_>>();

        if is_low_tax_bead(bead_id) {
            assert!(
                classes.contains("canonical_matrix_e2e"),
                "{bead_id} must carry canonical matrix reruns"
            );
            assert!(
                classes.contains("negative_path_proof"),
                "{bead_id} must carry a narrow negative-path proof surface"
            );
            assert_eq!(
                classes.len(),
                2,
                "{bead_id} should stay on the two-surface D-track validation shape"
            );
        } else if is_commit_path_bead(bead_id) {
            assert!(
                classes.contains("persistent_phase_e2e"),
                "{bead_id} must rerun the flagship persistent phase pack"
            );
            assert!(
                classes.contains("topology_interference"),
                "{bead_id} must carry topology-sensitive interference coverage"
            );
            assert!(
                classes.contains("crash_fault_recovery"),
                "{bead_id} must carry crash/fault/recovery coverage"
            );
            assert_eq!(
                classes.len(),
                3,
                "{bead_id} should stay on the three-surface C-track validation shape"
            );
        } else {
            panic!("unexpected included bead classification: {bead_id}");
        }
    }
}

#[test]
fn operator_entrypoint_exists() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(ENTRYPOINT_PATH);
    assert!(
        path.is_file(),
        "operator entrypoint missing at {}",
        path.display()
    );
}
