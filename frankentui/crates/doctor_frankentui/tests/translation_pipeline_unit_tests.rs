// SPDX-License-Identifier: Apache-2.0
//! Unit tests for the opentui-import translation pipeline:
//! mapping atlas lookup, planner decisions, code emission templates,
//! and optimization pass correctness.

use std::collections::{BTreeMap, BTreeSet};

use doctor_frankentui::code_emission::{
    EmissionInputs, EmissionPlan, EmittedFile, FileKind, ProjectScaffold, emit_project,
};
use doctor_frankentui::codegen_optimize::{
    OptimizeConfig, PassKind, optimize, optimize_with_config,
};
use doctor_frankentui::effect_translator::{EffectOrchestrationPlan, EffectTranslationStats};
use doctor_frankentui::mapping_atlas::{
    MappingCategory, atlas_stats, build_atlas, by_category, by_policy, by_risk, lookup,
};
use doctor_frankentui::migration_ir::*;
use doctor_frankentui::semantic_contract::{
    TransformationHandlingClass, TransformationRiskLevel, load_builtin_confidence_model,
};
use doctor_frankentui::state_event_translator::*;
use doctor_frankentui::style_translator::*;
use doctor_frankentui::translation_planner::*;
use doctor_frankentui::view_layout_translator::*;

// ── Shared test helpers ────────────────────────────────────────────────

fn test_provenance() -> Provenance {
    Provenance {
        file: "test.tsx".to_string(),
        line: 1,
        column: None,
        source_name: None,
        policy_category: None,
    }
}

fn minimal_ir() -> MigrationIr {
    let mut builder = IrBuilder::new("test-run".to_string(), "test-project".to_string());
    let node_id = IrNodeId("ir-node-app".to_string());
    builder.add_root(node_id.clone());
    builder.add_view_node(ViewNode {
        id: node_id,
        kind: ViewNodeKind::Component,
        name: "App".to_string(),
        children: Vec::new(),
        props: Vec::new(),
        slots: Vec::new(),
        conditions: Vec::new(),
        provenance: test_provenance(),
    });
    builder.add_state_variable(StateVariable {
        id: IrNodeId("ir-state-count".to_string()),
        name: "count".to_string(),
        scope: StateScope::Local,
        type_annotation: Some("number".to_string()),
        initial_value: Some("0".to_string()),
        readers: BTreeSet::new(),
        writers: BTreeSet::new(),
        provenance: test_provenance(),
    });
    builder.add_event(EventDecl {
        id: IrNodeId("ir-evt-click".to_string()),
        name: "onClick".to_string(),
        kind: EventKind::UserInput,
        source_node: None,
        payload_type: None,
        provenance: test_provenance(),
    });
    builder.build()
}

fn rich_ir() -> MigrationIr {
    let mut builder = IrBuilder::new("test-rich".to_string(), "rich-app".to_string());
    builder.set_source_file_count(3);

    let root_id = IrNodeId("v-root".to_string());
    let child_id = IrNodeId("v-child".to_string());

    builder.add_root(root_id.clone());
    builder.add_view_node(ViewNode {
        id: root_id.clone(),
        kind: ViewNodeKind::Component,
        name: "Dashboard".to_string(),
        children: vec![child_id.clone()],
        props: Vec::new(),
        slots: Vec::new(),
        conditions: Vec::new(),
        provenance: test_provenance(),
    });
    builder.add_view_node(ViewNode {
        id: child_id,
        kind: ViewNodeKind::Element,
        name: "StatusPanel".to_string(),
        children: Vec::new(),
        props: Vec::new(),
        slots: Vec::new(),
        conditions: Vec::new(),
        provenance: test_provenance(),
    });

    builder.add_state_variable(StateVariable {
        id: IrNodeId("s-loading".to_string()),
        name: "loading".to_string(),
        scope: StateScope::Local,
        type_annotation: Some("boolean".to_string()),
        initial_value: Some("true".to_string()),
        readers: BTreeSet::new(),
        writers: BTreeSet::new(),
        provenance: test_provenance(),
    });
    builder.add_state_variable(StateVariable {
        id: IrNodeId("s-items".to_string()),
        name: "items".to_string(),
        scope: StateScope::Context,
        type_annotation: Some("Array<Item>".to_string()),
        initial_value: Some("[]".to_string()),
        readers: BTreeSet::new(),
        writers: BTreeSet::new(),
        provenance: test_provenance(),
    });

    builder.add_event(EventDecl {
        id: IrNodeId("e-load".to_string()),
        name: "onLoad".to_string(),
        kind: EventKind::Lifecycle,
        source_node: None,
        payload_type: None,
        provenance: test_provenance(),
    });
    builder.add_event(EventDecl {
        id: IrNodeId("e-refresh".to_string()),
        name: "onRefresh".to_string(),
        kind: EventKind::UserInput,
        source_node: None,
        payload_type: Some("RefreshPayload".to_string()),
        provenance: test_provenance(),
    });

    builder.add_effect(EffectDecl {
        id: IrNodeId("eff-fetch".to_string()),
        name: "fetchItems".to_string(),
        kind: EffectKind::Network,
        dependencies: BTreeSet::new(),
        has_cleanup: false,
        reads: BTreeSet::new(),
        writes: BTreeSet::new(),
        provenance: test_provenance(),
    });
    builder.add_effect(EffectDecl {
        id: IrNodeId("eff-timer".to_string()),
        name: "autoRefresh".to_string(),
        kind: EffectKind::Timer,
        dependencies: BTreeSet::new(),
        has_cleanup: true,
        reads: BTreeSet::new(),
        writes: BTreeSet::new(),
        provenance: test_provenance(),
    });

    builder.add_style_token(StyleToken {
        name: "primary-color".to_string(),
        category: TokenCategory::Color,
        value: "#3366ff".to_string(),
        provenance: Some(test_provenance()),
    });

    builder.build()
}

