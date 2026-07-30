use super::*;

#[test]
fn fixture_catalog_mode_matches_plural_fixture_semantics() {
    assert_eq!(fixture_catalog_mode("mixed-1000"), CatalogMode::GettextPo);
    assert_eq!(
        fixture_catalog_mode("gettext-commerce-pl-1000"),
        CatalogMode::GettextPo
    );
    assert_eq!(
        fixture_catalog_mode("catalog-icu-heavy"),
        CatalogMode::IcuPo
    );
}

fn first_po_summary_difference(left: &PoSemanticSummary, right: &PoSemanticSummary) -> String {
    if left.headers != right.headers {
        return format!(
            "headers differ\nleft={}\nright={}",
            canonical_json_string(&left.headers).unwrap_or_else(|_| "<left-json-error>".to_owned()),
            canonical_json_string(&right.headers)
                .unwrap_or_else(|_| "<right-json-error>".to_owned())
        );
    }

    if left.items.len() != right.items.len() {
        return format!(
            "item count differs: left={} right={}",
            left.items.len(),
            right.items.len()
        );
    }

    for (index, (left_item, right_item)) in left.items.iter().zip(&right.items).enumerate() {
        if left_item != right_item {
            return format!(
                "item {} differs\nleft={}\nright={}",
                index,
                canonical_json_string(left_item).unwrap_or_else(|_| "<left-json-error>".to_owned()),
                canonical_json_string(right_item)
                    .unwrap_or_else(|_| "<right-json-error>".to_owned())
            );
        }
    }

    "no summary difference found".to_owned()
}

#[test]
#[ignore = "manual compatibility probe for external adapters"]
fn debug_polib_gettext_ui_de_compatibility() {
    let workspace = workspace_root().expect("workspace");
    let scenarios = vec![
        BenchmarkScenario {
            id: "po-parse/gettext-ui-de-1000/ferrocat-owned".to_owned(),
            comparison_group: "po-parse/gettext-ui-de-1000".to_owned(),
            workload: "po-parse".to_owned(),
            operation: "parse".to_owned(),
            fixture: "gettext-ui-de-1000".to_owned(),
            implementation: "ferrocat-parse".to_owned(),
            warmup_runs: 0,
            measured_runs: 1,
            minimum_sample_millis: Some(1),
        },
        BenchmarkScenario {
            id: "po-parse/gettext-ui-de-1000/polib".to_owned(),
            comparison_group: "po-parse/gettext-ui-de-1000".to_owned(),
            workload: "po-parse".to_owned(),
            operation: "parse".to_owned(),
            fixture: "gettext-ui-de-1000".to_owned(),
            implementation: "polib".to_owned(),
            warmup_runs: 0,
            measured_runs: 1,
            minimum_sample_millis: Some(1),
        },
    ];

    let prepared = PreparedScenario::prepare(&workspace, &scenarios).expect("prepared");
    let internal =
        execute_scenario(&workspace, &prepared, &scenarios[0], 1, true).expect("internal parse");
    let polib =
        execute_scenario(&workspace, &prepared, &scenarios[1], 1, true).expect("polib parse");

    let ExecutionArtifact::PoSummary(internal_summary) =
        internal.artifact.expect("internal artifact")
    else {
        panic!("internal scenario did not return a po summary");
    };
    let ExecutionArtifact::PoSummary(polib_summary) = polib.artifact.expect("polib artifact")
    else {
        panic!("polib scenario did not return a po summary");
    };

    assert_eq!(
        internal_summary,
        polib_summary,
        "{}",
        first_po_summary_difference(&internal_summary, &polib_summary)
    );
}

#[test]
#[ignore = "manual compatibility probe for external adapters"]
fn debug_pofile_gettext_ui_de_compatibility() {
    let workspace = workspace_root().expect("workspace");
    let scenarios = vec![
        BenchmarkScenario {
            id: "po-parse/gettext-ui-de-1000/ferrocat-owned".to_owned(),
            comparison_group: "po-parse/gettext-ui-de-1000".to_owned(),
            workload: "po-parse".to_owned(),
            operation: "parse".to_owned(),
            fixture: "gettext-ui-de-1000".to_owned(),
            implementation: "ferrocat-parse".to_owned(),
            warmup_runs: 0,
            measured_runs: 1,
            minimum_sample_millis: Some(1),
        },
        BenchmarkScenario {
            id: "po-parse/gettext-ui-de-1000/pofile".to_owned(),
            comparison_group: "po-parse/gettext-ui-de-1000".to_owned(),
            workload: "po-parse".to_owned(),
            operation: "parse".to_owned(),
            fixture: "gettext-ui-de-1000".to_owned(),
            implementation: "pofile".to_owned(),
            warmup_runs: 0,
            measured_runs: 1,
            minimum_sample_millis: Some(1),
        },
    ];

    let prepared = PreparedScenario::prepare(&workspace, &scenarios).expect("prepared");
    let internal =
        execute_scenario(&workspace, &prepared, &scenarios[0], 1, true).expect("internal parse");
    let pofile =
        execute_scenario(&workspace, &prepared, &scenarios[1], 1, true).expect("pofile parse");

    let ExecutionArtifact::PoSummary(internal_summary) =
        internal.artifact.expect("internal artifact")
    else {
        panic!("internal scenario did not return a po summary");
    };
    let ExecutionArtifact::PoSummary(pofile_summary) = pofile.artifact.expect("pofile artifact")
    else {
        panic!("pofile scenario did not return a po summary");
    };

    assert_eq!(
        internal_summary,
        pofile_summary,
        "{}",
        first_po_summary_difference(&internal_summary, &pofile_summary)
    );
}

