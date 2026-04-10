#![forbid(unsafe_code)]

//! Rollout go/no-go scorecard for the Asupersync migration (bd-2crbt).
//!
//! Combines shadow-run determinism evidence with benchmark-gate performance
//! evidence into a single structured verdict that operators can use for
//! release decisions.
//!
//! # Design
//!
//! A [`RolloutScorecard`] aggregates:
//! - One or more [`ShadowRunResult`]s proving frame-level determinism.
//! - An optional [`GateResult`] proving performance budgets are met.
//! - Policy-configurable thresholds (minimum shadow match ratio, required
//!   scenario coverage).
//!
//! The scorecard emits structured JSONL evidence and produces a [`RolloutVerdict`]
//! that is either `Go`, `NoGo`, or `Inconclusive` (not enough evidence).
//!
//! # Example
//!
//! ```ignore
//! use ftui_harness::rollout_scorecard::{RolloutScorecard, RolloutScorecardConfig};
//!
//! let config = RolloutScorecardConfig::default()
//!     .min_shadow_scenarios(3)
//!     .min_match_ratio(1.0);
//!
//! let mut scorecard = RolloutScorecard::new(config);
//! scorecard.add_shadow_result(shadow_result_1);
//! scorecard.add_shadow_result(shadow_result_2);
//! scorecard.set_benchmark_gate(gate_result);
//!
//! let verdict = scorecard.evaluate();
//! assert!(verdict.is_go());
//! ```

use crate::benchmark_gate::GateResult;
use crate::shadow_run::{ShadowRunResult, ShadowVerdict};
use ftui_runtime::effect_system::QueueTelemetry;

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for rollout scorecard evaluation.
#[derive(Debug, Clone)]
pub struct RolloutScorecardConfig {
    /// Minimum number of shadow-run scenarios required for a `Go` verdict.
    /// Default: 1.
    pub min_shadow_scenarios: usize,
    /// Minimum frame match ratio (0.0–1.0) across all shadow runs.
    /// Default: 1.0 (100% match required).
    pub min_match_ratio: f64,
    /// Whether a passing benchmark gate is required for `Go`.
    /// Default: false (benchmark evidence is informational, not blocking).
    pub require_benchmark_pass: bool,
}

impl Default for RolloutScorecardConfig {
    fn default() -> Self {
        Self {
            min_shadow_scenarios: 1,
            min_match_ratio: 1.0,
            require_benchmark_pass: false,
        }
    }
}

impl RolloutScorecardConfig {
    /// Set the minimum number of shadow scenarios required.
    #[must_use]
    pub fn min_shadow_scenarios(mut self, n: usize) -> Self {
        self.min_shadow_scenarios = n;
        self
    }

    /// Set the minimum frame match ratio (0.0–1.0).
    #[must_use]
    pub fn min_match_ratio(mut self, ratio: f64) -> Self {
        self.min_match_ratio = ratio.clamp(0.0, 1.0);
        self
    }

    /// Require a passing benchmark gate for `Go` verdict.
    #[must_use]
    pub fn require_benchmark_pass(mut self, required: bool) -> Self {
        self.require_benchmark_pass = required;
        self
    }
}

// ============================================================================
// Verdict
// ============================================================================

/// Go/no-go verdict from the rollout scorecard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RolloutVerdict {
    /// All evidence meets thresholds — safe to proceed with rollout.
    Go,
    /// Evidence shows determinism failure or performance regression.
    NoGo,
    /// Not enough evidence to make a decision.
    Inconclusive,
}

impl RolloutVerdict {
    /// Whether the verdict is `Go`.
    #[must_use]
    pub fn is_go(self) -> bool {
        matches!(self, Self::Go)
    }

    /// Human-readable label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Go => "GO",
            Self::NoGo => "NO-GO",
            Self::Inconclusive => "INCONCLUSIVE",
        }
    }
}

impl std::fmt::Display for RolloutVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

// ============================================================================
// Scorecard
// ============================================================================

