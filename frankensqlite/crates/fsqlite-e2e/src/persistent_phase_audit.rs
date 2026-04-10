use std::time::Duration;

use fsqlite_wal::ConsolidationMetricsSnapshot;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct PersistentLatencySummary {
    pub sample_count: u64,
    pub p50_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
    pub max_us: u64,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct PersistentRetryStageCounts {
    /// Total retry attempts across begin/body/commit stages.
    ///
    /// This intentionally excludes `duplicate_after_retry_exits`, which is a
    /// terminal outcome rather than an additional retry stage.
    pub total_retries: u64,
    pub begin_retries: u64,
    pub body_retries: u64,
    pub commit_retries: u64,
    /// Count of duplicate-row exits after a prior retry likely committed.
    pub duplicate_after_retry_exits: u64,
}

impl PersistentRetryStageCounts {
    pub fn merge(&mut self, other: Self) {
        self.total_retries = self.total_retries.saturating_add(other.total_retries);
        self.begin_retries = self.begin_retries.saturating_add(other.begin_retries);
        self.body_retries = self.body_retries.saturating_add(other.body_retries);
        self.commit_retries = self.commit_retries.saturating_add(other.commit_retries);
        self.duplicate_after_retry_exits = self
            .duplicate_after_retry_exits
            .saturating_add(other.duplicate_after_retry_exits);
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PersistentOperationTiming {
    pub wall_time: Duration,
    pub begin_retry_handoff: Duration,
    pub statement_execute_body: Duration,
    pub commit_roundtrip: Duration,
    pub rollback_cleanup: Duration,
    pub retry_backoff_sleep: Duration,
}

#[derive(Debug, Clone, Serialize)]
pub struct PersistentOperationBucketSummary {
    pub total_us: u64,
    pub avg_us_per_operation: u64,
    pub latency_us: PersistentLatencySummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct PersistentMeasuredCommitBucketSummary {
    pub total_us: u64,
    pub avg_us_per_recorded_commit: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PersistentMeasuredCommitSubBuckets {
    pub recorded_commit_count: u64,
    pub commit_center: PersistentMeasuredCommitBucketSummary,
    pub post_commit_cleanup_publish: PersistentMeasuredCommitBucketSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct PersistentResidualSummary {
    pub total_us: i64,
    pub avg_us_per_operation: i64,
    pub abs_fraction_basis_points: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PersistentCommitRoundtripGapSummary {
    /// Signed difference between harness-measured `COMMIT` wall time and the
    /// inner pager phase instrumentation totals for the same recorded commits.
    pub total_us: i64,
    pub avg_us_per_recorded_commit: i64,
    pub abs_fraction_basis_points: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PersistentOperationWallTimeAudit {
    pub operation_count: u64,
    pub wall_time: PersistentOperationBucketSummary,
    pub begin_retry_handoff: PersistentOperationBucketSummary,
    pub statement_execute_body: PersistentOperationBucketSummary,
    pub commit_roundtrip: PersistentOperationBucketSummary,
    pub rollback_cleanup: PersistentOperationBucketSummary,
    pub retry_backoff_sleep: PersistentOperationBucketSummary,
    pub retry_stage_counts: PersistentRetryStageCounts,
    pub measured_commit_sub_buckets: Option<PersistentMeasuredCommitSubBuckets>,
    pub measured_commit_roundtrip_gap: Option<PersistentCommitRoundtripGapSummary>,
    pub accounted_total_us: u64,
    pub residual: PersistentResidualSummary,
}

#[must_use]
pub fn duration_micros_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

fn sum_duration_micros(iter: impl IntoIterator<Item = Duration>) -> u64 {
    iter.into_iter().fold(0_u64, |acc, duration| {
        acc.saturating_add(duration_micros_u64(duration))
    })
}

#[must_use]
pub fn persistent_latency_summary(sorted: &[Duration]) -> PersistentLatencySummary {
    PersistentLatencySummary {
        sample_count: u64::try_from(sorted.len()).unwrap_or(u64::MAX),
        p50_us: duration_micros_u64(percentile(sorted, 50.0)),
        p95_us: duration_micros_u64(percentile(sorted, 95.0)),
        p99_us: duration_micros_u64(percentile(sorted, 99.0)),
        max_us: duration_micros_u64(sorted.last().copied().unwrap_or(Duration::ZERO)),
    }
}

fn summarize_operation_bucket(
    operation_timings: &[PersistentOperationTiming],
    bucket: impl Fn(&PersistentOperationTiming) -> Duration,
) -> PersistentOperationBucketSummary {
    let mut samples: Vec<Duration> = operation_timings.iter().map(bucket).collect();
    samples.sort();
    let total_us = sum_duration_micros(samples.iter().copied());
    let operation_count = u64::try_from(samples.len()).unwrap_or(u64::MAX);
    PersistentOperationBucketSummary {
        total_us,
        avg_us_per_operation: total_us.checked_div(operation_count).unwrap_or(0),
        latency_us: persistent_latency_summary(&samples),
    }
}

fn signed_total_delta_us(left: u64, right: u64) -> i64 {
    let delta = i128::from(left) - i128::from(right);
    delta.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

fn abs_i64_as_u64(value: i64) -> u64 {
    value.unsigned_abs()
}

fn residual_fraction_basis_points(total_us: u64, residual_us: i64) -> Option<u64> {
    (total_us > 0).then_some(abs_i64_as_u64(residual_us).saturating_mul(10_000) / total_us)
}

pub fn sleep_with_accounting(operation_timing: &mut PersistentOperationTiming, duration: Duration) {
    std::thread::sleep(duration);
    operation_timing.retry_backoff_sleep += duration;
}

#[must_use]
pub fn build_measured_commit_sub_buckets(
    metrics: &ConsolidationMetricsSnapshot,
) -> Option<PersistentMeasuredCommitSubBuckets> {
    let recorded_commit_count = metrics.commit_phase_count;
    if recorded_commit_count == 0 {
        return None;
    }
    let commit_center_total_us = metrics
        .commit_phase_a_us_total
        .saturating_add(metrics.commit_phase_b_us_total);
    let post_commit_cleanup_publish_total_us = metrics
        .commit_phase_c1_us_total
        .saturating_add(metrics.commit_phase_c2_us_total);
    Some(PersistentMeasuredCommitSubBuckets {
        recorded_commit_count,
        commit_center: PersistentMeasuredCommitBucketSummary {
            total_us: commit_center_total_us,
            avg_us_per_recorded_commit: commit_center_total_us
                .checked_div(recorded_commit_count)
                .unwrap_or(0),
        },
        post_commit_cleanup_publish: PersistentMeasuredCommitBucketSummary {
            total_us: post_commit_cleanup_publish_total_us,
            avg_us_per_recorded_commit: post_commit_cleanup_publish_total_us
                .checked_div(recorded_commit_count)
                .unwrap_or(0),
        },
    })
}

#[must_use]
pub fn build_operation_wall_time_audit(
    operation_timings: &[PersistentOperationTiming],
    retry_stage_counts: PersistentRetryStageCounts,
    measured_commit_sub_buckets: Option<PersistentMeasuredCommitSubBuckets>,
) -> PersistentOperationWallTimeAudit {
    let wall_time = summarize_operation_bucket(operation_timings, |timing| timing.wall_time);
    let begin_retry_handoff =
        summarize_operation_bucket(operation_timings, |timing| timing.begin_retry_handoff);
    let statement_execute_body =
        summarize_operation_bucket(operation_timings, |timing| timing.statement_execute_body);
    let commit_roundtrip =
        summarize_operation_bucket(operation_timings, |timing| timing.commit_roundtrip);
    let rollback_cleanup =
        summarize_operation_bucket(operation_timings, |timing| timing.rollback_cleanup);
    let retry_backoff_sleep =
        summarize_operation_bucket(operation_timings, |timing| timing.retry_backoff_sleep);
    let measured_commit_total_us = measured_commit_sub_buckets
        .as_ref()
        .map(|sub_buckets| {
            sub_buckets
                .commit_center
                .total_us
                .saturating_add(sub_buckets.post_commit_cleanup_publish.total_us)
        })
        .unwrap_or(0);
    let accounted_total_us = begin_retry_handoff
        .total_us
        .saturating_add(statement_execute_body.total_us)
        .saturating_add(commit_roundtrip.total_us)
        .saturating_add(rollback_cleanup.total_us)
        .saturating_add(retry_backoff_sleep.total_us);
    let residual_total_us = signed_total_delta_us(wall_time.total_us, accounted_total_us);
    let operation_count = u64::try_from(operation_timings.len()).unwrap_or(u64::MAX);
    let measured_commit_roundtrip_gap = measured_commit_sub_buckets.as_ref().map(|sub_buckets| {
        PersistentCommitRoundtripGapSummary {
            total_us: signed_total_delta_us(commit_roundtrip.total_us, measured_commit_total_us),
            avg_us_per_recorded_commit: signed_total_delta_us(
                commit_roundtrip.total_us,
                measured_commit_total_us,
            )
            .checked_div(i64::try_from(sub_buckets.recorded_commit_count).unwrap_or(i64::MAX))
            .unwrap_or(0),
            abs_fraction_basis_points: residual_fraction_basis_points(
                commit_roundtrip.total_us,
                signed_total_delta_us(commit_roundtrip.total_us, measured_commit_total_us),
            ),
        }
    });
    let residual = PersistentResidualSummary {
        total_us: residual_total_us,
        avg_us_per_operation: residual_total_us
            .checked_div(i64::try_from(operation_count).unwrap_or(i64::MAX))
            .unwrap_or(0),
        abs_fraction_basis_points: residual_fraction_basis_points(
            wall_time.total_us,
            residual_total_us,
        ),
    };

    PersistentOperationWallTimeAudit {
        operation_count,
        wall_time,
        begin_retry_handoff,
        statement_execute_body,
        commit_roundtrip,
        rollback_cleanup,
        retry_backoff_sleep,
        retry_stage_counts,
        measured_commit_sub_buckets,
        measured_commit_roundtrip_gap,
        accounted_total_us,
        residual,
    }
}

#[must_use]
pub fn format_operation_wall_time_audit(audit: &PersistentOperationWallTimeAudit) -> String {
    let measured_commit = audit.measured_commit_sub_buckets.as_ref().map_or_else(
        || {
            "commit_center_avg=n/a post_commit_avg=n/a commit_gap_avg=n/a commit_gap_abs_bp=n/a"
                .to_owned()
        },
        |sub_buckets| {
            let commit_gap = audit.measured_commit_roundtrip_gap.as_ref().map_or_else(
                || "n/a".to_owned(),
                |gap| {
                    format!(
                        "{}us commit_gap_abs_bp={}",
                        gap.avg_us_per_recorded_commit,
                        gap.abs_fraction_basis_points.unwrap_or(0),
                    )
                },
            );
            format!(
                "commit_center_avg={}us post_commit_avg={}us commit_gap_avg={}",
                sub_buckets.commit_center.avg_us_per_recorded_commit,
                sub_buckets
                    .post_commit_cleanup_publish
                    .avg_us_per_recorded_commit,
                commit_gap,
            )
        },
    );
    format!(
        "ops={} wall_avg={}us begin_avg={}us execute_avg={}us commit_avg={}us rollback_avg={}us {} backoff_avg={}us retry_stage={{begin:{}, body:{}, commit:{}, total:{}}} duplicate_after_retry_exits={} accounted_total={}us residual_total={}us residual_avg={}us residual_abs_bp={}",
        audit.operation_count,
        audit.wall_time.avg_us_per_operation,
        audit.begin_retry_handoff.avg_us_per_operation,
        audit.statement_execute_body.avg_us_per_operation,
        audit.commit_roundtrip.avg_us_per_operation,
        audit.rollback_cleanup.avg_us_per_operation,
        measured_commit,
        audit.retry_backoff_sleep.avg_us_per_operation,
        audit.retry_stage_counts.begin_retries,
        audit.retry_stage_counts.body_retries,
        audit.retry_stage_counts.commit_retries,
        audit.retry_stage_counts.total_retries,
        audit.retry_stage_counts.duplicate_after_retry_exits,
        audit.accounted_total_us,
        audit.residual.total_us,
        audit.residual.avg_us_per_operation,
        audit.residual.abs_fraction_basis_points.unwrap_or(0),
    )
}

#[must_use]
pub fn percentile(sorted: &[Duration], pct: f64) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let idx = ((pct / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::{
        PersistentMeasuredCommitBucketSummary, PersistentMeasuredCommitSubBuckets,
        PersistentOperationTiming, PersistentRetryStageCounts, build_measured_commit_sub_buckets,
        build_operation_wall_time_audit,
    };
    use fsqlite_wal::{ConsolidationMetricsSnapshot, PhasePercentiles, WakeReasonSnapshot};
    use std::time::Duration;

    fn micros(value: u64) -> Duration {
        Duration::from_micros(value)
    }

    #[test]
    fn operation_wall_time_audit_keeps_commit_roundtrip_separate_from_measured_commit() {
        let timings = vec![
            PersistentOperationTiming {
                wall_time: micros(100),
                begin_retry_handoff: micros(10),
                statement_execute_body: micros(20),
                commit_roundtrip: micros(40),
                rollback_cleanup: micros(5),
                retry_backoff_sleep: Duration::ZERO,
            },
            PersistentOperationTiming {
                wall_time: micros(120),
                begin_retry_handoff: micros(20),
                statement_execute_body: micros(30),
                commit_roundtrip: micros(50),
                rollback_cleanup: micros(10),
                retry_backoff_sleep: micros(5),
            },
        ];
        let audit = build_operation_wall_time_audit(
            &timings,
            PersistentRetryStageCounts {
                total_retries: 4,
                begin_retries: 1,
                body_retries: 1,
                commit_retries: 2,
                duplicate_after_retry_exits: 1,
            },
            Some(PersistentMeasuredCommitSubBuckets {
                recorded_commit_count: 2,
                commit_center: PersistentMeasuredCommitBucketSummary {
                    total_us: 30,
                    avg_us_per_recorded_commit: 15,
                },
                post_commit_cleanup_publish: PersistentMeasuredCommitBucketSummary {
                    total_us: 20,
                    avg_us_per_recorded_commit: 10,
                },
            }),
        );

        assert_eq!(audit.commit_roundtrip.total_us, 90);
        assert_eq!(audit.rollback_cleanup.total_us, 15);
        assert_eq!(audit.accounted_total_us, 190);
        assert_eq!(audit.residual.total_us, 30);
        assert_eq!(
            audit
                .measured_commit_roundtrip_gap
                .as_ref()
                .expect("measured commit gap should exist")
                .total_us,
            40
        );
        assert_eq!(audit.retry_stage_counts.total_retries, 4);
        assert_eq!(audit.retry_stage_counts.duplicate_after_retry_exits, 1);
    }

    #[test]
    fn measured_commit_sub_buckets_split_commit_center_and_post_commit() {
        let metrics = ConsolidationMetricsSnapshot {
            groups_flushed: 0,
            frames_consolidated: 0,
            transactions_batched: 0,
            fsyncs_total: 0,
            flush_duration_us_total: 0,
            wait_duration_us_total: 0,
            max_group_size_observed: 0,
            busy_retries: 0,
            prepare_us_total: 0,
            consolidator_lock_wait_us_total: 0,
            consolidator_flushing_wait_us_total: 0,
            flusher_arrival_wait_us_total: 0,
            inner_lock_wait_us_total: 0,
            exclusive_lock_us_total: 0,
            wal_append_us_total: 0,
            wal_sync_us_total: 0,
            waiter_epoch_wait_us_total: 0,
            flusher_commits: 0,
            waiter_commits: 0,
            commit_phase_a_us_total: 30,
            commit_phase_b_us_total: 60,
            commit_phase_c1_us_total: 15,
            commit_phase_c2_us_total: 9,
            commit_phase_count: 3,
            hist_consolidator_lock_wait: PhasePercentiles {
                p50: 0,
                p95: 0,
                p99: 0,
                max: 0,
                count: 0,
                mean_us: 0,
            },
            hist_arrival_wait: PhasePercentiles {
                p50: 0,
                p95: 0,
                p99: 0,
                max: 0,
                count: 0,
                mean_us: 0,
            },
            hist_wal_backend_lock_wait: PhasePercentiles {
                p50: 0,
                p95: 0,
                p99: 0,
                max: 0,
                count: 0,
                mean_us: 0,
            },
            hist_wal_append: PhasePercentiles {
                p50: 0,
                p95: 0,
                p99: 0,
                max: 0,
                count: 0,
                mean_us: 0,
            },
            hist_exclusive_lock: PhasePercentiles {
                p50: 0,
                p95: 0,
                p99: 0,
                max: 0,
                count: 0,
                mean_us: 0,
            },
            hist_waiter_epoch_wait: PhasePercentiles {
                p50: 0,
                p95: 0,
                p99: 0,
                max: 0,
                count: 0,
                mean_us: 0,
            },
            hist_phase_b: PhasePercentiles {
                p50: 0,
                p95: 0,
                p99: 0,
                max: 0,
                count: 0,
                mean_us: 0,
            },
            hist_wal_sync: PhasePercentiles {
                p50: 0,
                p95: 0,
                p99: 0,
                max: 0,
                count: 0,
                mean_us: 0,
            },
            hist_full_commit: PhasePercentiles {
                p50: 0,
                p95: 0,
                p99: 0,
                max: 0,
                count: 0,
                mean_us: 0,
            },
            wake_reasons: WakeReasonSnapshot::default(),
        };

        let sub_buckets =
            build_measured_commit_sub_buckets(&metrics).expect("sub-buckets should exist");

        assert_eq!(sub_buckets.recorded_commit_count, 3);
        assert_eq!(sub_buckets.commit_center.total_us, 90);
        assert_eq!(sub_buckets.commit_center.avg_us_per_recorded_commit, 30);
        assert_eq!(sub_buckets.post_commit_cleanup_publish.total_us, 24);
        assert_eq!(
            sub_buckets
                .post_commit_cleanup_publish
                .avg_us_per_recorded_commit,
            8
        );
    }
}