#[test]
#[ignore = "manual compatibility probe for external adapters"]
fn debug_msgmerge_gettext_ui_de_merge_compatibility() {
    let workspace = workspace_root().expect("workspace");
    let scenarios = vec![
        BenchmarkScenario {
            id: "po-merge/gettext-ui-de-1000/ferrocat".to_owned(),
            comparison_group: "po-merge/gettext-ui-de-1000".to_owned(),
            workload: "po-merge-update".to_owned(),
            operation: "merge".to_owned(),
            fixture: "gettext-ui-de-1000".to_owned(),
            implementation: "ferrocat-merge".to_owned(),
            warmup_runs: 0,
            measured_runs: 1,
            minimum_sample_millis: Some(1),
        },
        BenchmarkScenario {
            id: "po-merge/gettext-ui-de-1000/msgmerge".to_owned(),
            comparison_group: "po-merge/gettext-ui-de-1000".to_owned(),
            workload: "po-merge-update".to_owned(),
            operation: "merge".to_owned(),
            fixture: "gettext-ui-de-1000".to_owned(),
            implementation: "msgmerge".to_owned(),
            warmup_runs: 0,
            measured_runs: 1,
            minimum_sample_millis: Some(1),
        },
    ];

    let prepared = PreparedScenario::prepare(&workspace, &scenarios).expect("prepared");
    let internal =
        execute_scenario(&workspace, &prepared, &scenarios[0], 1, true).expect("internal merge");
    let external =
        execute_scenario(&workspace, &prepared, &scenarios[1], 1, true).expect("msgmerge");

    let internal_rendered = match internal.artifact.expect("internal artifact") {
        ExecutionArtifact::RenderedPo(content) => content,
        ExecutionArtifact::RenderedPoPath(path) => {
            std::fs::read_to_string(path).expect("read internal rendered output")
        }
        other => panic!("unexpected internal artifact: {other:?}"),
    };
    let external_rendered = match external.artifact.expect("external artifact") {
        ExecutionArtifact::RenderedPo(content) => content,
        ExecutionArtifact::RenderedPoPath(path) => {
            std::fs::read_to_string(path).expect("read external rendered output")
        }
        other => panic!("unexpected external artifact: {other:?}"),
    };

    let internal_summary =
        PoSemanticSummary::from_po_file(&parse_po(&internal_rendered).expect("parse internal"));
    let external_summary =
        PoSemanticSummary::from_po_file(&parse_po(&external_rendered).expect("parse external"));

    assert_eq!(
        internal_summary,
        external_summary,
        "{}",
        first_po_summary_difference(&internal_summary, &external_summary)
    );
}

#[test]
#[ignore = "manual compatibility probe for external adapters"]
fn debug_msgmerge_gettext_ui_de_update_compatibility() {
    let workspace = workspace_root().expect("workspace");
    let scenarios = vec![
        BenchmarkScenario {
            id: "po-update/gettext-ui-de-1000/ferrocat".to_owned(),
            comparison_group: "po-update/gettext-ui-de-1000".to_owned(),
            workload: "po-merge-update".to_owned(),
            operation: "update-catalog".to_owned(),
            fixture: "gettext-ui-de-1000".to_owned(),
            implementation: "ferrocat-update-catalog".to_owned(),
            warmup_runs: 0,
            measured_runs: 1,
            minimum_sample_millis: Some(1),
        },
        BenchmarkScenario {
            id: "po-update/gettext-ui-de-1000/ferrocat-file".to_owned(),
            comparison_group: "po-update/gettext-ui-de-1000".to_owned(),
            workload: "po-merge-update".to_owned(),
            operation: "update-catalog".to_owned(),
            fixture: "gettext-ui-de-1000".to_owned(),
            implementation: "ferrocat-update-catalog-file".to_owned(),
            warmup_runs: 0,
            measured_runs: 1,
            minimum_sample_millis: Some(1),
        },
        BenchmarkScenario {
            id: "po-update/gettext-ui-de-1000/msgmerge".to_owned(),
            comparison_group: "po-update/gettext-ui-de-1000".to_owned(),
            workload: "po-merge-update".to_owned(),
            operation: "update-catalog".to_owned(),
            fixture: "gettext-ui-de-1000".to_owned(),
            implementation: "msgmerge".to_owned(),
            warmup_runs: 0,
            measured_runs: 1,
            minimum_sample_millis: Some(1),
        },
    ];

    let prepared = PreparedScenario::prepare(&workspace, &scenarios).expect("prepared");
    let internal =
        execute_scenario(&workspace, &prepared, &scenarios[0], 1, true).expect("internal update");
    let internal_file = execute_scenario(&workspace, &prepared, &scenarios[1], 1, true)
        .expect("internal file-based update");
    let external =
        execute_scenario(&workspace, &prepared, &scenarios[2], 1, true).expect("msgmerge");

    assert_eq!(
        internal.reported_digest, internal_file.reported_digest,
        "file-based catalog update must match the pre-structured variant"
    );

    let internal_rendered = match internal.artifact.expect("internal artifact") {
        ExecutionArtifact::RenderedPo(content) => content,
        ExecutionArtifact::RenderedPoPath(path) => {
            std::fs::read_to_string(path).expect("read internal rendered output")
        }
        other => panic!("unexpected internal artifact: {other:?}"),
    };
    let external_rendered = match external.artifact.expect("external artifact") {
        ExecutionArtifact::RenderedPo(content) => content,
        ExecutionArtifact::RenderedPoPath(path) => {
            std::fs::read_to_string(path).expect("read external rendered output")
        }
        other => panic!("unexpected external artifact: {other:?}"),
    };

    let internal_summary =
        PoSemanticSummary::from_po_file(&parse_po(&internal_rendered).expect("parse internal"));
    let external_summary =
        PoSemanticSummary::from_po_file(&parse_po(&external_rendered).expect("parse external"));

    assert_eq!(
        internal_summary,
        external_summary,
        "{}",
        first_po_summary_difference(&internal_summary, &external_summary)
    );
}

