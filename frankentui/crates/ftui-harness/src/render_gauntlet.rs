#![forbid(unsafe_code)]

//! Render equivalence, replay, and tail-latency gauntlet (bd-40lhe).
//!
//! The standing safety net for render performance work. Every render optimization
//! must pass this gauntlet before graduating to production. The gauntlet integrates:
//!
//! - **Fixture suite** (`fixture_suite`): canonical, challenge, and negative-control workloads
//! - **Render certificates** (`render_certificate`): skip-safety verification
//! - **Presenter equivalence** (`presenter_equivalence`): ANSI output identity checks
//! - **Layout reuse** (`layout_reuse`): cache correctness verification
//! - **Cost surface** (`cost_surface`): stage-level regression detection
//! - **Baseline capture** (`baseline_capture`): latency percentile comparison
//!
//! # Gauntlet structure
//!
//! ```text
//! GauntletSuite
//! ├── EquivalenceGate    — visible output must match baseline
//! ├── ReplayGate         — deterministic replay produces identical checksums
//! ├── TailLatencyGate    — p95/p99 must not regress beyond threshold
//! ├── CertificateGate    — skip decisions must not produce stale frames
//! ├── ChallengeGate      — adversarial fixtures must not crash or corrupt
//! └── NegativeControlGate — no-change fixtures must remain unchanged
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use ftui_harness::render_gauntlet::*;
//!
//! let config = GauntletConfig::default_strict();
//! let suite = GauntletSuite::new(config);
//! let report = suite.run_all();
//! assert!(report.passed(), "gauntlet failed: {}", report.summary());
//! ```

use crate::baseline_capture::FixtureFamily;
use crate::fixture_suite::SuitePartition;

// ============================================================================
// Gauntlet Gates
// ============================================================================

/// Individual gates in the gauntlet, each testing a different correctness property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GauntletGate {
    /// Visible output must match baseline (ANSI byte identity or equivalence-class match).
    Equivalence,
    /// Deterministic replay with same seed must produce identical frame checksums.
    Replay,
    /// Tail latency (p95/p99) must not regress beyond configured threshold.
    TailLatency,
    /// Certificate skip decisions must not produce visibly different output.
    Certificate,
    /// Adversarial fixtures must complete without panic, corruption, or resource leak.
    Challenge,
    /// Negative-control fixtures must produce unchanged output.
    NegativeControl,
}

impl GauntletGate {
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Equivalence => "equivalence",
            Self::Replay => "replay",
            Self::TailLatency => "tail-latency",
            Self::Certificate => "certificate",
            Self::Challenge => "challenge",
            Self::NegativeControl => "negative-control",
        }
    }

    /// Whether this gate blocks promotion (true) or is informative (false).
    #[must_use]
    pub const fn is_gating(&self) -> bool {
        match self {
            Self::Equivalence => true,
            Self::Replay => true,
            Self::TailLatency => true,
            Self::Certificate => true,
            Self::Challenge => true,
            Self::NegativeControl => true,
        }
    }

    /// Which fixture partitions feed this gate.
    #[must_use]
    pub const fn fixture_partitions(&self) -> &'static [SuitePartition] {
        match self {
            Self::Equivalence => &[SuitePartition::Canonical],
            Self::Replay => &[SuitePartition::Canonical],
            Self::TailLatency => &[SuitePartition::Canonical],
            Self::Certificate => &[SuitePartition::Canonical, SuitePartition::Challenge],
            Self::Challenge => &[SuitePartition::Challenge],
            Self::NegativeControl => &[SuitePartition::NegativeControl],
        }
    }

    /// What failure artifacts this gate produces on failure.
    #[must_use]
    pub const fn failure_artifacts(&self) -> &'static [&'static str] {
        match self {
            Self::Equivalence => &[
                "ansi_diff.txt",
                "baseline_transcript.jsonl",
                "current_transcript.jsonl",
                "mismatch_cell_report.json",
            ],
            Self::Replay => &[
                "replay_checksums.json",
                "divergence_frame_index.json",
                "replay_input_sequence.jsonl",
            ],
            Self::TailLatency => &[
                "latency_histogram.json",
                "p99_regression_detail.json",
                "stage_breakdown.json",
            ],
            Self::Certificate => &[
                "certificate_decision_log.jsonl",
                "shadow_comparison.json",
                "stale_frame_evidence.json",
            ],
            Self::Challenge => &[
                "challenge_results.json",
                "panic_backtrace.txt",
                "resource_leak_report.json",
            ],
            Self::NegativeControl => &[
                "control_diff.json",
                "unexpected_change_cells.json",
            ],
        }
    }

    pub const ALL: &'static [GauntletGate] = &[
        Self::Equivalence,
        Self::Replay,
        Self::TailLatency,
        Self::Certificate,
        Self::Challenge,
        Self::NegativeControl,
    ];
}