/// Rollout go/no-go scorecard.
///
/// Collects shadow-run and benchmark evidence, then evaluates against
/// configured thresholds to produce a [`RolloutVerdict`].
#[derive(Debug)]
pub struct RolloutScorecard {
    config: RolloutScorecardConfig,
    shadow_results: Vec<ShadowRunResult>,
    benchmark_gate: Option<GateResult>,
}

impl RolloutScorecard {
    /// Create a new scorecard with the given configuration.
    pub fn new(config: RolloutScorecardConfig) -> Self {
        Self {
            config,
            shadow_results: Vec::new(),
            benchmark_gate: None,
        }
    }

    /// Add a shadow-run comparison result.
    pub fn add_shadow_result(&mut self, result: ShadowRunResult) {
        self.shadow_results.push(result);
    }

    /// Set the benchmark gate result.
    pub fn set_benchmark_gate(&mut self, result: GateResult) {
        self.benchmark_gate = Some(result);
    }

    /// Number of shadow scenarios recorded.
    #[must_use]
    pub fn shadow_scenario_count(&self) -> usize {
        self.shadow_results.len()
    }

    /// Number of shadow scenarios that matched (all frames identical).
    #[must_use]
    pub fn shadow_match_count(&self) -> usize {
        self.shadow_results
            .iter()
            .filter(|r| r.verdict == ShadowVerdict::Match)
            .count()
    }

    /// Aggregate frame match ratio across all shadow runs.
    #[must_use]
    pub fn aggregate_match_ratio(&self) -> f64 {
        if self.shadow_results.is_empty() {
            return 0.0;
        }
        let total_frames: usize = self.shadow_results.iter().map(|r| r.frames_compared).sum();
        if total_frames == 0 {
            return 1.0;
        }
        let matched_frames: usize = self
            .shadow_results
            .iter()
            .flat_map(|r| r.frame_comparisons.iter())
            .filter(|c| c.matched)
            .count();
        matched_frames as f64 / total_frames as f64
    }

    /// Evaluate the scorecard and produce a verdict.
    #[must_use]
    pub fn evaluate(&self) -> RolloutVerdict {
        // Check minimum scenario coverage
        if self.shadow_results.len() < self.config.min_shadow_scenarios {
            return RolloutVerdict::Inconclusive;
        }

        // Check shadow determinism
        let match_ratio = self.aggregate_match_ratio();
        if match_ratio < self.config.min_match_ratio {
            return RolloutVerdict::NoGo;
        }

        // Check any shadow divergence
        if self
            .shadow_results
            .iter()
            .any(|r| r.verdict == ShadowVerdict::Diverged)
        {
            return RolloutVerdict::NoGo;
        }

        // Check benchmark gate if required
        if self.config.require_benchmark_pass {
            match &self.benchmark_gate {
                None => return RolloutVerdict::Inconclusive,
                Some(gate) if !gate.passed() => return RolloutVerdict::NoGo,
                _ => {}
            }
        }

        RolloutVerdict::Go
    }

    /// Produce a structured summary for operator review.
    #[must_use]
    pub fn summary(&self) -> RolloutSummary {
        let verdict = self.evaluate();
        RolloutSummary {
            verdict,
            shadow_scenarios: self.shadow_results.len(),
            shadow_matches: self.shadow_match_count(),
            aggregate_match_ratio: self.aggregate_match_ratio(),
            total_frames_compared: self.shadow_results.iter().map(|r| r.frames_compared).sum(),
            benchmark_passed: self.benchmark_gate.as_ref().map(|g| g.passed()),
            min_shadow_scenarios_required: self.config.min_shadow_scenarios,
            min_match_ratio_required: self.config.min_match_ratio,
            benchmark_required: self.config.require_benchmark_pass,
        }
    }
}