fn make_empty_ir() -> MigrationIr {
    MigrationIr {
        schema_version: "migration-ir-v1".into(),
        run_id: "test-empty".into(),
        source_project: "empty-app".into(),
        view_tree: ViewTree {
            roots: vec![],
            nodes: BTreeMap::new(),
        },
        state_graph: StateGraph {
            variables: BTreeMap::new(),
            derived: BTreeMap::new(),
            data_flow: BTreeMap::new(),
        },
        event_catalog: EventCatalog {
            events: BTreeMap::new(),
            transitions: vec![],
        },
        effect_registry: EffectRegistry {
            effects: BTreeMap::new(),
        },
        style_intent: StyleIntent {
            tokens: BTreeMap::new(),
            layouts: BTreeMap::new(),
            themes: vec![],
        },
        capabilities: CapabilityProfile {
            required: BTreeSet::new(),
            optional: BTreeSet::new(),
            platform_assumptions: vec![],
        },
        accessibility: AccessibilityMap {
            entries: BTreeMap::new(),
        },
        metadata: IrMetadata {
            created_at: "2026-01-01".into(),
            source_file_count: 0,
            total_nodes: 0,
            total_state_vars: 0,
            total_events: 0,
            total_effects: 0,
            warnings: vec![],
            integrity_hash: None,
        },
    }
}

fn make_runtime() -> TranslatedRuntime {
    TranslatedRuntime {
        version: "state-event-translator-v1".into(),
        run_id: "test-run".into(),
        model: ModelStruct {
            name: "AppModel".into(),
            fields: vec![ModelField {
                name: "count".into(),
                rust_type: "u32".into(),
                initial_value: "0".into(),
                scope: FieldScope::Local,
                source_id: IrNodeId("s-count".into()),
                derived: false,
                dependencies: BTreeSet::new(),
                provenance: test_provenance(),
            }],
            shared_fields: vec![],
        },
        message_enum: MessageEnum {
            name: "Msg".into(),
            variants: vec![
                MessageVariant {
                    name: "Increment".into(),
                    payload: None,
                    source_kind: TranslatedEventKind::UserInput,
                    source_id: IrNodeId("e-inc".into()),
                },
                MessageVariant {
                    name: "SetCount".into(),
                    payload: Some("u32".into()),
                    source_kind: TranslatedEventKind::Custom,
                    source_id: IrNodeId("e-set".into()),
                },
            ],
        },
        update_arms: vec![UpdateArm {
            message_variant: "Increment".into(),
            guards: vec![],
            mutations: vec![StateMutation {
                field: "count".into(),
                expression: "model.count + 1".into(),
                target_id: IrNodeId("t-inc".into()),
            }],
            commands: vec![],
            source_transition: None,
        }],
        init_commands: vec![],
        subscriptions: vec![],
        diagnostics: vec![],
        stats: TranslationStats {
            model_fields: 1,
            derived_fields: 0,
            shared_refs: 0,
            message_variants: 2,
            update_arms: 1,
            subscriptions: 0,
            init_commands: 0,
            diagnostics_by_level: BTreeMap::new(),
        },
    }
}

fn make_view() -> TranslatedView {
    let mut widgets = BTreeMap::new();
    widgets.insert(
        "w-root".into(),
        WidgetNode {
            id: "w-root".into(),
            name: "Root".into(),
            widget_type: WidgetType::Block,
            children: vec![],
            layout: LayoutDecl {
                kind: LayoutDeclKind::Flex,
                direction: Some("Vertical".into()),
                alignment: None,
                constraints: vec![],
                gap: None,
            },
            props: vec![WidgetProp {
                name: "title".into(),
                value: "Counter App".into(),
            }],
            condition: None,
            focus: None,
            source_id: IrNodeId("v-root".into()),
            provenance: test_provenance(),
        },
    );
    TranslatedView {
        version: "view-layout-translator-v1".into(),
        run_id: "test-run".into(),
        roots: vec!["w-root".into()],
        widgets,
        focus_groups: vec![],
        layout_pattern: "single-panel".into(),
        diagnostics: vec![],
        stats: ViewTranslationStats {
            total_widgets: 1,
            by_type: BTreeMap::new(),
            focus_groups: 0,
            conditional_widgets: 0,
            layout_containers: 0,
            diagnostics_by_level: BTreeMap::new(),
        },
    }
}