fn debug_external_workflow_compatibility(operation: &str, implementation: &str) {
    let workspace = workspace_root().expect("workspace");
    let internal_impl = if operation == "merge" {
        "ferrocat-merge"
    } else {
        "ferrocat-update-catalog"
    };
    let fixture = "gettext-ui-de-1000";
    let group_prefix = if operation == "merge" {
        "po-merge"
    } else {
        "po-update"
    };
    let scenarios = vec![
        BenchmarkScenario {
            id: format!("{group_prefix}/{fixture}/internal"),
            comparison_group: format!("{group_prefix}/{fixture}"),
            workload: "po-merge-update".to_owned(),
            operation: operation.to_owned(),
            fixture: fixture.to_owned(),
            implementation: internal_impl.to_owned(),
            warmup_runs: 0,
            measured_runs: 1,
            minimum_sample_millis: Some(1),
        },
        BenchmarkScenario {
            id: format!("{group_prefix}/{fixture}/{implementation}"),
            comparison_group: format!("{group_prefix}/{fixture}"),
            workload: "po-merge-update".to_owned(),
            operation: operation.to_owned(),
            fixture: fixture.to_owned(),
            implementation: implementation.to_owned(),
            warmup_runs: 0,
            measured_runs: 1,
            minimum_sample_millis: Some(1),
        },
    ];

    let prepared = PreparedScenario::prepare(&workspace, &scenarios).expect("prepared");
    let internal =
        execute_scenario(&workspace, &prepared, &scenarios[0], 1, true).expect("internal workflow");
    let external =
        execute_scenario(&workspace, &prepared, &scenarios[1], 1, true).expect("external workflow");

    let internal_rendered = match internal.artifact.expect("internal artifact") {
        ExecutionArtifact::RenderedPo(content) => content,
        ExecutionArtifact::RenderedPoPath(path) => {
            std::fs::read_to_string(path).expect("read internal rendered output")
        }
        other => panic!("unexpected internal artifact: {other:?}"),
    };
    let external_rendered = match external.artifact.expect("external artifact") {
        ExecutionArtifact::RenderedPo(content) => content,
        ExecutionArtifact::RenderedPoPath(path) => {
            std::fs::read_to_string(path).expect("read external rendered output")
        }
        other => panic!("unexpected external artifact: {other:?}"),
    };

    let internal_summary =
        PoSemanticSummary::from_po_file(&parse_po(&internal_rendered).expect("parse internal"));
    let external_summary =
        PoSemanticSummary::from_po_file(&parse_po(&external_rendered).expect("parse external"));

    assert_eq!(
        external.reported_digest,
        digest_summary(&external_summary).expect("external digest"),
        "{}",
        first_po_summary_difference(
            &PoSemanticSummary::from_po_file(
                &parse_po(&external_rendered).expect("reparse external"),
            ),
            &external_summary
        )
    );

    assert_eq!(
        internal_summary,
        external_summary,
        "{}",
        first_po_summary_difference(&internal_summary, &external_summary)
    );
}

#[test]
#[ignore = "manual compatibility probe for external adapters"]
fn debug_pofile_gettext_ui_de_merge_workflow_compatibility() {
    debug_external_workflow_compatibility("merge", "pofile");
}

#[test]
#[ignore = "manual compatibility probe for external adapters"]
fn debug_pofile_ts_gettext_ui_de_merge_workflow_compatibility() {
    debug_external_workflow_compatibility("merge", "pofile-ts");
}

#[test]
#[ignore = "manual compatibility probe for external adapters"]
fn debug_polib_gettext_ui_de_merge_workflow_compatibility() {
    debug_external_workflow_compatibility("merge", "polib");
}