/// Structured summary of the rollout scorecard for operator review.
#[derive(Debug, Clone)]
pub struct RolloutSummary {
    /// Final verdict.
    pub verdict: RolloutVerdict,
    /// Number of shadow scenarios executed.
    pub shadow_scenarios: usize,
    /// Number of shadow scenarios that matched.
    pub shadow_matches: usize,
    /// Aggregate frame match ratio (0.0–1.0).
    pub aggregate_match_ratio: f64,
    /// Total frames compared across all shadow runs.
    pub total_frames_compared: usize,
    /// Benchmark gate result (None if not provided).
    pub benchmark_passed: Option<bool>,
    /// Configuration: minimum shadow scenarios required.
    pub min_shadow_scenarios_required: usize,
    /// Configuration: minimum match ratio required.
    pub min_match_ratio_required: f64,
    /// Configuration: whether benchmark is required.
    pub benchmark_required: bool,
}

impl RolloutSummary {
    /// Serialize the summary to a JSON string for machine consumption.
    ///
    /// This produces a self-contained evidence artifact that CI, operator
    /// dashboards, and go/no-go gates can consume without parsing human text.
    #[must_use]
    pub fn to_json(&self) -> String {
        let benchmark_str = match self.benchmark_passed {
            Some(true) => "\"pass\"",
            Some(false) => "\"fail\"",
            None => "null",
        };
        format!(
            concat!(
                "{{",
                "\"verdict\":\"{verdict}\",",
                "\"shadow_scenarios\":{scenarios},",
                "\"shadow_matches\":{matches},",
                "\"aggregate_match_ratio\":{ratio},",
                "\"total_frames_compared\":{frames},",
                "\"benchmark_passed\":{bench},",
                "\"config\":{{",
                "\"min_shadow_scenarios\":{min_scenarios},",
                "\"min_match_ratio\":{min_ratio},",
                "\"benchmark_required\":{bench_required}",
                "}}",
                "}}"
            ),
            verdict = self.verdict.label(),
            scenarios = self.shadow_scenarios,
            matches = self.shadow_matches,
            ratio = self.aggregate_match_ratio,
            frames = self.total_frames_compared,
            bench = benchmark_str,
            min_scenarios = self.min_shadow_scenarios_required,
            min_ratio = self.min_match_ratio_required,
            bench_required = self.benchmark_required,
        )
    }
}

impl std::fmt::Display for RolloutSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Rollout Scorecard ===")?;
        writeln!(f, "Verdict: {}", self.verdict)?;
        writeln!(
            f,
            "Shadow: {}/{} scenarios matched ({} required)",
            self.shadow_matches, self.shadow_scenarios, self.min_shadow_scenarios_required,
        )?;
        writeln!(
            f,
            "Match ratio: {:.1}% (>= {:.1}% required)",
            self.aggregate_match_ratio * 100.0,
            self.min_match_ratio_required * 100.0,
        )?;
        writeln!(f, "Frames compared: {}", self.total_frames_compared)?;
        match self.benchmark_passed {
            Some(true) => writeln!(f, "Benchmark: PASS")?,
            Some(false) => writeln!(f, "Benchmark: FAIL")?,
            None if self.benchmark_required => writeln!(f, "Benchmark: MISSING (required)")?,
            None => writeln!(f, "Benchmark: not provided")?,
        }
        Ok(())
    }
}

// ============================================================================
// Evidence bundle (bd-2crbt AC #2, #3)
// ============================================================================

/// Self-contained rollout evidence bundle for release decisions.
///
/// Combines the scorecard verdict with queue telemetry and runtime lane
/// information so operators can make go/no-go decisions from a single
/// artifact without correlating across multiple logs.
#[derive(Debug, Clone)]
pub struct RolloutEvidenceBundle {
    /// Scorecard summary with verdict.
    pub scorecard: RolloutSummary,
    /// Queue telemetry snapshot at evidence-collection time.
    pub queue_telemetry: Option<QueueTelemetry>,
    /// Requested runtime lane.
    pub requested_lane: String,
    /// Resolved runtime lane (after fallback).
    pub resolved_lane: String,
    /// Rollout policy in effect.
    pub rollout_policy: String,
}