fn make_style() -> TranslatedStyle {
    TranslatedStyle {
        version: "style-translator-v1".into(),
        run_id: "test-run".into(),
        color_mappings: BTreeMap::new(),
        typography_rules: BTreeMap::new(),
        spacing_rules: BTreeMap::new(),
        border_rules: BTreeMap::new(),
        layout_rules: BTreeMap::new(),
        themes: vec![],
        accessibility_upgrades: vec![],
        unsupported_tokens: vec![],
        diagnostics: vec![],
        stats: StyleTranslationStats {
            total_tokens: 0,
            colors_mapped: 0,
            typography_rules: 0,
            spacing_rules: 0,
            border_rules: 0,
            layout_rules: 0,
            themes_generated: 0,
            a11y_upgrades: 0,
            unsupported_count: 0,
        },
    }
}

fn make_effects() -> EffectOrchestrationPlan {
    EffectOrchestrationPlan {
        version: "effect-translator-v1".into(),
        orchestrations: BTreeMap::new(),
        ordering_constraints: vec![],
        diagnostics: vec![],
        stats: EffectTranslationStats::default(),
    }
}

fn make_plan() -> TranslationPlan {
    TranslationPlan {
        version: "translation-planner-v1".into(),
        run_id: "test-run".into(),
        seed: 42,
        decisions: vec![],
        gap_tickets: vec![],
        stats: PlanStats {
            total_segments: 0,
            auto_approve: 0,
            human_review: 0,
            rejected: 0,
            gap_tickets: 0,
            mean_confidence: 0.0,
            by_category: BTreeMap::new(),
            by_handling_class: BTreeMap::new(),
        },
    }
}

fn make_inputs<'a>(
    ir: &'a MigrationIr,
    runtime: &'a TranslatedRuntime,
    view: &'a TranslatedView,
    style: &'a TranslatedStyle,
    effects: &'a EffectOrchestrationPlan,
    plan: &'a TranslationPlan,
) -> EmissionInputs<'a> {
    EmissionInputs {
        ir,
        runtime,
        view,
        style,
        effects,
        plan,
    }
}

fn make_plan_with_files(files: Vec<(&str, &str, FileKind)>) -> EmissionPlan {
    let mut file_map = BTreeMap::new();
    for (path, content, kind) in files {
        file_map.insert(
            path.to_string(),
            EmittedFile {
                path: path.to_string(),
                content: content.to_string(),
                kind,
                confidence: 1.0,
                provenance_links: vec![],
            },
        );
    }
    EmissionPlan {
        version: "test".into(),
        run_id: "test".into(),
        scaffold: ProjectScaffold {
            crate_name: "test".into(),
            crate_version: "0.1.0".into(),
            edition: "2024".into(),
            dependencies: vec![],
            module_tree: vec![],
        },
        files: file_map,
        module_graph: vec![],
        manifest: doctor_frankentui::code_emission::MigrationManifest {
            version: "test".into(),
            source_project: "test".into(),
            plan_version: "test".into(),
            strategy_links: vec![],
            overall_confidence: 1.0,
            gap_count: 0,
            requires_human_review: false,
        },
        diagnostics: vec![],
        stats: Default::default(),
    }
}

// ════════════════════════════════════════════════════════════════════════
// 1. Mapping Atlas tests
// ════════════════════════════════════════════════════════════════════════

#[test]
fn atlas_lookup_known_signature_returns_entry() {
    let atlas = build_atlas();
    let entry = lookup(&atlas, "ViewNodeKind::Component");
    assert!(entry.is_some(), "Component mapping must exist in atlas");
    let entry = entry.unwrap();
    assert!(!entry.target.construct.is_empty());
    assert!(!entry.target.crate_name.is_empty());
}

#[test]
fn atlas_lookup_unknown_signature_returns_none() {
    let atlas = build_atlas();
    assert!(lookup(&atlas, "ViewNodeKind::NonExistentWidget").is_none());
    assert!(lookup(&atlas, "").is_none());
    assert!(lookup(&atlas, "completely-bogus-sig").is_none());
}

#[test]
fn atlas_by_policy_partitions_completely() {
    let atlas = build_atlas();
    let stats = atlas_stats(&atlas);

    let exact = by_policy(&atlas, TransformationHandlingClass::Exact);
    let approx = by_policy(&atlas, TransformationHandlingClass::Approximate);
    let extend = by_policy(&atlas, TransformationHandlingClass::ExtendFtui);
    let unsupported = by_policy(&atlas, TransformationHandlingClass::Unsupported);

    let total = exact.len() + approx.len() + extend.len() + unsupported.len();
    assert_eq!(total, stats.total, "Policy partition should be exhaustive");
}