#[test]
#[ignore = "manual compatibility probe for external adapters"]
fn debug_babel_gettext_ui_de_update_workflow_compatibility() {
    debug_external_workflow_compatibility("update-catalog", "babel");
}

#[test]
fn canonical_po_summary_ignores_item_order() {
    let first = PoSemanticSummary::from_po_file(&PoFile {
        headers: vec![ferrocat_po::Header {
            key: "Language".to_owned(),
            value: "de".to_owned(),
        }],
        items: vec![
            ferrocat_po::PoItem {
                msgid: "b".to_owned(),
                msgstr: MsgStr::Singular("B".to_owned()),
                ..ferrocat_po::PoItem::default()
            },
            ferrocat_po::PoItem {
                msgid: "a".to_owned(),
                msgstr: MsgStr::Singular("A".to_owned()),
                ..ferrocat_po::PoItem::default()
            },
        ],
        ..PoFile::default()
    });
    let second = PoSemanticSummary::from_po_file(&PoFile {
        headers: vec![ferrocat_po::Header {
            key: "Language".to_owned(),
            value: "de".to_owned(),
        }],
        items: vec![
            ferrocat_po::PoItem {
                msgid: "a".to_owned(),
                msgstr: MsgStr::Singular("A".to_owned()),
                ..ferrocat_po::PoItem::default()
            },
            ferrocat_po::PoItem {
                msgid: "b".to_owned(),
                msgstr: MsgStr::Singular("B".to_owned()),
                ..ferrocat_po::PoItem::default()
            },
        ],
        ..PoFile::default()
    });

    assert_eq!(
        digest_summary(&first).expect("digest"),
        digest_summary(&second).expect("digest")
    );
}

#[test]
fn statistics_use_elapsed_distribution() {
    let stats = ScenarioStatistics::from_samples(&[
        ExecutionResult {
            tool_version: "tool".to_owned(),
            reported_digest: "a".to_owned(),
            elapsed_ns: 10,
            baseline_elapsed_ns: None,
            bytes_processed: 1024,
            items_processed: Some(10),
            messages_processed: None,
            artifact: None,
        },
        ExecutionResult {
            tool_version: "tool".to_owned(),
            reported_digest: "a".to_owned(),
            elapsed_ns: 30,
            baseline_elapsed_ns: None,
            bytes_processed: 1024,
            items_processed: Some(10),
            messages_processed: None,
            artifact: None,
        },
        ExecutionResult {
            tool_version: "tool".to_owned(),
            reported_digest: "a".to_owned(),
            elapsed_ns: 20,
            baseline_elapsed_ns: None,
            bytes_processed: 1024,
            items_processed: Some(10),
            messages_processed: None,
            artifact: None,
        },
    ]);

    assert_eq!(stats.median_elapsed_ns, 20);
    assert_eq!(stats.min_elapsed_ns, 10);
    assert_eq!(stats.max_elapsed_ns, 30);
    assert_eq!(stats.median_absolute_deviation_ns, 10);
    assert!(stats.mean_elapsed_ns > 0.0);
    assert!(stats.stddev_elapsed_ns > 0.0);
    assert!(stats.coefficient_of_variation > 0.0);
    assert!(stats.relative_span_percent > 0.0);
    assert!(stats.noisy);
}

#[test]
fn regression_check_passes_growth_within_threshold() {
    let baseline = regression_report([regression_scenario_report(
        "po-parse/mixed-10000/ferrocat-owned",
        100_000_000,
        false,
    )]);
    let current = regression_report([regression_scenario_report(
        "po-parse/mixed-10000/ferrocat-owned",
        110_000_000,
        false,
    )]);

    let report = compare_regression_reports(&baseline, &current, 20.0).expect("regression report");

    assert!(!report.has_failures());
    assert_eq!(report.passed.len(), 1);
    assert!(report.render().contains("result: PASS"));
}

#[test]
fn regression_check_flags_meaningful_slowdown() {
    let baseline = regression_report([regression_scenario_report(
        "po-update/catalog-icu-heavy/ferrocat",
        100_000_000,
        false,
    )]);
    let current = regression_report([regression_scenario_report(
        "po-update/catalog-icu-heavy/ferrocat",
        130_000_000,
        false,
    )]);

    let report = compare_regression_reports(&baseline, &current, 20.0).expect("regression report");

    assert!(report.has_failures());
    assert_eq!(report.failures.len(), 1);
    assert!(
        report
            .render()
            .contains("FAIL po-update/catalog-icu-heavy/ferrocat")
    );
}

#[test]
fn regression_check_skips_noisy_scenarios() {
    let baseline = regression_report([regression_scenario_report(
        "icu-parse/icu-nested-1000/ferrocat",
        100_000_000,
        false,
    )]);
    let current = regression_report([regression_scenario_report(
        "icu-parse/icu-nested-1000/ferrocat",
        180_000_000,
        true,
    )]);

    let report = compare_regression_reports(&baseline, &current, 20.0).expect("regression report");

    assert!(!report.has_failures());
    assert_eq!(report.skipped_noisy.len(), 1);
    assert!(
        report
            .render()
            .contains("SKIP noisy icu-parse/icu-nested-1000/ferrocat")
    );
}

