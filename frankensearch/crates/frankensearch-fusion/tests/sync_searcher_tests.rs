//! Integration tests for synchronous two-tier search orchestration.

use std::sync::Arc;

use frankensearch_core::filter::SearchFilter;
use frankensearch_core::{ScoreSource, SearchError, SearchPhase, TwoTierConfig};
use frankensearch_fusion::SyncTwoTierSearcher;
use frankensearch_index::{InMemoryTwoTierIndex, InMemoryVectorIndex, SearchParams};

fn normalize(values: Vec<f32>) -> Vec<f32> {
    let norm = values.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm <= f32::EPSILON {
        return values;
    }
    values.into_iter().map(|v| v / norm).collect()
}

fn rank_flip_index() -> Arc<InMemoryTwoTierIndex> {
    let doc_ids = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];
    let fast = InMemoryVectorIndex::from_vectors(
        doc_ids.clone(),
        vec![
            normalize(vec![1.0, 0.0]),
            normalize(vec![0.95, 0.05]),
            normalize(vec![0.0, 1.0]),
        ],
        2,
    )
    .expect("fast index");
    let quality = InMemoryVectorIndex::from_vectors(
        doc_ids,
        vec![
            normalize(vec![0.0, 1.0]),
            normalize(vec![1.0, 0.0]),
            normalize(vec![0.2, 0.8]),
        ],
        2,
    )
    .expect("quality index");
    Arc::new(InMemoryTwoTierIndex::new(fast, Some(quality)))
}

#[test]
fn search_collect_returns_progressive_metrics() {
    let searcher = SyncTwoTierSearcher::new(rank_flip_index(), TwoTierConfig::default());
    let (results, metrics) = searcher
        .search_collect(&normalize(vec![1.0, 0.0]), 3)
        .expect("search collect");

    assert_eq!(results.len(), 3);
    assert!(metrics.phase1_vectors_searched > 0);
    assert!(metrics.phase2_vectors_searched > 0);
    assert_eq!(results[0].source, ScoreSource::SemanticQuality);
}

#[test]
fn search_iter_yields_initial_then_refined() {
    let searcher = SyncTwoTierSearcher::new(rank_flip_index(), TwoTierConfig::default());
    let phases = searcher
        .search_iter(&normalize(vec![1.0, 0.0]), 3)
        .collect::<Vec<_>>();

    assert_eq!(phases.len(), 2);
    assert!(matches!(phases[0], SearchPhase::Initial { .. }));
    assert!(matches!(phases[1], SearchPhase::Refined { .. }));
}

#[test]
fn fast_only_mode_skips_phase_two() {
    let config = TwoTierConfig {
        fast_only: true,
        ..TwoTierConfig::default()
    };
    let searcher = SyncTwoTierSearcher::new(rank_flip_index(), config);
    let phases = searcher
        .search_iter(&normalize(vec![1.0, 0.0]), 3)
        .collect::<Vec<_>>();

    assert_eq!(phases.len(), 1);
    assert!(matches!(phases[0], SearchPhase::Initial { .. }));
    let (_, metrics) = searcher
        .search_collect(&normalize(vec![1.0, 0.0]), 3)
        .expect("search collect");
    assert_eq!(metrics.skip_reason.as_deref(), Some("fast_only_enabled"));
}

#[test]
fn search_filter_is_applied_to_results() {
    struct ExcludeA;
    impl SearchFilter for ExcludeA {
        fn matches(&self, doc_id: &str, _metadata: Option<&serde_json::Value>) -> bool {
            doc_id != "a"
        }
        fn name(&self) -> &'static str {
            "exclude-a"
        }
    }

    let searcher = SyncTwoTierSearcher::new(rank_flip_index(), TwoTierConfig::default());
    let (results, _) = searcher
        .search_collect_with_filter(&normalize(vec![1.0, 0.0]), 3, Some(&ExcludeA))
        .expect("filtered search");
    assert!(results.iter().all(|result| result.doc_id != "a"));
}