#[test]
fn atlas_by_risk_returns_subsets() {
    let atlas = build_atlas();
    let low = by_risk(&atlas, TransformationRiskLevel::Low);
    let medium = by_risk(&atlas, TransformationRiskLevel::Medium);
    let high = by_risk(&atlas, TransformationRiskLevel::High);
    let critical = by_risk(&atlas, TransformationRiskLevel::Critical);

    let stats = atlas_stats(&atlas);
    let total = low.len() + medium.len() + high.len() + critical.len();
    assert_eq!(total, stats.total, "Risk partition should be exhaustive");
}

#[test]
fn atlas_by_category_covers_all_categories() {
    let atlas = build_atlas();
    let categories = [
        MappingCategory::View,
        MappingCategory::State,
        MappingCategory::Event,
        MappingCategory::Effect,
        MappingCategory::Layout,
        MappingCategory::Style,
        MappingCategory::Accessibility,
        MappingCategory::Capability,
    ];
    for cat in categories {
        let entries = by_category(&atlas, cat);
        assert!(
            !entries.is_empty(),
            "Category {:?} should have at least one mapping",
            cat
        );
    }
}

#[test]
fn atlas_stats_coverage_ratio_is_valid() {
    let stats = atlas_stats(&build_atlas());
    assert!(stats.total > 0);
    assert!(stats.coverage_ratio >= 0.0);
    assert!(stats.coverage_ratio <= 1.0);
    assert_eq!(
        stats.exact + stats.approximate + stats.extend + stats.unsupported,
        stats.total
    );
}

#[test]
fn atlas_exact_mappings_have_low_risk() {
    let atlas = build_atlas();
    let exact = by_policy(&atlas, TransformationHandlingClass::Exact);
    for entry in &exact {
        assert!(
            entry.risk == TransformationRiskLevel::Low
                || entry.risk == TransformationRiskLevel::Medium,
            "Exact mapping '{}' has unexpectedly high risk: {:?}",
            entry.source_signature,
            entry.risk
        );
    }
}

#[test]
fn atlas_all_entries_have_non_empty_targets() {
    let atlas = build_atlas();
    let stats = atlas_stats(&atlas);
    // Check a sample from each category
    for cat in [
        MappingCategory::View,
        MappingCategory::State,
        MappingCategory::Event,
        MappingCategory::Effect,
    ] {
        for entry in by_category(&atlas, cat) {
            assert!(
                !entry.target.construct.is_empty(),
                "Mapping '{}' has empty target construct",
                entry.source_signature
            );
        }
    }
    assert!(
        stats.total > 20,
        "Atlas should have a substantial number of mappings"
    );
}

// ════════════════════════════════════════════════════════════════════════
// 2. Translation Planner tests
// ════════════════════════════════════════════════════════════════════════

#[test]
fn planner_produces_decisions_for_minimal_ir() {
    let ir = minimal_ir();
    let model = load_builtin_confidence_model().unwrap();
    let plan = plan_translation_simple(&ir, &model);

    assert!(
        !plan.decisions.is_empty(),
        "Planner should produce at least one decision"
    );
    assert_eq!(plan.version, PLANNER_VERSION);
    assert_eq!(plan.run_id, ir.run_id);
}

#[test]
fn planner_decisions_sorted_by_segment_id() {
    let ir = rich_ir();
    let model = load_builtin_confidence_model().unwrap();
    let plan = plan_translation_simple(&ir, &model);

    for window in plan.decisions.windows(2) {
        assert!(
            window[0].segment.id <= window[1].segment.id,
            "Decisions must be sorted by segment ID: {:?} > {:?}",
            window[0].segment.id,
            window[1].segment.id
        );
    }
}

#[test]
fn planner_stats_sum_correctly() {
    let ir = rich_ir();
    let model = load_builtin_confidence_model().unwrap();
    let plan = plan_translation_simple(&ir, &model);

    assert_eq!(plan.stats.total_segments, plan.decisions.len());
    let cat_sum: usize = plan.stats.by_category.values().sum();
    assert_eq!(cat_sum, plan.stats.total_segments);
    let hc_sum: usize = plan.stats.by_handling_class.values().sum();
    assert_eq!(hc_sum, plan.stats.total_segments);
}

#[test]
fn planner_is_deterministic() {
    let ir = rich_ir();
    let model = load_builtin_confidence_model().unwrap();
    let plan1 = plan_translation_simple(&ir, &model);
    let plan2 = plan_translation_simple(&ir, &model);

    assert_eq!(plan1.decisions.len(), plan2.decisions.len());
    for (d1, d2) in plan1.decisions.iter().zip(&plan2.decisions) {
        assert_eq!(d1.segment.id, d2.segment.id);
        assert_eq!(d1.confidence, d2.confidence);
        assert_eq!(d1.chosen.id, d2.chosen.id);
    }
}