#[test]
fn regression_check_skips_semantic_digest_changes() {
    let baseline = regression_report([regression_scenario_report_with_digest(
        "po-parse/mixed-10000/ferrocat-owned",
        100_000_000,
        false,
        "digest-a",
    )]);
    let current = regression_report([regression_scenario_report_with_digest(
        "po-parse/mixed-10000/ferrocat-owned",
        140_000_000,
        false,
        "digest-b",
    )]);

    let report = compare_regression_reports(&baseline, &current, 20.0).expect("regression report");

    assert!(!report.has_failures());
    assert_eq!(report.skipped_semantics_changed.len(), 1);
    assert!(
        report
            .render()
            .contains("SKIP semantics-changed po-parse/mixed-10000/ferrocat-owned")
    );
}

#[test]
fn regression_check_fails_when_baseline_scenario_disappears() {
    let baseline = regression_report([regression_scenario_report(
        "po-parse/mixed-10000/ferrocat-borrowed",
        100_000_000,
        false,
    )]);
    let current = regression_report([]);

    let report = compare_regression_reports(&baseline, &current, 20.0).expect("regression report");

    assert!(report.has_failures());
    assert_eq!(
        report.missing_current,
        vec!["po-parse/mixed-10000/ferrocat-borrowed"]
    );
    assert!(
        report
            .render()
            .contains("FAIL missing-current po-parse/mixed-10000/ferrocat-borrowed")
    );
}

#[test]
fn round_robin_schedule_interleaves_scenarios_by_round() {
    assert_eq!(round_robin_schedule(&[2, 1, 3]), vec![0, 1, 2, 0, 2, 2]);
}

#[test]
fn calibrate_iterations_uses_median_probe_elapsed() {
    let iterations = calibrate_iterations(250, &[10_000_000, 20_000_000, 90_000_000]);

    assert_eq!(iterations, 13);
}

#[test]
fn profile_loads_serious_v1() {
    let workspace = workspace_root().expect("workspace");
    let profile = BenchmarkProfile::load(&workspace, "serious-v1").expect("profile");
    assert_eq!(profile.name, "serious-v1");
    assert!(!profile.scenarios.is_empty());
}

#[test]
fn profile_loads_gettext_compat_v1() {
    let workspace = workspace_root().expect("workspace");
    let profile = BenchmarkProfile::load(&workspace, "gettext-compat-v1").expect("profile");
    assert_eq!(profile.name, "gettext-compat-v1");
    assert!(!profile.scenarios.is_empty());
}

#[test]
fn profile_loads_gettext_official_v1() {
    let workspace = workspace_root().expect("workspace");
    let profile = BenchmarkProfile::load(&workspace, "gettext-official-v1").expect("profile");
    assert_eq!(profile.name, "gettext-official-v1");
    assert!(!profile.scenarios.is_empty());
}

#[test]
fn profile_loads_gettext_official_quick_v1() {
    let workspace = workspace_root().expect("workspace");
    let profile = BenchmarkProfile::load(&workspace, "gettext-official-quick-v1").expect("profile");
    assert_eq!(profile.name, "gettext-official-quick-v1");
    assert!(!profile.scenarios.is_empty());
    assert_eq!(profile.tool_requirement(), ToolRequirement::External);
}

#[test]
fn profile_loads_rust_scheduled_v1_without_external_tools() {
    let workspace = workspace_root().expect("workspace");
    let profile = BenchmarkProfile::load(&workspace, "rust-scheduled-v1").expect("profile");
    assert_eq!(profile.name, "rust-scheduled-v1");
    assert!(!profile.scenarios.is_empty());
    assert_eq!(profile.tool_requirement(), ToolRequirement::RustOnly);
    let operations = profile
        .scenarios
        .iter()
        .map(|scenario| scenario.operation.as_str())
        .collect::<BTreeSet<_>>();
    let expected = [
        "combine-catalogs",
        "audit-catalogs",
        "measure-catalog-coverage",
        "review-catalogs",
        "compile-catalog-artifact",
    ];
    let missing = expected
        .into_iter()
        .filter(|operation| !operations.contains(operation))
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "missing PR workflow operations: {missing:?}"
    );
}

#[test]
fn catalog_workflow_operations_produce_validated_artifacts() {
    let workspace = workspace_root().expect("workspace");
    let operations = [
        ("combine-catalogs", "ferrocat-combine-catalogs"),
        ("audit-catalogs", "ferrocat-audit-catalogs"),
        (
            "measure-catalog-coverage",
            "ferrocat-measure-catalog-coverage",
        ),
        ("review-catalogs", "ferrocat-review-catalogs"),
        (
            "compile-catalog-artifact",
            "ferrocat-compile-catalog-artifact",
        ),
    ];

    for (operation, implementation) in operations {
        let scenario = BenchmarkScenario {
            id: format!("test/{operation}"),
            comparison_group: format!("test/{operation}"),
            workload: operation.to_owned(),
            operation: operation.to_owned(),
            fixture: "catalog-modern-de-1000".to_owned(),
            implementation: implementation.to_owned(),
            warmup_runs: 0,
            measured_runs: 1,
            minimum_sample_millis: Some(1),
        };
        let prepared = PreparedScenario::prepare(&workspace, std::slice::from_ref(&scenario))
            .expect("prepare catalog workflow");
        let result = execute_scenario(&workspace, &prepared, &scenario, 1, true)
            .expect("run catalog workflow");
        let validated = prepared
            .validate(&result)
            .expect("validate catalog workflow");

        assert_eq!(result.reported_digest, validated, "operation {operation}");
    }
}