#[test]
fn quality_weight_controls_refined_ranking() {
    let query = normalize(vec![1.0, 0.0]);
    let fast_only_blend = SyncTwoTierSearcher::new(
        rank_flip_index(),
        TwoTierConfig {
            quality_weight: 0.0,
            candidate_multiplier: 3,
            ..TwoTierConfig::default()
        },
    );
    let quality_only_blend = SyncTwoTierSearcher::new(
        rank_flip_index(),
        TwoTierConfig {
            quality_weight: 1.0,
            candidate_multiplier: 3,
            ..TwoTierConfig::default()
        },
    );

    let (fast_results, _) = fast_only_blend
        .search_collect(&query, 3)
        .expect("fast blend");
    let (quality_results, _) = quality_only_blend
        .search_collect(&query, 3)
        .expect("quality blend");

    assert_eq!(fast_results[0].doc_id, "a");
    assert_eq!(quality_results[0].doc_id, "b");
}

#[test]
fn candidate_multiplier_changes_refinement_recall() {
    let query = normalize(vec![1.0, 0.0]);
    let strict_budget = SyncTwoTierSearcher::new(
        rank_flip_index(),
        TwoTierConfig {
            quality_weight: 1.0,
            candidate_multiplier: 1,
            ..TwoTierConfig::default()
        },
    );
    let relaxed_budget = SyncTwoTierSearcher::new(
        rank_flip_index(),
        TwoTierConfig {
            quality_weight: 1.0,
            candidate_multiplier: 3,
            ..TwoTierConfig::default()
        },
    )
    .with_search_params(SearchParams {
        parallel_enabled: true,
        parallel_threshold: 1,
        parallel_chunk_size: 2,
    });

    let (strict_results, _) = strict_budget
        .search_collect(&query, 1)
        .expect("strict budget search");
    let (relaxed_results, _) = relaxed_budget
        .search_collect(&query, 1)
        .expect("relaxed budget search");

    assert_eq!(strict_results[0].doc_id, "a");
    assert_eq!(relaxed_results[0].doc_id, "b");
}

#[test]
fn empty_index_is_graceful() {
    let empty_fast = InMemoryVectorIndex::from_vectors(Vec::new(), Vec::new(), 2).expect("empty");
    let empty_two_tier = Arc::new(InMemoryTwoTierIndex::new(empty_fast, None));
    let searcher = SyncTwoTierSearcher::new(empty_two_tier, TwoTierConfig::default());

    let (results, metrics) = searcher
        .search_collect(&normalize(vec![1.0, 0.0]), 5)
        .expect("empty search");
    assert!(results.is_empty());
    assert_eq!(metrics.phase1_vectors_searched, 0);
    assert_eq!(
        metrics.skip_reason.as_deref(),
        Some("quality_index_unavailable")
    );
}

#[test]
fn dimension_mismatch_is_reported_by_iterator() {
    let searcher = SyncTwoTierSearcher::new(rank_flip_index(), TwoTierConfig::default());
    let phases = searcher.search_iter(&[], 3).collect::<Vec<_>>();

    assert_eq!(phases.len(), 1);
    match &phases[0] {
        SearchPhase::RefinementFailed { error, .. } => {
            assert!(matches!(error, SearchError::DimensionMismatch { .. }));
        }
        other => panic!("expected RefinementFailed, got {other:?}"),
    }
}

#[test]
fn concurrent_search_collect_is_thread_safe() {
    let searcher = Arc::new(SyncTwoTierSearcher::new(
        rank_flip_index(),
        TwoTierConfig::default(),
    ));
    let query = normalize(vec![1.0, 0.0]);

    let mut handles = Vec::new();
    for _ in 0..8 {
        let searcher = Arc::clone(&searcher);
        let query = query.clone();
        handles.push(std::thread::spawn(move || {
            for _ in 0..20 {
                let (results, metrics) = searcher.search_collect(&query, 3).expect("search");
                assert_eq!(results.len(), 3);
                assert!(metrics.phase1_total_ms >= 0.0);
            }
        }));
    }

    for handle in handles {
        handle.join().expect("thread join");
    }
}