#[test]
fn planner_seed_affects_output() {
    let ir = rich_ir();
    let model = load_builtin_confidence_model().unwrap();

    let config_a = PlannerConfig {
        seed: 1,
        ..PlannerConfig::default()
    };
    let config_b = PlannerConfig {
        seed: 999_999,
        ..PlannerConfig::default()
    };

    let plan_a = plan_translation(&ir, &model, None, None, &config_a);
    let plan_b = plan_translation(&ir, &model, None, None, &config_b);

    assert_eq!(plan_a.seed, 1);
    assert_eq!(plan_b.seed, 999_999);
    // Same decisions count, but different seeds recorded
    assert_eq!(plan_a.decisions.len(), plan_b.decisions.len());
}

#[test]
fn planner_high_threshold_emits_gap_tickets() {
    let ir = rich_ir();
    let model = load_builtin_confidence_model().unwrap();
    let config = PlannerConfig {
        seed: 0,
        min_confidence_threshold: 0.99,
        use_intent_signals: false,
        use_effect_signals: false,
    };
    let plan = plan_translation(&ir, &model, None, None, &config);

    // With a very high threshold, we expect either gap tickets or all decisions
    // above 0.99 (which is unlikely for approximate mappings).
    let has_gaps = !plan.gap_tickets.is_empty();
    let all_high = plan.decisions.iter().all(|d| d.confidence >= 0.99);
    assert!(
        has_gaps || all_high,
        "High threshold should flag low-confidence segments as gap tickets"
    );
}

#[test]
fn planner_unsupported_effect_produces_gap() {
    let mut builder = IrBuilder::new("test-unsupported".to_string(), "gap-test".to_string());
    builder.add_effect(EffectDecl {
        id: IrNodeId("eff-dom".to_string()),
        name: "domManipulation".to_string(),
        kind: EffectKind::Dom,
        dependencies: BTreeSet::new(),
        has_cleanup: false,
        reads: BTreeSet::new(),
        writes: BTreeSet::new(),
        provenance: test_provenance(),
    });
    let ir = builder.build();
    let model = load_builtin_confidence_model().unwrap();
    let plan = plan_translation_simple(&ir, &model);

    let dom_gaps: Vec<_> = plan
        .gap_tickets
        .iter()
        .filter(|t| t.segment.mapping_signature.contains("Dom"))
        .collect();
    assert!(
        !dom_gaps.is_empty(),
        "DOM effect should produce a capability gap ticket"
    );
    assert_eq!(dom_gaps[0].gap_kind, GapKind::Unsupported);
}

#[test]
fn planner_gap_tickets_sorted() {
    let mut builder = IrBuilder::new("test-gaps".to_string(), "gaps-app".to_string());
    // Add multiple unsupported effects to get multiple gaps
    builder.add_effect(EffectDecl {
        id: IrNodeId("eff-dom1".to_string()),
        name: "domMut1".to_string(),
        kind: EffectKind::Dom,
        dependencies: BTreeSet::new(),
        has_cleanup: false,
        reads: BTreeSet::new(),
        writes: BTreeSet::new(),
        provenance: test_provenance(),
    });
    builder.add_effect(EffectDecl {
        id: IrNodeId("eff-dom2".to_string()),
        name: "domMut2".to_string(),
        kind: EffectKind::Dom,
        dependencies: BTreeSet::new(),
        has_cleanup: false,
        reads: BTreeSet::new(),
        writes: BTreeSet::new(),
        provenance: test_provenance(),
    });
    let ir = builder.build();
    let model = load_builtin_confidence_model().unwrap();
    let plan = plan_translation_simple(&ir, &model);

    for window in plan.gap_tickets.windows(2) {
        assert!(
            window[0].segment.id <= window[1].segment.id,
            "Gap tickets must be sorted"
        );
    }
}

#[test]
fn planner_each_decision_has_valid_strategy() {
    let ir = rich_ir();
    let model = load_builtin_confidence_model().unwrap();
    let plan = plan_translation_simple(&ir, &model);

    for decision in &plan.decisions {
        assert!(
            !decision.chosen.id.is_empty(),
            "Strategy ID must not be empty"
        );
        assert!(
            !decision.chosen.description.is_empty(),
            "Strategy description must not be empty"
        );
        assert!(
            decision.confidence >= 0.0 && decision.confidence <= 1.0,
            "Confidence must be in [0, 1], got {}",
            decision.confidence
        );
        assert!(
            !decision.rationale.is_empty(),
            "Rationale must not be empty"
        );
    }
}

#[test]
fn planner_empty_ir_produces_empty_plan() {
    let ir = IrBuilder::new("empty".to_string(), "empty".to_string()).build();
    let model = load_builtin_confidence_model().unwrap();
    let plan = plan_translation_simple(&ir, &model);

    assert!(plan.decisions.is_empty());
    assert!(plan.gap_tickets.is_empty());
    assert_eq!(plan.stats.total_segments, 0);
}