#[test]
fn profile_loads_gettext_workflows_ecosystem_v1() {
    let workspace = workspace_root().expect("workspace");
    let profile =
        BenchmarkProfile::load(&workspace, "gettext-workflows-ecosystem-v1").expect("profile");
    assert_eq!(profile.name, "gettext-workflows-ecosystem-v1");
    assert!(!profile.scenarios.is_empty());
}

#[test]
fn profile_loads_catalog_order_default_v1_without_external_tools() {
    let workspace = workspace_root().expect("workspace");
    let profile = BenchmarkProfile::load(&workspace, "catalog-order-default-v1").expect("profile");
    assert_eq!(profile.name, "catalog-order-default-v1");
    assert_eq!(profile.scenarios.len(), 1);
    assert_eq!(profile.tool_requirement(), ToolRequirement::RustOnly);
}

#[test]
fn profile_loads_storage_formats_v1() {
    let workspace = workspace_root().expect("workspace");
    let profile = BenchmarkProfile::load(&workspace, "storage-formats-v1").expect("profile");
    assert_eq!(profile.name, "storage-formats-v1");
    assert!(!profile.scenarios.is_empty());
}

#[test]
fn adapter_response_schema_accepts_optional_artifacts() {
    let response = serde_json::from_str::<AdapterResponse>(
        r#"{
                "implementation":"polib",
                "workload":"po-parse",
                "fixture":"mixed-1000",
                "success":true,
                "semantic_digest":"abc",
                "elapsed_ns":123,
                "bytes_processed":456,
                "items_processed":10,
                "messages_processed":null,
                "tool_version":"polib 1.0",
                "po_summary":{"headers":[],"items":[]},
                "icu_summary":null,
                "po_output_path":null
            }"#,
    )
    .expect("schema");

    assert_eq!(response.implementation, "polib");
    assert!(response.po_summary.is_some());
}

#[test]
fn benchmark_environment_reports_missing_tools() {
    let workspace = workspace_root().expect("workspace");
    let error = BenchmarkEnvironment::detect(
        &workspace,
        Some(OsStr::new("/definitely-missing")),
        ToolRequirement::External,
    )
    .expect_err("expected failure");
    assert!(error.contains("benchmark environment verification failed"));
}

#[test]
fn run_profile_supports_internal_compare_groups() {
    let workspace = workspace_root().expect("workspace");
    let profile = BenchmarkProfile {
        name: "test-internal".to_owned(),
        minimum_sample_millis: 1,
        scenarios: vec![
            BenchmarkScenario {
                id: "po-parse/mixed-1000/owned".to_owned(),
                comparison_group: "po-parse/mixed-1000".to_owned(),
                workload: "po-parse".to_owned(),
                operation: "parse".to_owned(),
                fixture: "mixed-1000".to_owned(),
                implementation: "ferrocat-parse".to_owned(),
                warmup_runs: 1,
                measured_runs: 2,
                minimum_sample_millis: Some(1),
            },
            BenchmarkScenario {
                id: "po-parse/mixed-1000/borrowed".to_owned(),
                comparison_group: "po-parse/mixed-1000".to_owned(),
                workload: "po-parse".to_owned(),
                operation: "parse".to_owned(),
                fixture: "mixed-1000".to_owned(),
                implementation: "ferrocat-parse-borrowed".to_owned(),
                warmup_runs: 1,
                measured_runs: 2,
                minimum_sample_millis: Some(1),
            },
        ],
    };
    let environment = BenchmarkEnvironment {
        git_sha: "test-sha".to_owned(),
        system_label: "Test CPU (16 GB RAM, TestOS arm64)".to_owned(),
        os: "test-os".to_owned(),
        cpu_model: "test-cpu".to_owned(),
        memory_bytes: 16 * 1024 * 1024 * 1024,
        rustc_version: "rustc test".to_owned(),
        node_version: "node test".to_owned(),
        python_version: "python test".to_owned(),
        msgmerge_version: "msgmerge test".to_owned(),
        msgcat_version: "msgcat test".to_owned(),
        node_adapter_version: "node adapters".to_owned(),
        python_adapter_version: "python adapters".to_owned(),
        python_program: PathBuf::from("python3"),
    };

    let report = run_profile(&workspace, &environment, &profile).expect("report");
    assert_eq!(report.profile, "test-internal");
    assert_eq!(report.scenarios.len(), 2);
    assert_eq!(
        report.scenarios[0].semantic_digest,
        report.scenarios[1].semantic_digest
    );
}

#[test]
fn format_memory_label_uses_clean_gigabyte_counts() {
    assert_eq!(format_memory_label(32 * 1024 * 1024 * 1024), "32 GB RAM");
}

