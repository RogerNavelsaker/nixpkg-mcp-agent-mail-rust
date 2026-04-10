//! Contract tests for db300_shadow_oracle_contract.toml (bd-db300.7.5.6).
//!
//! This contract pins the shared shadow-oracle vocabulary used by E2, E3, D1,
//! and E4 so rollout, fallback, divergence capture, and replay semantics stay
//! consistent across the performance program.

#![allow(clippy::struct_field_names)]

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::Deserialize;

const BEAD_ID: &str = "bd-db300.7.5.6";
const CONTRACT_PATH: &str = "db300_shadow_oracle_contract.toml";

const EXPECTED_SCOPE_IDS: [&str; 4] = [
    "decision_equivalence",
    "invariant_hash",
    "state_hash",
    "user_visible_result",
];
const EXPECTED_POLICY_IDS: [&str; 4] = [
    "bounded_regret_decision_delta",
    "ephemeral_field_scrub",
    "exact_identity",
    "state_hash_semantic_scrub",
];
const EXPECTED_MODE_IDS: [&str; 4] = ["forced", "off", "sampled", "shadow_canary"];
const EXPECTED_DIVERGENCE_IDS: [&str; 6] = [
    "decision_budget_exceeded",
    "fallback_contract_breach",
    "invariant_violation",
    "observability_gap",
    "semantic_result_mismatch",
    "state_hash_mismatch",
];
const EXPECTED_KILL_SWITCH_IDS: [&str; 3] = [
    "immediate_surface_latch",
    "observability_fail_closed",
    "rate_based_decision_latch",
];
const EXPECTED_SCRIPT_IDS: [&str; 5] = [
    "d1_parallel_wal_suite",
    "e2_fused_entry_suite",
    "e3_metadata_publication_suite",
    "e4_controller_guardrails_suite",
    "g5_6_shadow_oracle_suite",
];
const EXPECTED_SURFACE_IDS: [&str; 4] = [
    "d1_parallel_wal",
    "e2_fused_entry",
    "e3_metadata_publication",
    "e4_controller_decisions",
];

#[derive(Debug, Deserialize)]
struct ShadowOracleContractDocument {
    meta: Meta,
    global_defaults: GlobalDefaults,
    logging: LoggingContract,
    counterexample_bundle: CounterexampleBundle,
    #[serde(default, rename = "equivalence_scope")]
    equivalence_scopes: Vec<EquivalenceScope>,
    #[serde(default, rename = "allowed_difference_policy")]
    allowed_difference_policies: Vec<AllowedDifferencePolicy>,
    #[serde(default, rename = "shadow_mode")]
    shadow_modes: Vec<ShadowMode>,
    #[serde(default, rename = "divergence_class")]
    divergence_classes: Vec<DivergenceClass>,
    #[serde(default, rename = "kill_switch_profile")]
    kill_switch_profiles: Vec<KillSwitchProfile>,
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
    oracle_result_authority: String,
    default_shadow_mode: String,
    default_rollout_stage: String,
    concurrent_writer_requirement: String,
    default_eligibility_rule: String,
    fallback_policy: String,
    safe_mode_policy: String,
}