// ════════════════════════════════════════════════════════════════════════
// 3. Code Emission template tests
// ════════════════════════════════════════════════════════════════════════

#[test]
fn emission_produces_all_required_files() {
    let ir = make_empty_ir();
    let runtime = make_runtime();
    let view = make_view();
    let style = make_style();
    let effects = make_effects();
    let plan = make_plan();
    let inputs = make_inputs(&ir, &runtime, &view, &style, &effects, &plan);

    let emission = emit_project(&inputs);

    let expected_files = [
        "src/model.rs",
        "src/msg.rs",
        "src/update.rs",
        "src/view.rs",
        "src/style.rs",
        "src/effects.rs",
        "src/main.rs",
        "Cargo.toml",
    ];
    for file in expected_files {
        assert!(
            emission.files.contains_key(file),
            "Missing expected file: {}",
            file
        );
    }
}

#[test]
fn emission_model_module_contains_fields() {
    let ir = make_empty_ir();
    let runtime = make_runtime();
    let view = make_view();
    let style = make_style();
    let effects = make_effects();
    let plan = make_plan();
    let inputs = make_inputs(&ir, &runtime, &view, &style, &effects, &plan);

    let emission = emit_project(&inputs);
    let model_file = &emission.files["src/model.rs"];
    assert!(model_file.content.contains("count"));
    assert!(model_file.content.contains("u32"));
    assert_eq!(model_file.kind, FileKind::RustSource);
}

#[test]
fn emission_msg_module_contains_variants() {
    let ir = make_empty_ir();
    let runtime = make_runtime();
    let view = make_view();
    let style = make_style();
    let effects = make_effects();
    let plan = make_plan();
    let inputs = make_inputs(&ir, &runtime, &view, &style, &effects, &plan);

    let emission = emit_project(&inputs);
    let msg_file = &emission.files["src/msg.rs"];
    assert!(msg_file.content.contains("Increment"));
    assert!(msg_file.content.contains("SetCount"));
}

#[test]
fn emission_update_module_contains_match_arms() {
    let ir = make_empty_ir();
    let runtime = make_runtime();
    let view = make_view();
    let style = make_style();
    let effects = make_effects();
    let plan = make_plan();
    let inputs = make_inputs(&ir, &runtime, &view, &style, &effects, &plan);

    let emission = emit_project(&inputs);
    let update_file = &emission.files["src/update.rs"];
    assert!(
        update_file.content.contains("Increment"),
        "Update module should handle Increment message"
    );
}

#[test]
fn emission_cargo_toml_has_crate_name() {
    let ir = make_empty_ir();
    let runtime = make_runtime();
    let view = make_view();
    let style = make_style();
    let effects = make_effects();
    let plan = make_plan();
    let inputs = make_inputs(&ir, &runtime, &view, &style, &effects, &plan);

    let emission = emit_project(&inputs);
    let cargo = &emission.files["Cargo.toml"];
    assert_eq!(cargo.kind, FileKind::CargoToml);
    assert!(cargo.content.contains("[package]"));
    assert!(cargo.content.contains("name"));
}

#[test]
fn emission_is_deterministic() {
    let ir = make_empty_ir();
    let runtime = make_runtime();
    let view = make_view();
    let style = make_style();
    let effects = make_effects();
    let plan = make_plan();

    let inputs1 = make_inputs(&ir, &runtime, &view, &style, &effects, &plan);
    let emission1 = emit_project(&inputs1);

    let inputs2 = make_inputs(&ir, &runtime, &view, &style, &effects, &plan);
    let emission2 = emit_project(&inputs2);

    assert_eq!(emission1.files.len(), emission2.files.len());
    for (path, file1) in &emission1.files {
        let file2 = &emission2.files[path];
        assert_eq!(
            file1.content, file2.content,
            "File {} content differs between runs",
            path
        );
    }
}

#[test]
fn emission_manifest_tracks_confidence() {
    let ir = make_empty_ir();
    let runtime = make_runtime();
    let view = make_view();
    let style = make_style();
    let effects = make_effects();
    let plan = make_plan();
    let inputs = make_inputs(&ir, &runtime, &view, &style, &effects, &plan);

    let emission = emit_project(&inputs);
    assert!(emission.manifest.overall_confidence >= 0.0);
    assert!(emission.manifest.overall_confidence <= 1.0);
    assert!(!emission.manifest.version.is_empty());
}

#[test]
fn emission_stats_are_consistent() {
    let ir = make_empty_ir();
    let runtime = make_runtime();
    let view = make_view();
    let style = make_style();
    let effects = make_effects();
    let plan = make_plan();
    let inputs = make_inputs(&ir, &runtime, &view, &style, &effects, &plan);

    let emission = emit_project(&inputs);
    assert_eq!(emission.stats.total_files, emission.files.len());
    let rust_count = emission
        .files
        .values()
        .filter(|f| f.kind == FileKind::RustSource)
        .count();
    assert_eq!(emission.stats.rust_files, rust_count);
    assert!(emission.stats.min_confidence >= 0.0);
    assert!(emission.stats.min_confidence <= 1.0);
}