#[test]
fn build_system_label_combines_cpu_memory_and_os() {
    assert_eq!(
        build_system_label_with_os("Apple M1 Pro", 32 * 1024 * 1024 * 1024, "macOS arm64"),
        "Apple M1 Pro (32 GB RAM, macOS arm64)"
    );
}

#[test]
fn owned_and_borrowed_match_on_gettext_plural_fixture() {
    let fixture = crate::fixtures::fixture_by_name("gettext-commerce-pl-1000").expect("fixture");
    let owned = ferrocat_po::parse_po(fixture.content()).expect("owned parse");
    let borrowed = ferrocat_po::parse_po_borrowed(fixture.content()).expect("borrowed parse");

    let owned_summary = PoSemanticSummary::from_po_file(&owned);
    let borrowed_summary = PoSemanticSummary::from_borrowed_po_file(&borrowed);

    assert_eq!(
        canonical_json_string(&owned_summary).expect("owned json"),
        canonical_json_string(&borrowed_summary).expect("borrowed json")
    );
}

/// Legacy owned-parse conversion, kept as the equivalence reference for the
/// borrowed template ingestion path used by the timed benchmark loops.
fn legacy_extracted_messages_from_template(template: &PoFile) -> Vec<ExtractedMessage> {
    template
        .items
        .iter()
        .filter(|item| !item.obsolete)
        .map(|item| {
            let comments: Vec<String> = item.extracted_comments.iter().cloned().collect();
            let origin: Vec<CatalogOrigin> = item
                .references
                .iter()
                .map(|reference| crate::fixtures::parse_origin(reference))
                .collect();
            if let Some(msgid_plural) = item.msgid_plural.as_deref() {
                ExtractedMessage::Plural(ExtractedPluralMessage {
                    msgid: item.msgid.clone(),
                    msgctxt: item.msgctxt.clone(),
                    source: PluralSource {
                        one: Some(item.msgid.clone()),
                        other: msgid_plural.to_owned(),
                    },
                    comments,
                    origin,
                    placeholders: BTreeMap::default(),
                })
            } else {
                ExtractedMessage::Singular(ExtractedSingularMessage {
                    msgid: item.msgid.clone(),
                    msgctxt: item.msgctxt.clone(),
                    comments,
                    origin,
                    placeholders: BTreeMap::default(),
                })
            }
        })
        .collect()
}

/// Legacy owned-parse conversion for the merge input, kept as the equivalence
/// reference for [`merge_inputs_from_template`].
fn legacy_merge_inputs_from_template(template: &PoFile) -> Vec<MergeMessageInput<'_>> {
    template
        .items
        .iter()
        .map(|item| MergeMessageInput {
            msgctxt: item.msgctxt.as_deref().map(Cow::Borrowed),
            msgid: Cow::Borrowed(item.msgid.as_str()),
            msgid_plural: item.msgid_plural.as_deref().map(Cow::Borrowed),
            references: item
                .references
                .iter()
                .map(|value| Cow::Borrowed(value.as_str()))
                .collect(),
            extracted_comments: item
                .extracted_comments
                .iter()
                .map(|value| Cow::Borrowed(value.as_str()))
                .collect(),
            flags: item
                .flags
                .iter()
                .map(|value| Cow::Borrowed(value.as_str()))
                .collect(),
        })
        .collect()
}

#[test]
fn borrowed_template_ingestion_matches_owned_conversion() {
    for fixture_name in [
        "gettext-ui-de-1000",
        "gettext-commerce-pl-1000",
        "mixed-1000",
    ] {
        let fixture = merge_fixture_by_name(fixture_name).expect("merge fixture");
        let owned_fixture = OwnedMergeFixture::from_fixture(&fixture);
        let template_pot = owned_fixture.template_pot.as_str();

        let owned = parse_po(template_pot).expect("owned template parse");
        let borrowed = parse_po_borrowed(template_pot).expect("borrowed template parse");
        assert_eq!(
            extracted_messages_from_template(borrowed),
            legacy_extracted_messages_from_template(&owned),
            "extraction input drifted for {fixture_name}"
        );

        let borrowed = parse_po_borrowed(template_pot).expect("borrowed template parse");
        assert_eq!(
            merge_inputs_from_template(borrowed),
            legacy_merge_inputs_from_template(&owned),
            "merge input drifted for {fixture_name}"
        );
    }
}

#[test]
fn origin_from_reference_matches_parse_origin() {
    for reference in [
        "src/app.rs:10",
        "src/app.rs",
        "src/app.rs:",
        "src/app.rs:abc",
        "10",
        "",
        "a:b:12",
    ] {
        assert_eq!(
            origin_from_reference(Cow::Borrowed(reference)),
            crate::fixtures::parse_origin(reference),
            "borrowed origin drifted for {reference:?}"
        );
        assert_eq!(
            origin_from_reference(Cow::Owned(reference.to_owned())),
            crate::fixtures::parse_origin(reference),
            "owned origin drifted for {reference:?}"
        );
    }
}