// ============================================================================
// Gate Result
// ============================================================================

/// Outcome of a single gate in the gauntlet.
#[derive(Debug, Clone)]
pub struct GateResult {
    /// Which gate was evaluated.
    pub gate: GauntletGate,
    /// Whether the gate passed.
    pub passed: bool,
    /// Number of fixtures tested.
    pub fixtures_tested: u32,
    /// Number of fixtures that passed.
    pub fixtures_passed: u32,
    /// Summary of what was verified.
    pub summary: String,
    /// Failure details (empty if passed).
    pub failures: Vec<GateFailure>,
    /// Wall-clock time for this gate in milliseconds.
    pub duration_ms: u64,
}

impl GateResult {
    /// Create a passing result.
    #[must_use]
    pub fn pass(gate: GauntletGate, fixtures: u32, summary: &str, duration_ms: u64) -> Self {
        Self {
            gate,
            passed: true,
            fixtures_tested: fixtures,
            fixtures_passed: fixtures,
            summary: summary.to_string(),
            failures: Vec::new(),
            duration_ms,
        }
    }

    /// Create a failing result.
    #[must_use]
    pub fn fail(
        gate: GauntletGate,
        fixtures_tested: u32,
        fixtures_passed: u32,
        failures: Vec<GateFailure>,
        duration_ms: u64,
    ) -> Self {
        let summary = format!(
            "{}/{} fixtures passed, {} failure(s)",
            fixtures_passed,
            fixtures_tested,
            failures.len()
        );
        Self {
            gate,
            passed: false,
            fixtures_tested,
            fixtures_passed,
            summary,
            failures,
            duration_ms,
        }
    }
}

/// Details of a single fixture failure within a gate.
#[derive(Debug, Clone)]
pub struct GateFailure {
    /// Fixture that failed.
    pub fixture_id: String,
    /// What went wrong.
    pub reason: String,
    /// Failure category for triage.
    pub category: FailureCategory,
    /// Artifacts produced for diagnosis.
    pub artifacts: Vec<String>,
}

/// Categories of gauntlet failure for triage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailureCategory {
    /// Visible output differs from baseline.
    SemanticRegression,
    /// Logging or metrics are missing or malformed.
    ObservabilityGap,
    /// Optimization only helps curated benchmarks, not challenge fixtures.
    BenchmarkOverfit,
    /// Challenge fixture showed graceful fallback (expected, not a failure).
    ExpectedFallback,
    /// Certificate issued incorrect skip decision.
    StaleCertificate,
    /// Cache returned stale data.
    StaleCache,
    /// Tail latency regressed beyond threshold.
    TailRegression,
    /// Resource leak detected (memory, handles, threads).
    ResourceLeak,
    /// Panic or crash during fixture execution.
    Crash,
}

impl FailureCategory {
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::SemanticRegression => "semantic-regression",
            Self::ObservabilityGap => "observability-gap",
            Self::BenchmarkOverfit => "benchmark-overfit",
            Self::ExpectedFallback => "expected-fallback",
            Self::StaleCertificate => "stale-certificate",
            Self::StaleCache => "stale-cache",
            Self::TailRegression => "tail-regression",
            Self::ResourceLeak => "resource-leak",
            Self::Crash => "crash",
        }
    }

    /// Whether this category indicates a real problem (vs expected behavior).
    #[must_use]
    pub const fn is_real_failure(&self) -> bool {
        !matches!(self, Self::ExpectedFallback)
    }
}

// ============================================================================
// Gauntlet Configuration
// ============================================================================