impl RolloutEvidenceBundle {
    /// Serialize the full evidence bundle to JSON.
    #[must_use]
    pub fn to_json(&self) -> String {
        let qt_json = match &self.queue_telemetry {
            Some(qt) => format!(
                concat!(
                    "{{",
                    "\"enqueued\":{e},",
                    "\"processed\":{p},",
                    "\"dropped\":{d},",
                    "\"high_water\":{hw},",
                    "\"in_flight\":{inf}",
                    "}}"
                ),
                e = qt.enqueued,
                p = qt.processed,
                d = qt.dropped,
                hw = qt.high_water,
                inf = qt.in_flight,
            ),
            None => "null".to_string(),
        };
        format!(
            concat!(
                "{{",
                "\"schema_version\":\"1.0.0\",",
                "\"scorecard\":{sc},",
                "\"queue_telemetry\":{qt},",
                "\"runtime\":{{",
                "\"requested_lane\":\"{rl}\",",
                "\"resolved_lane\":\"{rsl}\",",
                "\"rollout_policy\":\"{rp}\"",
                "}}",
                "}}"
            ),
            sc = self.scorecard.to_json(),
            qt = qt_json,
            rl = self.requested_lane,
            rsl = self.resolved_lane,
            rp = self.rollout_policy,
        )
    }
}