#[derive(Debug, Deserialize)]
struct LoggingContract {
    common_fields: Vec<String>,
    first_failure_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CounterexampleBundle {
    bundle_schema: String,
    required_files: Vec<String>,
    #[serde(default)]
    optional_files: Vec<String>,
    required_fields: Vec<String>,
    #[serde(default)]
    optional_fields: Vec<String>,
    minimization_expectation: String,
    replay_contract: String,
}

#[derive(Debug, Deserialize)]
struct EquivalenceScope {
    scope_id: String,
    comparison_kind: String,
    normalization_rules: Vec<String>,
    blocker: bool,
}

#[derive(Debug, Deserialize)]
struct AllowedDifferencePolicy {
    policy_id: String,
    applies_to_scopes: Vec<String>,
    allowed_differences: Vec<String>,
    forbidden_differences: Vec<String>,
    #[serde(default)]
    decision_budget_rule: String,
}

#[derive(Debug, Deserialize)]
struct ShadowMode {
    mode_id: String,
    oracle_executes: bool,
    candidate_executes: bool,
    authoritative_result: String,
    sample_rate_rule: String,
    escalation_rule: String,
}

#[derive(Debug, Deserialize)]
struct DivergenceClass {
    divergence_class_id: String,
    severity: String,
    description: String,
    default_kill_switch_profile: String,
}

#[derive(Debug, Deserialize)]
struct KillSwitchProfile {
    profile_id: String,
    threshold_count: usize,
    threshold_window: String,
    action: String,
    reset_rule: String,
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
    oracle_identity: String,
    candidate_identity: String,
    equivalence_scopes: Vec<String>,
    allowed_difference_policies: Vec<String>,
    supported_shadow_modes: Vec<String>,
    divergence_classes: Vec<String>,
    kill_switch_profiles: Vec<String>,
    named_scripts: Vec<String>,
    required_log_fields: Vec<String>,
    unit_test_obligations: Vec<String>,
    integration_test_obligations: Vec<String>,
    e2e_obligations: Vec<String>,
}

fn load_contract() -> ShadowOracleContractDocument {
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
    toml::from_str::<ShadowOracleContractDocument>(&content).unwrap_or_else(|error| {
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
        document.global_defaults.oracle_result_authority,
        "conservative_oracle_wins"
    );
    assert_eq!(document.global_defaults.default_shadow_mode, "off");
    assert_eq!(
        document.global_defaults.default_rollout_stage,
        "shadow_only"
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
            .default_eligibility_rule
            .contains("forced")
            && document
                .global_defaults
                .default_eligibility_rule
                .contains("sampled"),
        "default eligibility must be gated by forced and sampled evidence"
    );
    assert!(
        document
            .global_defaults
            .fallback_policy
            .contains("counterexample bundle"),
        "fallback policy must require bundle capture"
    );
    assert!(
        document
            .global_defaults
            .safe_mode_policy
            .contains("conservative"),
        "safe-mode policy must fail closed"
    );
}

#[test]
fn scope_policy_mode_divergence_and_script_vocabularies_are_exact() {
    let document = load_contract();

    let scope_ids = document
        .equivalence_scopes
        .iter()
        .map(|scope| scope.scope_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(scope_ids, expected(&EXPECTED_SCOPE_IDS));
    for scope in &document.equivalence_scopes {
        assert!(
            !scope.comparison_kind.trim().is_empty(),
            "comparison kind missing for {}",
            scope.scope_id
        );
        assert!(
            !scope.normalization_rules.is_empty(),
            "normalization rules missing for {}",
            scope.scope_id
        );
        match scope.scope_id.as_str() {
            "decision_equivalence" => assert!(
                !scope.blocker,
                "decision equivalence should escalate through a policy budget, not a direct blocker bit"
            ),
            "user_visible_result" | "state_hash" | "invariant_hash" => {
                assert!(scope.blocker, "{} must be a blocker scope", scope.scope_id);
            }
            _ => {}
        }
    }

    let policy_ids = document
        .allowed_difference_policies
        .iter()
        .map(|policy| policy.policy_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(policy_ids, expected(&EXPECTED_POLICY_IDS));
    for policy in &document.allowed_difference_policies {
        assert!(
            !policy.applies_to_scopes.is_empty(),
            "policy {} must name scopes",
            policy.policy_id
        );
        assert!(
            !policy.forbidden_differences.is_empty(),
            "policy {} must name forbidden differences",
            policy.policy_id
        );
        if policy.policy_id == "bounded_regret_decision_delta" {
            assert!(
                !policy.allowed_differences.is_empty(),
                "bounded regret policy must name allowed action drift"
            );
            assert!(
                !policy.decision_budget_rule.trim().is_empty(),
                "bounded regret policy must define a budget rule"
            );
        }
    }

    let mode_ids = document
        .shadow_modes
        .iter()
        .map(|mode| mode.mode_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(mode_ids, expected(&EXPECTED_MODE_IDS));
    let forced = document
        .shadow_modes
        .iter()
        .find(|mode| mode.mode_id == "forced")
        .expect("forced mode missing");
    assert!(forced.oracle_executes && forced.candidate_executes);
    assert_eq!(forced.authoritative_result, "oracle");
    let canary = document
        .shadow_modes
        .iter()
        .find(|mode| mode.mode_id == "shadow_canary")
        .expect("shadow_canary mode missing");
    assert!(
        canary
            .authoritative_result
            .contains("candidate_with_oracle_override"),
        "canary must name oracle override semantics"
    );
    for mode in &document.shadow_modes {
        assert!(
            !mode.sample_rate_rule.trim().is_empty(),
            "sample rate rule missing for {}",
            mode.mode_id
        );
        assert!(
            !mode.escalation_rule.trim().is_empty(),
            "escalation rule missing for {}",
            mode.mode_id
        );
    }

    let divergence_ids = document
        .divergence_classes
        .iter()
        .map(|divergence| divergence.divergence_class_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(divergence_ids, expected(&EXPECTED_DIVERGENCE_IDS));
    let kill_switch_ids = document
        .kill_switch_profiles
        .iter()
        .map(|profile| profile.profile_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(kill_switch_ids, expected(&EXPECTED_KILL_SWITCH_IDS));
    for divergence in &document.divergence_classes {
        assert!(
            matches!(divergence.severity.as_str(), "blocker" | "warning"),
            "invalid severity for {}",
            divergence.divergence_class_id
        );
        assert!(
            !divergence.description.trim().is_empty(),
            "description missing for {}",
            divergence.divergence_class_id
        );
        assert!(
            kill_switch_ids.contains(divergence.default_kill_switch_profile.as_str()),
            "divergence {} references unknown default kill switch {}",
            divergence.divergence_class_id,
            divergence.default_kill_switch_profile
        );
    }

    for profile in &document.kill_switch_profiles {
        assert!(
            profile.threshold_count > 0,
            "kill switch {} must have a positive threshold",
            profile.profile_id
        );
        assert!(
            !profile.threshold_window.trim().is_empty()
                && !profile.action.trim().is_empty()
                && !profile.reset_rule.trim().is_empty(),
            "kill switch {} must be fully specified",
            profile.profile_id
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
fn counterexample_bundle_and_logging_contract_are_replayable() {
    let document = load_contract();
    let required_bundle_fields = document
        .counterexample_bundle
        .required_fields
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for required in [
        "trace_id",
        "scenario_id",
        "shadow_run_id",
        "oracle_path",
        "candidate_path",
        "replay_command",
        "diagnostic_json_pointer",
        "artifact_entries",
    ] {
        assert!(
            required_bundle_fields.contains(required),
            "counterexample bundle must contain {}",
            required
        );
    }
    assert!(
        document
            .counterexample_bundle
            .required_files
            .iter()
            .any(|entry| entry == "replay.sh"),
        "bundle must include replay.sh"
    );
    assert!(
        document
            .counterexample_bundle
            .required_files
            .iter()
            .any(|entry| entry == "structured_logs.ndjson"),
        "bundle must include structured logs"
    );
    assert!(
        !document
            .counterexample_bundle
            .bundle_schema
            .trim()
            .is_empty()
            && !document
                .counterexample_bundle
                .minimization_expectation
                .trim()
                .is_empty()
            && !document
                .counterexample_bundle
                .replay_contract
                .trim()
                .is_empty(),
        "bundle schema and replay contract must be explicit"
    );
    assert!(
        document
            .counterexample_bundle
            .optional_fields
            .iter()
            .any(|field| field == "decision_id"),
        "bundle must be able to carry controller decision lineage"
    );
    assert!(
        document
            .counterexample_bundle
            .optional_files
            .iter()
            .any(|file| file == "minimal_reproduction.json"),
        "bundle should allow minimized reproductions"
    );

    let common_fields = document
        .logging
        .common_fields
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for required in [
        "trace_id",
        "scenario_id",
        "shadow_run_id",
        "surface_id",
        "oracle_path",
        "candidate_path",
        "equivalence_scope",
        "allowed_difference_policy",
        "shadow_mode",
        "fallback_state",
        "kill_switch_state",
        "counterexample_bundle",
    ] {
        assert!(
            common_fields.contains(required),
            "common logging field missing: {required}"
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
        "first_failure_json_pointer",
        "replay_command",
        "diagnostic_json_pointer",
    ] {
        assert!(
            first_failure_fields.contains(required),
            "first-failure field missing: {required}"
        );
    }
}

#[test]
fn every_surface_contract_covers_e2_e3_d1_and_e4_with_explicit_obligations() {
    let document = load_contract();
    let surface_ids = document
        .surface_contracts
        .iter()
        .map(|surface| surface.surface_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(surface_ids, expected(&EXPECTED_SURFACE_IDS));

    let scopes = document
        .equivalence_scopes
        .iter()
        .map(|scope| scope.scope_id.as_str())
        .collect::<BTreeSet<_>>();
    let policies = document
        .allowed_difference_policies
        .iter()
        .map(|policy| policy.policy_id.as_str())
        .collect::<BTreeSet<_>>();
    let modes = document
        .shadow_modes
        .iter()
        .map(|mode| mode.mode_id.as_str())
        .collect::<BTreeSet<_>>();
    let divergences = document
        .divergence_classes
        .iter()
        .map(|divergence| divergence.divergence_class_id.as_str())
        .collect::<BTreeSet<_>>();
    let kill_switches = document
        .kill_switch_profiles
        .iter()
        .map(|profile| profile.profile_id.as_str())
        .collect::<BTreeSet<_>>();
    let scripts = document
        .named_scripts
        .iter()
        .map(|script| script.script_id.as_str())
        .collect::<BTreeSet<_>>();
    let common_log_fields = document
        .logging
        .common_fields
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();

    for surface in &document.surface_contracts {
        assert!(
            matches!(surface.pillar_id.as_str(), "E2" | "E3" | "D1" | "E4"),
            "unexpected pillar id for {}",
            surface.surface_id
        );
        assert!(
            !surface.title.trim().is_empty()
                && !surface.oracle_identity.trim().is_empty()
                && !surface.candidate_identity.trim().is_empty(),
            "surface {} must name oracle and candidate identities",
            surface.surface_id
        );
        assert!(
            !surface.unit_test_obligations.is_empty()
                && !surface.integration_test_obligations.is_empty()
                && !surface.e2e_obligations.is_empty(),
            "surface {} must have unit, integration, and e2e obligations",
            surface.surface_id
        );
        assert!(
            !surface.required_log_fields.is_empty(),
            "surface {} must name required log fields",
            surface.surface_id
        );
        for scope in &surface.equivalence_scopes {
            assert!(
                scopes.contains(scope.as_str()),
                "surface {} references unknown scope {}",
                surface.surface_id,
                scope
            );
        }
        for policy in &surface.allowed_difference_policies {
            assert!(
                policies.contains(policy.as_str()),
                "surface {} references unknown policy {}",
                surface.surface_id,
                policy
            );
        }
        for mode in &surface.supported_shadow_modes {
            assert!(
                modes.contains(mode.as_str()),
                "surface {} references unknown mode {}",
                surface.surface_id,
                mode
            );
        }
        for divergence in &surface.divergence_classes {
            assert!(
                divergences.contains(divergence.as_str()),
                "surface {} references unknown divergence {}",
                surface.surface_id,
                divergence
            );
        }
        for kill_switch in &surface.kill_switch_profiles {
            assert!(
                kill_switches.contains(kill_switch.as_str()),
                "surface {} references unknown kill switch {}",
                surface.surface_id,
                kill_switch
            );
        }
        for script in &surface.named_scripts {
            assert!(
                scripts.contains(script.as_str()),
                "surface {} references unknown script {}",
                surface.surface_id,
                script
            );
        }
        assert!(
            surface
                .supported_shadow_modes
                .iter()
                .any(|mode| mode == "forced")
                && surface
                    .supported_shadow_modes
                    .iter()
                    .any(|mode| mode == "sampled")
                && surface
                    .supported_shadow_modes
                    .iter()
                    .any(|mode| mode == "shadow_canary"),
            "surface {} must define forced, sampled, and canary modes",
            surface.surface_id
        );
        for field in [
            "trace_id",
            "scenario_id",
            "shadow_run_id",
            "oracle_path",
            "candidate_path",
        ] {
            assert!(
                common_log_fields.contains(field),
                "missing common log field {}",
                field
            );
        }
        for field in &surface.required_log_fields {
            assert!(
                !field.trim().is_empty(),
                "surface {} has a blank required log field",
                surface.surface_id
            );
        }
        for obligation in surface
            .unit_test_obligations
            .iter()
            .chain(surface.integration_test_obligations.iter())
            .chain(surface.e2e_obligations.iter())
        {
            assert!(
                !obligation.trim().is_empty(),
                "surface {} has blank obligation",
                surface.surface_id
            );
        }
    }

    let e4 = document
        .surface_contracts
        .iter()
        .find(|surface| surface.surface_id == "e4_controller_decisions")
        .expect("missing e4 surface");
    assert!(
        e4.equivalence_scopes
            .iter()
            .any(|scope| scope == "decision_equivalence"),
        "E4 must include decision equivalence"
    );
    assert!(
        e4.allowed_difference_policies
            .iter()
            .any(|policy| policy == "bounded_regret_decision_delta"),
        "E4 must use the bounded regret policy"
    );
    assert!(
        e4.divergence_classes
            .iter()
            .any(|divergence| divergence == "decision_budget_exceeded"),
        "E4 must classify decision-budget divergence"
    );
    assert!(
        e4.kill_switch_profiles
            .iter()
            .any(|profile| profile == "rate_based_decision_latch"),
        "E4 must define a rate-based decision latch"
    );

    for surface_id in [
        "e2_fused_entry",
        "e3_metadata_publication",
        "d1_parallel_wal",
    ] {
        let surface = document
            .surface_contracts
            .iter()
            .find(|entry| entry.surface_id == surface_id)
            .unwrap_or_else(|| panic!("missing surface {}", surface_id));
        assert!(
            surface
                .equivalence_scopes
                .iter()
                .any(|scope| scope == "state_hash"),
            "{} must include state_hash comparison",
            surface_id
        );
        assert!(
            surface
                .equivalence_scopes
                .iter()
                .any(|scope| scope == "user_visible_result"),
            "{} must include user_visible_result comparison",
            surface_id
        );
    }
}