/// Configuration for the render gauntlet.
#[derive(Debug, Clone)]
pub struct GauntletConfig {
    /// Maximum allowed p95 regression percentage (e.g., 10.0 = 10%).
    pub p95_regression_threshold_pct: f64,
    /// Maximum allowed p99 regression percentage.
    pub p99_regression_threshold_pct: f64,
    /// Whether to run challenge fixtures.
    pub run_challenges: bool,
    /// Whether to run negative controls.
    pub run_negative_controls: bool,
    /// Whether certificate shadow-run comparison is required.
    pub require_certificate_shadow: bool,
    /// Which fixture family to scope to (None = all render fixtures).
    pub family_filter: Option<FixtureFamily>,
    /// Maximum wall-clock seconds for the entire gauntlet.
    pub timeout_secs: u32,
}

impl GauntletConfig {
    /// Default strict: all gates enabled, 10% p95/p99 threshold.
    #[must_use]
    pub const fn default_strict() -> Self {
        Self {
            p95_regression_threshold_pct: 10.0,
            p99_regression_threshold_pct: 15.0,
            run_challenges: true,
            run_negative_controls: true,
            require_certificate_shadow: true,
            family_filter: None,
            timeout_secs: 300,
        }
    }

    /// Fast mode: skip challenges and shadow runs for quick iteration.
    #[must_use]
    pub const fn fast() -> Self {
        Self {
            p95_regression_threshold_pct: 15.0,
            p99_regression_threshold_pct: 20.0,
            run_challenges: false,
            run_negative_controls: true,
            require_certificate_shadow: false,
            family_filter: None,
            timeout_secs: 60,
        }
    }
}

// ============================================================================
// Gauntlet Suite
// ============================================================================

/// The complete render gauntlet suite.
#[derive(Debug, Clone)]
pub struct GauntletSuite {
    /// Configuration.
    pub config: GauntletConfig,
}

impl GauntletSuite {
    /// Create a new gauntlet suite.
    #[must_use]
    pub fn new(config: GauntletConfig) -> Self {
        Self { config }
    }

    /// Which gates are active given the current configuration.
    #[must_use]
    pub fn active_gates(&self) -> Vec<GauntletGate> {
        let mut gates = vec![
            GauntletGate::Equivalence,
            GauntletGate::Replay,
            GauntletGate::TailLatency,
        ];

        if self.config.require_certificate_shadow {
            gates.push(GauntletGate::Certificate);
        }

        if self.config.run_challenges {
            gates.push(GauntletGate::Challenge);
        }

        if self.config.run_negative_controls {
            gates.push(GauntletGate::NegativeControl);
        }

        gates
    }

    /// Number of active gates.
    #[must_use]
    pub fn gate_count(&self) -> usize {
        self.active_gates().len()
    }
}

// ============================================================================
// Gauntlet Report
// ============================================================================

/// Complete gauntlet execution report.
#[derive(Debug, Clone)]
pub struct GauntletReport {
    /// Per-gate results.
    pub gate_results: Vec<GateResult>,
    /// Total wall-clock time in milliseconds.
    pub total_duration_ms: u64,
    /// Configuration used.
    pub config: GauntletConfig,
}