impl std::fmt::Display for RolloutEvidenceBundle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Rollout Evidence Bundle ===")?;
        writeln!(
            f,
            "Lane: {} (resolved: {})",
            self.requested_lane, self.resolved_lane
        )?;
        writeln!(f, "Policy: {}", self.rollout_policy)?;
        write!(f, "{}", self.scorecard)?;
        if let Some(qt) = &self.queue_telemetry {
            writeln!(
                f,
                "Queue: enqueued={}, processed={}, dropped={}, high_water={}, in_flight={}",
                qt.enqueued, qt.processed, qt.dropped, qt.high_water, qt.in_flight
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shadow_run::{FrameComparison, ShadowRunResult, ShadowVerdict};

    use crate::lab_integration::LabOutput;

    fn empty_lab_output() -> LabOutput {
        LabOutput {
            frame_count: 0,
            frame_records: vec![],
            event_count: 0,
            event_log: vec![],
            tick_count: 0,
            anomaly_count: 0,
        }
    }

    fn make_shadow_result(verdict: ShadowVerdict, frames: usize) -> ShadowRunResult {
        let frame_comparisons: Vec<FrameComparison> = (0..frames)
            .map(|i| FrameComparison {
                index: i,
                baseline_checksum: 0xDEAD_BEEF,
                candidate_checksum: if verdict == ShadowVerdict::Match {
                    0xDEAD_BEEF
                } else {
                    0xCAFE_BABE
                },
                matched: verdict == ShadowVerdict::Match,
            })
            .collect();

        ShadowRunResult {
            verdict,
            scenario_name: "test".to_string(),
            seed: 42,
            frame_comparisons,
            first_divergence: if verdict == ShadowVerdict::Diverged {
                Some(0)
            } else {
                None
            },
            frames_compared: frames,
            baseline: empty_lab_output(),
            candidate: empty_lab_output(),
            baseline_label: "baseline".to_string(),
            candidate_label: "candidate".to_string(),
            run_total: 1,
        }
    }

    #[test]
    fn scorecard_go_with_matching_shadows() {
        let config = RolloutScorecardConfig::default().min_shadow_scenarios(2);
        let mut sc = RolloutScorecard::new(config);
        sc.add_shadow_result(make_shadow_result(ShadowVerdict::Match, 10));
        sc.add_shadow_result(make_shadow_result(ShadowVerdict::Match, 15));

        let verdict = sc.evaluate();
        assert_eq!(verdict, RolloutVerdict::Go);
        assert!(verdict.is_go());
        assert_eq!(sc.aggregate_match_ratio(), 1.0);
    }

    #[test]
    fn scorecard_nogo_with_diverged_shadow() {
        let config = RolloutScorecardConfig::default();
        let mut sc = RolloutScorecard::new(config);
        sc.add_shadow_result(make_shadow_result(ShadowVerdict::Diverged, 10));

        assert_eq!(sc.evaluate(), RolloutVerdict::NoGo);
    }

    #[test]
    fn scorecard_inconclusive_without_enough_scenarios() {
        let config = RolloutScorecardConfig::default().min_shadow_scenarios(3);
        let mut sc = RolloutScorecard::new(config);
        sc.add_shadow_result(make_shadow_result(ShadowVerdict::Match, 10));
        sc.add_shadow_result(make_shadow_result(ShadowVerdict::Match, 10));

        assert_eq!(sc.evaluate(), RolloutVerdict::Inconclusive);
    }

    #[test]
    fn scorecard_inconclusive_when_benchmark_required_but_missing() {
        let config = RolloutScorecardConfig::default().require_benchmark_pass(true);
        let mut sc = RolloutScorecard::new(config);
        sc.add_shadow_result(make_shadow_result(ShadowVerdict::Match, 10));

        assert_eq!(sc.evaluate(), RolloutVerdict::Inconclusive);
    }

    #[test]
    fn scorecard_summary_display() {
        let config = RolloutScorecardConfig::default().min_shadow_scenarios(1);
        let mut sc = RolloutScorecard::new(config);
        sc.add_shadow_result(make_shadow_result(ShadowVerdict::Match, 10));

        let summary = sc.summary();
        let text = summary.to_string();
        assert!(text.contains("GO"));
        assert!(text.contains("100.0%"));
        assert!(text.contains("10"));
    }

    #[test]
    fn verdict_labels() {
        assert_eq!(RolloutVerdict::Go.label(), "GO");
        assert_eq!(RolloutVerdict::NoGo.label(), "NO-GO");
        assert_eq!(RolloutVerdict::Inconclusive.label(), "INCONCLUSIVE");
        assert_eq!(format!("{}", RolloutVerdict::Go), "GO");
    }

    #[test]
    fn scorecard_summary_json_go() {
        let config = RolloutScorecardConfig::default().min_shadow_scenarios(1);
        let mut sc = RolloutScorecard::new(config);
        sc.add_shadow_result(make_shadow_result(ShadowVerdict::Match, 10));

        let json = sc.summary().to_json();
        assert!(json.contains("\"verdict\":\"GO\""));
        assert!(json.contains("\"shadow_scenarios\":1"));
        assert!(json.contains("\"shadow_matches\":1"));
        assert!(json.contains("\"total_frames_compared\":10"));
        assert!(json.contains("\"aggregate_match_ratio\":1"));
        assert!(json.contains("\"benchmark_passed\":null"));
    }

    #[test]
    fn scorecard_summary_json_nogo() {
        let config = RolloutScorecardConfig::default();
        let mut sc = RolloutScorecard::new(config);
        sc.add_shadow_result(make_shadow_result(ShadowVerdict::Diverged, 5));

        let json = sc.summary().to_json();
        assert!(json.contains("\"verdict\":\"NO-GO\""));
        assert!(json.contains("\"shadow_matches\":0"));
    }

    #[test]
    fn scorecard_e2e_with_real_shadow_run() {
        use crate::shadow_run::{ShadowRun, ShadowRunConfig};
        use ftui_core::event::Event;
        use ftui_core::geometry::Rect;
        use ftui_render::frame::Frame;
        use ftui_runtime::program::{Cmd, Model};
        use ftui_widgets::Widget;
        use ftui_widgets::paragraph::Paragraph;

        struct RolloutModel {
            ticks: u64,
        }

        #[derive(Debug, Clone)]
        enum RolloutMsg {
            Tick,
            Quit,
        }

        impl From<Event> for RolloutMsg {
            fn from(e: Event) -> Self {
                match e {
                    Event::Tick => RolloutMsg::Tick,
                    _ => RolloutMsg::Quit,
                }
            }
        }

        impl Model for RolloutModel {
            type Message = RolloutMsg;

            fn update(&mut self, msg: RolloutMsg) -> Cmd<RolloutMsg> {
                match msg {
                    RolloutMsg::Tick => {
                        self.ticks += 1;
                        Cmd::none()
                    }
                    RolloutMsg::Quit => Cmd::quit(),
                }
            }

            fn view(&self, frame: &mut Frame) {
                let text = format!("Ticks: {}", self.ticks);
                let area = Rect::new(0, 0, frame.width(), 1);
                Paragraph::new(text).render(area, frame);
            }
        }

        // Run 3 shadow scenarios with different seeds
        let mut scorecard =
            RolloutScorecard::new(RolloutScorecardConfig::default().min_shadow_scenarios(3));

        for seed in [42, 99, 7] {
            let config = ShadowRunConfig::new("rollout_e2e", "tick_counter", seed).viewport(40, 10);
            let result = ShadowRun::compare(
                config,
                || RolloutModel { ticks: 0 },
                |session| {
                    session.init();
                    for _ in 0..5 {
                        session.tick();
                        session.capture_frame();
                    }
                },
            );
            scorecard.add_shadow_result(result);
        }

        // All scenarios should match (same deterministic model)
        let verdict = scorecard.evaluate();
        assert_eq!(verdict, RolloutVerdict::Go);

        let summary = scorecard.summary();
        assert_eq!(summary.shadow_scenarios, 3);
        assert_eq!(summary.shadow_matches, 3);
        assert_eq!(summary.total_frames_compared, 15); // 5 frames × 3 scenarios
        assert!((summary.aggregate_match_ratio - 1.0).abs() < f64::EPSILON);
        assert!(summary.to_string().contains("GO"));
    }

    #[test]
    fn evidence_bundle_json_contains_all_sections() {
        let config = RolloutScorecardConfig::default().min_shadow_scenarios(1);
        let mut sc = RolloutScorecard::new(config);
        sc.add_shadow_result(make_shadow_result(ShadowVerdict::Match, 5));

        let bundle = RolloutEvidenceBundle {
            scorecard: sc.summary(),
            queue_telemetry: Some(QueueTelemetry {
                enqueued: 10,
                processed: 8,
                dropped: 1,
                high_water: 4,
                in_flight: 1,
            }),
            requested_lane: "structured".to_string(),
            resolved_lane: "structured".to_string(),
            rollout_policy: "shadow".to_string(),
        };

        let json = bundle.to_json();
        assert!(json.contains("\"schema_version\":\"1.0.0\""));
        assert!(json.contains("\"scorecard\":{"));
        assert!(json.contains("\"verdict\":\"GO\""));
        assert!(json.contains("\"queue_telemetry\":{"));
        assert!(json.contains("\"enqueued\":10"));
        assert!(json.contains("\"dropped\":1"));
        assert!(json.contains("\"runtime\":{"));
        assert!(json.contains("\"requested_lane\":\"structured\""));
        assert!(json.contains("\"rollout_policy\":\"shadow\""));
    }

    #[test]
    fn evidence_bundle_display_readable() {
        let config = RolloutScorecardConfig::default().min_shadow_scenarios(1);
        let mut sc = RolloutScorecard::new(config);
        sc.add_shadow_result(make_shadow_result(ShadowVerdict::Match, 5));

        let bundle = RolloutEvidenceBundle {
            scorecard: sc.summary(),
            queue_telemetry: None,
            requested_lane: "asupersync".to_string(),
            resolved_lane: "structured".to_string(),
            rollout_policy: "off".to_string(),
        };

        let text = bundle.to_string();
        assert!(text.contains("Rollout Evidence Bundle"));
        assert!(text.contains("asupersync"));
        assert!(text.contains("structured"));
        assert!(text.contains("GO"));
    }
}