fn regression_report<const N: usize>(scenarios: [ScenarioReport; N]) -> CompareReport {
    CompareReport {
        profile: "rust-scheduled-v1".to_owned(),
        generated_at: "2026-06-23T00:00:00Z".to_owned(),
        reference_host_policy: "test".to_owned(),
        environment: EnvironmentMetadata {
            git_sha: "test".to_owned(),
            system_label: "test".to_owned(),
            os: "test".to_owned(),
            cpu_model: "test".to_owned(),
            memory_bytes: 1,
            rustc_version: "rustc test".to_owned(),
            node_version: "node test".to_owned(),
            python_version: "python test".to_owned(),
            msgmerge_version: "msgmerge test".to_owned(),
            msgcat_version: "msgcat test".to_owned(),
            node_adapter_version: "node adapters test".to_owned(),
            python_adapter_version: "python adapters test".to_owned(),
        },
        scenarios: scenarios.into_iter().collect(),
    }
}

fn regression_scenario_report(id: &str, median_elapsed_ns: u128, noisy: bool) -> ScenarioReport {
    regression_scenario_report_with_digest(id, median_elapsed_ns, noisy, "digest")
}

fn regression_scenario_report_with_digest(
    id: &str,
    median_elapsed_ns: u128,
    noisy: bool,
    semantic_digest: &str,
) -> ScenarioReport {
    ScenarioReport {
        id: id.to_owned(),
        comparison_group: id
            .rsplit_once('/')
            .map_or(id, |(group, _)| group)
            .to_owned(),
        workload: "test".to_owned(),
        operation: "test".to_owned(),
        fixture: "test".to_owned(),
        implementation: "ferrocat-test".to_owned(),
        tool_version: "ferrocat@test".to_owned(),
        iterations_per_sample: 1,
        warmup_runs: 0,
        measured_runs: 3,
        semantic_digest: semantic_digest.to_owned(),
        baseline_strategy: None,
        baseline_fixture: None,
        statistics: regression_statistics(median_elapsed_ns, noisy),
        samples: Vec::new(),
    }
}

fn regression_statistics(median_elapsed_ns: u128, noisy: bool) -> ScenarioStatistics {
    ScenarioStatistics {
        median_elapsed_ns,
        mean_elapsed_ns: f64_from_u128(median_elapsed_ns),
        min_elapsed_ns: median_elapsed_ns,
        max_elapsed_ns: median_elapsed_ns,
        stddev_elapsed_ns: 0.0,
        median_absolute_deviation_ns: 0,
        coefficient_of_variation: if noisy { 0.06 } else { 0.01 },
        relative_span_percent: if noisy { 11.0 } else { 1.0 },
        noisy,
        median_mib_per_sec: 1.0,
        median_units_per_sec: 1.0,
        median_baseline_elapsed_ns: None,
        median_adjusted_elapsed_ns: None,
        median_adjusted_mib_per_sec: None,
        median_adjusted_units_per_sec: None,
    }
}

fn regression_scenario_report_with_iterations(
    id: &str,
    median_elapsed_ns: u128,
    iterations_per_sample: usize,
) -> ScenarioReport {
    ScenarioReport {
        iterations_per_sample,
        ..regression_scenario_report(id, median_elapsed_ns, false)
    }
}

#[test]
fn regression_check_normalizes_by_iterations_per_sample() {
    // Calibration guard (PR #155): same work (matching digest), but the
    // harness picked 29 iterations/sample for the baseline and 57 for the
    // current run. Each operation got faster, so the raw median_elapsed_ns
    // is larger while throughput improved. The check must read an
    // improvement, not a regression.
    let baseline = regression_report([regression_scenario_report_with_iterations(
        "icu-parse/icu-nested-1000/ferrocat",
        110_429_000,
        29,
    )]);
    let current = regression_report([regression_scenario_report_with_iterations(
        "icu-parse/icu-nested-1000/ferrocat",
        142_051_000,
        57,
    )]);

    let report = compare_regression_reports(&baseline, &current, 20.0).expect("regression report");

    assert!(
        !report.has_failures(),
        "calibration-only change must not fail: {}",
        report.render()
    );
    // per iteration: 3.808 ms -> 2.492 ms, about a 35% improvement
    assert_eq!(report.passed.len(), 1);
    assert!(
        report.passed[0].regression_percent < -30.0,
        "expected improvement near -35%, got {:.2}%",
        report.passed[0].regression_percent
    );
}

#[test]
fn regression_check_flags_per_iteration_slowdown_across_calibrations() {
    // The flip side: a genuine per-operation slowdown must still trip the
    // gate even when calibration hides it in the raw sample time. Baseline
    // 2 ms/iter (10 iters), current 3 ms/iter (5 iters) -> +50% per
    // iteration, although the raw sample time shrank from 20 ms to 15 ms and
    // the old raw comparison would have waved it through.
    let baseline = regression_report([regression_scenario_report_with_iterations(
        "icu-parse/icu-nested-1000/ferrocat",
        20_000_000,
        10,
    )]);
    let current = regression_report([regression_scenario_report_with_iterations(
        "icu-parse/icu-nested-1000/ferrocat",
        15_000_000,
        5,
    )]);

    let report = compare_regression_reports(&baseline, &current, 20.0).expect("regression report");

    assert!(
        report.has_failures(),
        "per-iteration slowdown must fail: {}",
        report.render()
    );
}