impl GauntletReport {
    /// Whether all gating gates passed.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.gate_results
            .iter()
            .filter(|r| r.gate.is_gating())
            .all(|r| r.passed)
    }

    /// Number of gates that passed.
    #[must_use]
    pub fn gates_passed(&self) -> usize {
        self.gate_results.iter().filter(|r| r.passed).count()
    }

    /// Number of gates that failed.
    #[must_use]
    pub fn gates_failed(&self) -> usize {
        self.gate_results.iter().filter(|r| !r.passed).count()
    }

    /// All failures across all gates.
    #[must_use]
    pub fn all_failures(&self) -> Vec<&GateFailure> {
        self.gate_results
            .iter()
            .flat_map(|r| r.failures.iter())
            .collect()
    }

    /// Real failures (excluding expected fallback).
    #[must_use]
    pub fn real_failures(&self) -> Vec<&GateFailure> {
        self.all_failures()
            .into_iter()
            .filter(|f| f.category.is_real_failure())
            .collect()
    }

    /// Human-readable summary.
    #[must_use]
    pub fn summary(&self) -> String {
        let status = if self.passed() { "PASSED" } else { "FAILED" };
        let real = self.real_failures().len();
        format!(
            "Gauntlet {}: {}/{} gates passed, {} real failure(s), {}ms",
            status,
            self.gates_passed(),
            self.gate_results.len(),
            real,
            self.total_duration_ms,
        )
    }

    /// Serialize to JSON.
    #[must_use]
    pub fn to_json(&self) -> String {
        let gates: Vec<String> = self
            .gate_results
            .iter()
            .map(|r| {
                let failure_entries: Vec<String> = r
                    .failures
                    .iter()
                    .map(|f| {
                        let arts: Vec<String> =
                            f.artifacts.iter().map(|a| format!("\"{a}\"")).collect();
                        format!(
                            r#"        {{
          "fixture_id": "{}",
          "reason": "{}",
          "category": "{}",
          "artifacts": [{}]
        }}"#,
                            f.fixture_id,
                            f.reason.replace('"', "\\\""),
                            f.category.label(),
                            arts.join(", "),
                        )
                    })
                    .collect();

                format!(
                    r#"    {{
      "gate": "{}",
      "passed": {},
      "fixtures_tested": {},
      "fixtures_passed": {},
      "duration_ms": {},
      "summary": "{}",
      "failures": [
{}
      ]
    }}"#,
                    r.gate.label(),
                    r.passed,
                    r.fixtures_tested,
                    r.fixtures_passed,
                    r.duration_ms,
                    r.summary.replace('"', "\\\""),
                    failure_entries.join(",\n"),
                )
            })
            .collect();

        format!(
            r#"{{
  "schema_version": 1,
  "passed": {},
  "gates_passed": {},
  "gates_failed": {},
  "real_failures": {},
  "total_duration_ms": {},
  "summary": "{}",
  "gates": [
{}
  ]
}}"#,
            self.passed(),
            self.gates_passed(),
            self.gates_failed(),
            self.real_failures().len(),
            self.total_duration_ms,
            self.summary().replace('"', "\\\""),
            gates.join(",\n"),
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_gates_labeled() {
        for gate in GauntletGate::ALL {
            assert!(!gate.label().is_empty());
            assert!(!gate.failure_artifacts().is_empty());
        }
        assert_eq!(GauntletGate::ALL.len(), 6);
    }

    #[test]
    fn all_gates_are_gating() {
        for gate in GauntletGate::ALL {
            assert!(gate.is_gating(), "{} should be gating", gate.label());
        }
    }

    #[test]
    fn gates_have_fixture_partitions() {
        for gate in GauntletGate::ALL {
            assert!(
                !gate.fixture_partitions().is_empty(),
                "{} has no fixture partitions",
                gate.label()
            );
        }
    }

    #[test]
    fn challenge_gate_uses_challenge_partition() {
        let partitions = GauntletGate::Challenge.fixture_partitions();
        assert!(partitions.contains(&SuitePartition::Challenge));
    }

    #[test]
    fn negative_control_gate_uses_negative_partition() {
        let partitions = GauntletGate::NegativeControl.fixture_partitions();
        assert!(partitions.contains(&SuitePartition::NegativeControl));
    }

    #[test]
    fn default_strict_config() {
        let config = GauntletConfig::default_strict();
        assert!((config.p95_regression_threshold_pct - 10.0).abs() < 0.01);
        assert!(config.run_challenges);
        assert!(config.run_negative_controls);
        assert!(config.require_certificate_shadow);
    }

    #[test]
    fn fast_config_skips_challenges() {
        let config = GauntletConfig::fast();
        assert!(!config.run_challenges);
        assert!(!config.require_certificate_shadow);
    }

    #[test]
    fn strict_suite_has_all_gates() {
        let suite = GauntletSuite::new(GauntletConfig::default_strict());
        assert_eq!(suite.gate_count(), 6);
    }

    #[test]
    fn fast_suite_has_fewer_gates() {
        let suite = GauntletSuite::new(GauntletConfig::fast());
        assert!(suite.gate_count() < 6);
        assert!(suite.gate_count() >= 3); // equivalence, replay, tail-latency always active
    }

    #[test]
    fn passing_report() {
        let report = GauntletReport {
            gate_results: vec![
                GateResult::pass(GauntletGate::Equivalence, 4, "All equivalent", 100),
                GateResult::pass(GauntletGate::Replay, 4, "All deterministic", 200),
            ],
            total_duration_ms: 300,
            config: GauntletConfig::default_strict(),
        };
        assert!(report.passed());
        assert_eq!(report.gates_passed(), 2);
        assert_eq!(report.gates_failed(), 0);
        assert!(report.all_failures().is_empty());
        assert!(report.summary().contains("PASSED"));
    }

    #[test]
    fn failing_report() {
        let report = GauntletReport {
            gate_results: vec![
                GateResult::pass(GauntletGate::Equivalence, 4, "OK", 100),
                GateResult::fail(
                    GauntletGate::TailLatency,
                    4,
                    3,
                    vec![GateFailure {
                        fixture_id: "render_pipeline_full_200x60".to_string(),
                        reason: "p99 regressed 25% (threshold 15%)".to_string(),
                        category: FailureCategory::TailRegression,
                        artifacts: vec!["latency_histogram.json".to_string()],
                    }],
                    200,
                ),
            ],
            total_duration_ms: 300,
            config: GauntletConfig::default_strict(),
        };
        assert!(!report.passed());
        assert_eq!(report.gates_failed(), 1);
        assert_eq!(report.real_failures().len(), 1);
        assert!(report.summary().contains("FAILED"));
    }

    #[test]
    fn expected_fallback_not_real_failure() {
        let failure = GateFailure {
            fixture_id: "challenge_resize_storm".to_string(),
            reason: "Fell back to full render under resize storm".to_string(),
            category: FailureCategory::ExpectedFallback,
            artifacts: vec![],
        };
        assert!(!failure.category.is_real_failure());
    }

    #[test]
    fn failure_categories_labeled() {
        for cat in [
            FailureCategory::SemanticRegression,
            FailureCategory::ObservabilityGap,
            FailureCategory::BenchmarkOverfit,
            FailureCategory::ExpectedFallback,
            FailureCategory::StaleCertificate,
            FailureCategory::StaleCache,
            FailureCategory::TailRegression,
            FailureCategory::ResourceLeak,
            FailureCategory::Crash,
        ] {
            assert!(!cat.label().is_empty());
        }
    }

    #[test]
    fn only_expected_fallback_is_not_real() {
        for cat in [
            FailureCategory::SemanticRegression,
            FailureCategory::ObservabilityGap,
            FailureCategory::BenchmarkOverfit,
            FailureCategory::StaleCertificate,
            FailureCategory::StaleCache,
            FailureCategory::TailRegression,
            FailureCategory::ResourceLeak,
            FailureCategory::Crash,
        ] {
            assert!(cat.is_real_failure(), "{} should be a real failure", cat.label());
        }
        assert!(!FailureCategory::ExpectedFallback.is_real_failure());
    }

    #[test]
    fn report_to_json_valid() {
        let report = GauntletReport {
            gate_results: vec![GateResult::pass(
                GauntletGate::Equivalence,
                3,
                "All OK",
                50,
            )],
            total_duration_ms: 50,
            config: GauntletConfig::default_strict(),
        };
        let json = report.to_json();
        assert!(json.contains("\"schema_version\": 1"));
        assert!(json.contains("\"passed\": true"));
        assert!(json.contains("\"gates_passed\": 1"));
        assert!(json.contains("\"gate\": \"equivalence\""));
    }

    #[test]
    fn report_json_with_failures() {
        let report = GauntletReport {
            gate_results: vec![GateResult::fail(
                GauntletGate::Certificate,
                2,
                1,
                vec![GateFailure {
                    fixture_id: "test".to_string(),
                    reason: "stale frame".to_string(),
                    category: FailureCategory::StaleCertificate,
                    artifacts: vec!["shadow.json".to_string()],
                }],
                100,
            )],
            total_duration_ms: 100,
            config: GauntletConfig::default_strict(),
        };
        let json = report.to_json();
        assert!(json.contains("\"passed\": false"));
        assert!(json.contains("\"stale-certificate\""));
        assert!(json.contains("\"shadow.json\""));
    }

    #[test]
    fn gate_result_pass_constructor() {
        let r = GateResult::pass(GauntletGate::Replay, 5, "All good", 42);
        assert!(r.passed);
        assert_eq!(r.fixtures_tested, 5);
        assert_eq!(r.fixtures_passed, 5);
        assert_eq!(r.duration_ms, 42);
        assert!(r.failures.is_empty());
    }

    #[test]
    fn gate_result_fail_constructor() {
        let r = GateResult::fail(GauntletGate::TailLatency, 5, 3, vec![], 100);
        assert!(!r.passed);
        assert_eq!(r.fixtures_tested, 5);
        assert_eq!(r.fixtures_passed, 3);
        assert!(r.summary.contains("3/5"));
    }
}