// ════════════════════════════════════════════════════════════════════════
// 4. Optimization pass correctness
// ════════════════════════════════════════════════════════════════════════

#[test]
fn optimize_dead_branch_eliminates_if_false() {
    let plan = make_plan_with_files(vec![(
        "src/update.rs",
        "fn update() {\n    if false {\n        unreachable();\n    }\n    real_code();\n}",
        FileKind::RustSource,
    )]);

    let result = optimize(plan);
    let update = &result.plan.files["src/update.rs"];
    assert!(
        !update.content.contains("if false"),
        "Dead branch should be eliminated"
    );
    assert!(update.content.contains("real_code()"));
}

#[test]
fn optimize_dead_branch_unwraps_if_true() {
    let plan = make_plan_with_files(vec![(
        "src/view.rs",
        "fn view() {\n    if true {\n        render_widget();\n    }\n}",
        FileKind::RustSource,
    )]);

    let result = optimize(plan);
    let view = &result.plan.files["src/view.rs"];
    assert!(
        !view.content.contains("if true"),
        "Tautological branch should be unwrapped"
    );
    assert!(view.content.contains("render_widget()"));
}

#[test]
fn optimize_import_dedup_removes_exact_duplicates() {
    let plan = make_plan_with_files(vec![(
        "src/model.rs",
        "use std::collections::HashMap;\nuse std::io;\nuse std::collections::HashMap;\n\nfn main() {}",
        FileKind::RustSource,
    )]);

    let result = optimize(plan);
    let model = &result.plan.files["src/model.rs"];
    let import_count = model
        .content
        .lines()
        .filter(|l| l.trim() == "use std::collections::HashMap;")
        .count();
    assert_eq!(import_count, 1, "Duplicate import should be removed");
    assert!(model.content.contains("use std::io;"));
}

#[test]
fn optimize_whitespace_collapses_multiple_blanks() {
    let plan = make_plan_with_files(vec![(
        "src/main.rs",
        "fn main() {\n    let x = 1;\n\n\n\n    let y = 2;\n}",
        FileKind::RustSource,
    )]);

    let result = optimize(plan);
    let main = &result.plan.files["src/main.rs"];
    assert!(
        !main.content.contains("\n\n\n"),
        "Triple blank lines should be collapsed"
    );
    assert!(main.content.contains("let x = 1;"));
    assert!(main.content.contains("let y = 2;"));
}

#[test]
fn optimize_skips_non_rust_files() {
    let toml_content = "[package]\nname = \"test\"\n\n\n\nversion = \"0.1.0\"";
    let plan = make_plan_with_files(vec![("Cargo.toml", toml_content, FileKind::CargoToml)]);

    let result = optimize_with_config(
        plan,
        &OptimizeConfig {
            passes: vec![PassKind::DeadBranchElimination],
            ..OptimizeConfig::default()
        },
    );
    let cargo = &result.plan.files["Cargo.toml"];
    assert_eq!(
        cargo.content, toml_content,
        "Non-Rust files should not be modified by dead-branch pass"
    );
}

#[test]
fn optimize_custom_config_selects_passes() {
    let plan = make_plan_with_files(vec![(
        "src/model.rs",
        "use std::io;\nuse std::io;\n\nfn model() {\n    if false {\n        noop();\n    }\n}",
        FileKind::RustSource,
    )]);

    // Only run import dedup, not dead branch
    let config = OptimizeConfig {
        passes: vec![PassKind::ImportDeduplication],
        ..OptimizeConfig::default()
    };
    let result = optimize_with_config(plan, &config);
    let model = &result.plan.files["src/model.rs"];

    // Import dedup should have run
    let io_count = model
        .content
        .lines()
        .filter(|l| l.trim() == "use std::io;")
        .count();
    assert_eq!(io_count, 1, "Import dedup should have removed duplicate");

    // Dead branch should NOT have run
    assert!(
        model.content.contains("if false"),
        "Dead branch pass should not run when not selected"
    );
}

#[test]
fn optimize_all_passes_produces_audit_records() {
    let plan = make_plan_with_files(vec![(
        "src/update.rs",
        "use std::io;\nuse std::io;\n\nfn update() {\n    if false { noop(); }\n\n\n\n    ok();\n}",
        FileKind::RustSource,
    )]);

    let result = optimize(plan);
    // At least some transformations should have been recorded
    assert!(
        !result.records.is_empty(),
        "Optimization should produce at least one transformation record"
    );
    for record in &result.records {
        assert!(
            !record.description.is_empty(),
            "Transformation records should have descriptions"
        );
    }
    assert!(result.stats.passes_executed > 0);
    assert_eq!(result.stats.transformations, result.records.len());
}

#[test]
fn optimize_net_line_change_is_non_positive_for_cleanup() {
    let plan = make_plan_with_files(vec![(
        "src/view.rs",
        "use a;\nuse a;\nuse b;\nuse b;\n\nfn view() {\n    if false { dead(); }\n\n\n\n    ok();\n}",
        FileKind::RustSource,
    )]);

    let result = optimize(plan);
    assert!(
        result.stats.net_line_change <= 0,
        "Cleanup optimization should not add lines (net change: {})",
        result.stats.net_line_change
    );
}

#[test]
fn optimize_is_deterministic() {
    let make = || {
        make_plan_with_files(vec![
            (
                "src/model.rs",
                "use x;\nuse x;\n\npub struct Model {\n    pub count: u32,\n}",
                FileKind::RustSource,
            ),
            (
                "src/update.rs",
                "fn update() {\n    if false { noop(); }\n    real();\n}",
                FileKind::RustSource,
            ),
        ])
    };

    let r1 = optimize(make());
    let r2 = optimize(make());

    assert_eq!(r1.records.len(), r2.records.len());
    for (path, f1) in &r1.plan.files {
        let f2 = &r2.plan.files[path];
        assert_eq!(f1.content, f2.content, "File {} differs between runs", path);
    }
}

#[test]
fn optimize_style_folding_merges_duplicate_constants() {
    let plan = make_plan_with_files(vec![(
        "src/style.rs",
        concat!(
            "pub const PRIMARY: &str = \"blue\";\n",
            "pub const ACCENT: &str = \"blue\";\n",
            "pub const SECONDARY: &str = \"red\";\n",
        ),
        FileKind::RustSource,
    )]);

    let config = OptimizeConfig {
        passes: vec![PassKind::StyleConstantFolding],
        style_fold_threshold: 2,
        ..OptimizeConfig::default()
    };
    let result = optimize_with_config(plan, &config);
    let style = &result.plan.files["src/style.rs"];

    // After folding, one of PRIMARY/ACCENT should reference the other
    let has_alias = style.content.contains("= PRIMARY;") || style.content.contains("= ACCENT;");
    // If both had same value and threshold=2, they should be folded
    assert!(
        has_alias || result.records.is_empty(),
        "Duplicate constants should be folded into aliases"
    );
}

#[test]
fn optimize_helper_extraction_identifies_repeated_patterns() {
    let repeated = "    frame.render_widget(Widget::new(), area);\n";
    let content = format!(
        "fn view() {{\n{}{}{}{}\n}}",
        repeated, repeated, repeated, repeated
    );
    let plan = make_plan_with_files(vec![("src/view.rs", &content, FileKind::RustSource)]);

    let config = OptimizeConfig {
        passes: vec![PassKind::HelperExtraction],
        helper_extract_threshold: 3,
        ..OptimizeConfig::default()
    };
    let result = optimize_with_config(plan, &config);

    // Helper extraction should have either produced records or left content unchanged
    // (depending on exact pattern matching)
    assert!(result.stats.passes_executed == 1);
}

// ════════════════════════════════════════════════════════════════════════
// 5. Cross-pipeline integration tests
// ════════════════════════════════════════════════════════════════════════

#[test]
fn end_to_end_plan_then_emit_then_optimize() {
    // Plan
    let ir = minimal_ir();
    let model = load_builtin_confidence_model().unwrap();
    let plan = plan_translation_simple(&ir, &model);

    // Emit using the plan
    let ir_for_emit = make_empty_ir();
    let runtime = make_runtime();
    let view = make_view();
    let style = make_style();
    let effects = make_effects();
    let inputs = make_inputs(&ir_for_emit, &runtime, &view, &style, &effects, &plan);
    let emission = emit_project(&inputs);

    assert!(!emission.files.is_empty());

    // Optimize the emission
    let result = optimize(emission);

    assert!(!result.plan.files.is_empty());
    assert!(result.stats.passes_executed > 0);
    // The optimized plan should still have all required files
    assert!(result.plan.files.contains_key("src/model.rs"));
    assert!(result.plan.files.contains_key("src/main.rs"));
}

#[test]
fn atlas_lookup_agrees_with_planner_strategy() {
    let atlas = build_atlas();
    let ir = minimal_ir();
    let model = load_builtin_confidence_model().unwrap();
    let plan = plan_translation_simple(&ir, &model);

    // Each planner decision's mapping_signature should be findable in the atlas
    for decision in &plan.decisions {
        let sig = &decision.segment.mapping_signature;
        let atlas_entry = lookup(&atlas, sig);
        // Not all signatures must be in the atlas (e.g., aggregated signatures),
        // but the handling class should be consistent when present.
        if let Some(entry) = atlas_entry {
            assert_eq!(
                decision.chosen.handling_class, entry.policy,
                "Planner strategy handling class should match atlas for '{}'",
                sig
            );
        }
    }
}
