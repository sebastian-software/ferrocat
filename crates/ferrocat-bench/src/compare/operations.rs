//! Scenario preparation and benchmark operation execution.

use super::*;

impl PreparedScenario {
    #[expect(
        clippy::too_many_lines,
        reason = "Scenario preparation is intentionally centralized so fixture setup remains deterministic."
    )]
    pub(super) fn prepare(
        workspace: &Path,
        scenarios: &[BenchmarkScenario],
    ) -> Result<Self, String> {
        let Some(first) = scenarios.first() else {
            return Err("cannot prepare empty benchmark scenario group".to_owned());
        };
        for scenario in scenarios {
            if scenario.operation != first.operation || scenario.fixture != first.fixture {
                return Err(format!(
                    "comparison group {} mixes incompatible operations or fixtures",
                    first.comparison_group
                ));
            }
        }

        let tempdir = tempfile::Builder::new()
            .prefix("ferrocat-compare-")
            .tempdir_in(workspace.join("target"))
            .map_err(|error| format!("failed to create compare tempdir: {error}"))?;

        match first.operation.as_str() {
            "parse" | "stringify" => {
                let fixture = load_fixture(&first.fixture)?;
                let input_path = tempdir.path().join("input.po");
                fs::write(&input_path, fixture.content()).map_err(|error| {
                    format!(
                        "failed to write fixture input {}: {error}",
                        input_path.display()
                    )
                })?;
                let po_file = parse_po(fixture.content())
                    .map_err(|error| format!("failed to parse fixture: {error}"))?;
                Ok(Self {
                    operation: first.operation.clone(),
                    fixture: first.fixture.clone(),
                    tempdir,
                    po_input_path: Some(input_path),
                    icu_messages_path: None,
                    existing_po_path: None,
                    pot_path: None,
                    po_content: Some(fixture.content().to_owned()),
                    catalog_fcl_content: None,
                    po_file: Some(po_file),
                    merge_fixture: None,
                    icu_messages: None,
                    catalog_workflow: None,
                })
            }
            "parse-catalog" => {
                let fixture = load_fixture(&first.fixture)?;
                let po_input_path = tempdir.path().join("catalog-input.po");
                fs::write(&po_input_path, fixture.content()).map_err(|error| {
                    format!(
                        "failed to write catalog fixture input {}: {error}",
                        po_input_path.display()
                    )
                })?;
                let po_content = fixture.content().to_owned();
                // Render an equivalent FCL catalog so the storage-format comparison
                // can parse the same logical catalog as both PO and FCL.
                let mut options =
                    ParseCatalogOptions::new(&po_content, "en").with_mode(CatalogMode::IcuPo);
                let fixture_locale = fixture_locale(&first.fixture);
                if let Some(locale) = fixture_locale.as_deref() {
                    options = options.with_locale(locale);
                }
                let parsed = parse_catalog(options)
                    .map_err(|error| format!("failed to parse catalog fixture: {error}"))?;
                let fcl_content = crate::render_fcl_catalog(&parsed);
                Ok(Self {
                    operation: first.operation.clone(),
                    fixture: first.fixture.clone(),
                    tempdir,
                    po_input_path: Some(po_input_path),
                    icu_messages_path: None,
                    existing_po_path: None,
                    pot_path: None,
                    po_content: Some(po_content),
                    catalog_fcl_content: Some(fcl_content),
                    po_file: None,
                    merge_fixture: None,
                    icu_messages: None,
                    catalog_workflow: None,
                })
            }
            "merge" | "update-catalog" => {
                let fixture = load_merge_fixture(&first.fixture)?;
                let existing_po_path = tempdir.path().join("existing.po");
                fs::write(&existing_po_path, fixture.existing_po()).map_err(|error| {
                    format!(
                        "failed to write merge fixture input {}: {error}",
                        existing_po_path.display()
                    )
                })?;
                let pot_path = if first.operation == "merge"
                    || scenarios
                        .iter()
                        .any(|scenario| !scenario.implementation.starts_with("ferrocat-"))
                {
                    let pot_path = tempdir.path().join("template.pot");
                    let pot = build_merge_pot(&fixture);
                    fs::write(&pot_path, pot).map_err(|error| {
                        format!(
                            "failed to write merge template {}: {error}",
                            pot_path.display()
                        )
                    })?;
                    Some(pot_path)
                } else {
                    None
                };
                Ok(Self {
                    operation: first.operation.clone(),
                    fixture: first.fixture.clone(),
                    tempdir,
                    po_input_path: None,
                    icu_messages_path: None,
                    existing_po_path: Some(existing_po_path),
                    pot_path,
                    po_content: None,
                    catalog_fcl_content: None,
                    po_file: None,
                    merge_fixture: Some(OwnedMergeFixture::from_fixture(&fixture)),
                    icu_messages: None,
                    catalog_workflow: None,
                })
            }
            "parse-icu" => {
                let fixture = load_icu_fixture(&first.fixture)?;
                let messages_path = tempdir.path().join("messages.json");
                let messages = fixture.messages().to_vec();
                let serialized = serde_json::to_string_pretty(&messages)
                    .map_err(|error| format!("failed to serialize ICU messages: {error}"))?;
                fs::write(&messages_path, serialized).map_err(|error| {
                    format!(
                        "failed to write ICU fixture input {}: {error}",
                        messages_path.display()
                    )
                })?;
                Ok(Self {
                    operation: first.operation.clone(),
                    fixture: first.fixture.clone(),
                    tempdir,
                    po_input_path: None,
                    icu_messages_path: Some(messages_path),
                    existing_po_path: None,
                    pot_path: None,
                    po_content: None,
                    catalog_fcl_content: None,
                    po_file: None,
                    merge_fixture: None,
                    icu_messages: Some(messages),
                    catalog_workflow: None,
                })
            }
            "combine-catalogs"
            | "audit-catalogs"
            | "measure-catalog-coverage"
            | "review-catalogs"
            | "compile-catalog-artifact" => {
                let fixture = load_fixture(&first.fixture)?;
                let catalog_workflow = CatalogWorkflowFixture::from_fixture(&fixture)?;
                Ok(Self {
                    operation: first.operation.clone(),
                    fixture: first.fixture.clone(),
                    tempdir,
                    po_input_path: None,
                    icu_messages_path: None,
                    existing_po_path: None,
                    pot_path: None,
                    po_content: None,
                    catalog_fcl_content: None,
                    po_file: None,
                    merge_fixture: None,
                    icu_messages: None,
                    catalog_workflow: Some(catalog_workflow),
                })
            }
            other => Err(format!("unsupported benchmark operation: {other}")),
        }
    }

    pub(super) fn prepare_cli_baseline(
        workspace: &Path,
        scenario: &BenchmarkScenario,
    ) -> Result<Option<CliBaselineScenario>, String> {
        match scenario.implementation.as_str() {
            "msgcat" => {
                let tempdir = tempfile::Builder::new()
                    .prefix("ferrocat-cli-baseline-")
                    .tempdir_in(workspace.join("target"))
                    .map_err(|error| format!("failed to create cli baseline tempdir: {error}"))?;
                let locale = fixture_locale(&scenario.fixture);
                let content = build_cli_baseline_po(locale.as_deref());
                let input_path = tempdir.path().join("baseline.po");
                fs::write(&input_path, &content).map_err(|error| {
                    format!(
                        "failed to write cli baseline input {}: {error}",
                        input_path.display()
                    )
                })?;
                Ok(Some(CliBaselineScenario {
                    label: format!("header-only-po:{}", locale.as_deref().unwrap_or("default")),
                    prepared: Self {
                        operation: scenario.operation.clone(),
                        fixture: format!("cli-baseline-msgcat-{}", scenario.fixture),
                        tempdir,
                        po_input_path: Some(input_path),
                        icu_messages_path: None,
                        existing_po_path: None,
                        pot_path: None,
                        po_content: Some(content),
                        catalog_fcl_content: None,
                        po_file: None,
                        merge_fixture: None,
                        icu_messages: None,
                        catalog_workflow: None,
                    },
                }))
            }
            "msgmerge" => {
                let tempdir = tempfile::Builder::new()
                    .prefix("ferrocat-cli-baseline-")
                    .tempdir_in(workspace.join("target"))
                    .map_err(|error| format!("failed to create cli baseline tempdir: {error}"))?;
                let locale = fixture_locale(&scenario.fixture);
                let existing = build_cli_baseline_po(locale.as_deref());
                let pot = build_cli_baseline_pot();
                let existing_po_path = tempdir.path().join("baseline-existing.po");
                let pot_path = tempdir.path().join("baseline-template.pot");
                fs::write(&existing_po_path, &existing).map_err(|error| {
                    format!(
                        "failed to write cli baseline existing {}: {error}",
                        existing_po_path.display()
                    )
                })?;
                fs::write(&pot_path, &pot).map_err(|error| {
                    format!(
                        "failed to write cli baseline template {}: {error}",
                        pot_path.display()
                    )
                })?;
                Ok(Some(CliBaselineScenario {
                    label: format!(
                        "header-only-merge:{}",
                        locale.as_deref().unwrap_or("default")
                    ),
                    prepared: Self {
                        operation: scenario.operation.clone(),
                        fixture: format!("cli-baseline-msgmerge-{}", scenario.fixture),
                        tempdir,
                        po_input_path: None,
                        icu_messages_path: None,
                        existing_po_path: Some(existing_po_path),
                        pot_path: Some(pot_path),
                        po_content: None,
                        catalog_fcl_content: None,
                        po_file: None,
                        merge_fixture: None,
                        icu_messages: None,
                        catalog_workflow: None,
                    },
                }))
            }
            _ => Ok(None),
        }
    }

    pub(super) fn validate(&self, result: &ExecutionResult) -> Result<String, String> {
        let digest = match self.operation.as_str() {
            "parse" => match result.artifact.as_ref() {
                Some(ExecutionArtifact::PoSummary(summary)) => digest_summary(summary)?,
                _ => {
                    return Err(format!(
                        "scenario {} expected PO summary artifact",
                        self.fixture
                    ));
                }
            },
            "parse-catalog" => match result.artifact.as_ref() {
                Some(ExecutionArtifact::CatalogSummary(summary)) => digest_summary(summary)?,
                _ => {
                    return Err(format!(
                        "scenario {} expected catalog summary artifact",
                        self.fixture
                    ));
                }
            },
            "stringify" | "merge" | "update-catalog" => {
                let content = match result.artifact.as_ref() {
                    Some(ExecutionArtifact::RenderedPo(content)) => Cow::Borrowed(content.as_str()),
                    Some(ExecutionArtifact::RenderedPoPath(path)) => {
                        Cow::Owned(fs::read_to_string(path).map_err(|error| {
                            format!(
                                "failed to read rendered PO output {}: {error}",
                                path.display()
                            )
                        })?)
                    }
                    _ => {
                        return Err(format!(
                            "scenario {} expected rendered PO artifact for {}",
                            self.fixture, self.operation
                        ));
                    }
                };
                let parsed = parse_po(&content)
                    .map_err(|error| format!("rendered output did not parse as PO: {error}"))?;
                digest_summary(&PoSemanticSummary::from_po_file(&parsed))?
            }
            "combine-catalogs" => {
                let content = match result.artifact.as_ref() {
                    Some(ExecutionArtifact::RenderedPo(content)) => content,
                    _ => {
                        return Err(format!(
                            "scenario {} expected combined PO artifact",
                            self.fixture
                        ));
                    }
                };
                let parsed = parse_po(content)
                    .map_err(|error| format!("combined output did not parse as PO: {error}"))?;
                digest_summary(&PoSemanticSummary::from_po_file(&parsed))?
            }
            "audit-catalogs"
            | "measure-catalog-coverage"
            | "review-catalogs"
            | "compile-catalog-artifact" => match result.artifact.as_ref() {
                Some(ExecutionArtifact::JsonSummary(summary)) => digest_summary(summary)?,
                _ => {
                    return Err(format!(
                        "scenario {} expected JSON summary artifact for {}",
                        self.fixture, self.operation
                    ));
                }
            },
            "parse-icu" => match result.artifact.as_ref() {
                Some(ExecutionArtifact::IcuSummary(summary)) => digest_summary(summary)?,
                _ => {
                    return Err(format!(
                        "scenario {} expected ICU summary artifact",
                        self.fixture
                    ));
                }
            },
            other => return Err(format!("unsupported validation operation: {other}")),
        };
        Ok(digest)
    }

    pub(super) fn run_internal_parse(
        &self,
        iterations: usize,
        borrowed: bool,
    ) -> Result<ExecutionResult, String> {
        let input = self
            .po_content
            .as_deref()
            .ok_or_else(|| "internal parse requires PO content".to_owned())?;
        // Time only the parse; build the digest summary once outside the loop so
        // the measured path is parsing alone, matching the external adapters
        // (which also reparse for the summary outside the timed loop).
        let (elapsed, summary) = if borrowed {
            let mut last = None;
            let start = Instant::now();
            for _ in 0..iterations {
                last = Some(
                    parse_po_borrowed(input)
                        .map_err(|error| format!("borrowed parse failed: {error}"))?,
                );
            }
            let elapsed = start.elapsed();
            let parsed = last.ok_or_else(|| "internal parse produced no result".to_owned())?;
            (elapsed, PoSemanticSummary::from_borrowed_po_file(&parsed))
        } else {
            let mut last = None;
            let start = Instant::now();
            for _ in 0..iterations {
                last =
                    Some(parse_po(input).map_err(|error| format!("owned parse failed: {error}"))?);
            }
            let elapsed = start.elapsed();
            let parsed = last.ok_or_else(|| "internal parse produced no result".to_owned())?;
            (elapsed, PoSemanticSummary::from_po_file(&parsed))
        };
        let digest = digest_summary(&summary)?;
        Ok(ExecutionResult {
            tool_version: INTERNAL_TOOL_VERSION.to_owned(),
            reported_digest: digest,
            elapsed_ns: elapsed.as_nanos(),
            baseline_elapsed_ns: None,
            bytes_processed: (input.len() * iterations) as u64,
            items_processed: summary
                .items
                .len()
                .checked_mul(iterations)
                .map(|value| value as u64),
            messages_processed: None,
            artifact: Some(ExecutionArtifact::PoSummary(summary)),
        })
    }

    pub(super) fn run_internal_parse_catalog(
        &self,
        iterations: usize,
    ) -> Result<ExecutionResult, String> {
        let content = self
            .po_content
            .as_deref()
            .ok_or_else(|| "internal parse-catalog-po requires PO content".to_owned())?;
        self.run_internal_parse_catalog_with(iterations, content, CatalogMode::IcuPo)
    }

    pub(super) fn run_internal_parse_catalog_fcl(
        &self,
        iterations: usize,
    ) -> Result<ExecutionResult, String> {
        let content = self
            .catalog_fcl_content
            .as_deref()
            .ok_or_else(|| "internal parse-catalog-fcl requires FCL content".to_owned())?;
        self.run_internal_parse_catalog_with(iterations, content, CatalogMode::IcuFcl)
    }

    pub(super) fn run_internal_parse_catalog_with(
        &self,
        iterations: usize,
        content: &str,
        mode: CatalogMode,
    ) -> Result<ExecutionResult, String> {
        let locale = fixture_locale(&self.fixture);
        let bytes_per_iteration = content.len();

        // Time only parse_catalog; build the digest summary once outside the loop.
        let mut last_parsed = None;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut options = ParseCatalogOptions::new(content, "en").with_mode(mode);
            if let Some(locale) = locale.as_deref() {
                options = options.with_locale(locale);
            }
            last_parsed = Some(
                parse_catalog(options).map_err(|error| format!("parse_catalog failed: {error}"))?,
            );
        }
        let elapsed = start.elapsed();
        let parsed =
            last_parsed.ok_or_else(|| "internal parse-catalog produced no result".to_owned())?;
        let summary = CatalogSemanticSummary::from_parsed_catalog(parsed)?;
        let digest = digest_summary(&summary)?;
        Ok(ExecutionResult {
            tool_version: INTERNAL_TOOL_VERSION.to_owned(),
            reported_digest: digest,
            elapsed_ns: elapsed.as_nanos(),
            baseline_elapsed_ns: None,
            bytes_processed: (bytes_per_iteration * iterations) as u64,
            items_processed: None,
            messages_processed: Some((summary.messages.len() * iterations) as u64),
            artifact: Some(ExecutionArtifact::CatalogSummary(summary)),
        })
    }

    pub(super) fn run_internal_stringify(
        &self,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let po_file = self
            .po_file
            .as_ref()
            .ok_or_else(|| "internal stringify requires parsed PO file".to_owned())?;
        let mut last_rendered = None;
        let start = Instant::now();
        let mut bytes_processed = 0usize;
        for _ in 0..iterations {
            let rendered = stringify_po(po_file, &SerializeOptions::default());
            bytes_processed += rendered.len();
            last_rendered = Some(rendered);
        }
        let elapsed = start.elapsed();
        let rendered = last_rendered
            .ok_or_else(|| "internal stringify produced no rendered content".to_owned())?;
        let summary = {
            let parsed = parse_po(&rendered).map_err(|error| {
                format!("stringify validation parse failed for rendered output: {error}")
            })?;
            PoSemanticSummary::from_po_file(&parsed)
        };
        let digest = digest_summary(&summary)?;
        Ok(ExecutionResult {
            tool_version: INTERNAL_TOOL_VERSION.to_owned(),
            reported_digest: digest,
            elapsed_ns: elapsed.as_nanos(),
            baseline_elapsed_ns: None,
            bytes_processed: bytes_processed as u64,
            items_processed: summary
                .items
                .len()
                .checked_mul(iterations)
                .map(|value| value as u64),
            messages_processed: None,
            artifact: capture_artifacts.then_some(ExecutionArtifact::RenderedPo(rendered)),
        })
    }

    pub(super) fn run_internal_merge(
        &self,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let fixture = self
            .merge_fixture
            .as_ref()
            .ok_or_else(|| "internal merge requires merge fixture".to_owned())?;
        let mut last_rendered = None;
        let start = Instant::now();
        let mut bytes_processed = 0usize;
        for _ in 0..iterations {
            // Parse the same template.pot the competitors read, then merge.
            // This keeps the catalog-update comparison strictly file-to-file
            // instead of handing ferrocat pre-structured messages.
            let template = parse_po(&fixture.template_pot)
                .map_err(|error| format!("merge template parse failed: {error}"))?;
            let extracted: Vec<MergeMessageInput<'_>> = template
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
                .collect();
            let rendered = merge_catalog(&fixture.existing_po, &extracted)
                .map_err(|error| format!("merge_catalog failed: {error}"))?;
            bytes_processed += rendered.len();
            last_rendered = Some(rendered);
        }
        let elapsed = start.elapsed();
        let rendered = last_rendered
            .ok_or_else(|| "internal merge produced no rendered content".to_owned())?;
        let summary = {
            let parsed = parse_po(&rendered)
                .map_err(|error| format!("merge output did not parse: {error}"))?;
            PoSemanticSummary::from_po_file(&parsed)
        };
        let digest = digest_summary(&summary)?;
        Ok(ExecutionResult {
            tool_version: INTERNAL_TOOL_VERSION.to_owned(),
            reported_digest: digest,
            elapsed_ns: elapsed.as_nanos(),
            baseline_elapsed_ns: None,
            bytes_processed: bytes_processed as u64,
            items_processed: summary
                .items
                .len()
                .checked_mul(iterations)
                .map(|value| value as u64),
            messages_processed: None,
            artifact: capture_artifacts.then_some(ExecutionArtifact::RenderedPo(rendered)),
        })
    }

    pub(super) fn run_internal_update_catalog(
        &self,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let fixture = self
            .merge_fixture
            .as_ref()
            .ok_or_else(|| "internal update-catalog requires merge fixture".to_owned())?;
        let mut last_rendered = None;
        let start = Instant::now();
        let mut bytes_processed = 0usize;
        let locale = fixture_locale(&self.fixture);
        let mode = fixture_catalog_mode(&self.fixture);
        for _ in 0..iterations {
            let mut options = UpdateCatalogOptions::new("en", fixture.api_messages.clone())
                .with_mode(mode)
                .with_existing(fixture.existing_po.as_str());
            if let Some(locale) = locale.as_deref() {
                options = options.with_locale(locale);
            }
            let updated = update_catalog(options)
                .map_err(|error| format!("update_catalog failed: {error}"))?;
            bytes_processed += updated.content.len();
            last_rendered = Some(updated.content);
        }
        let elapsed = start.elapsed();
        let rendered = last_rendered
            .ok_or_else(|| "internal update_catalog produced no rendered content".to_owned())?;
        let summary = {
            let parsed = parse_po(&rendered)
                .map_err(|error| format!("update_catalog output did not parse: {error}"))?;
            PoSemanticSummary::from_po_file(&parsed)
        };
        let digest = digest_summary(&summary)?;
        Ok(ExecutionResult {
            tool_version: INTERNAL_TOOL_VERSION.to_owned(),
            reported_digest: digest,
            elapsed_ns: elapsed.as_nanos(),
            baseline_elapsed_ns: None,
            bytes_processed: bytes_processed as u64,
            items_processed: summary
                .items
                .len()
                .checked_mul(iterations)
                .map(|value| value as u64),
            messages_processed: None,
            artifact: capture_artifacts.then_some(ExecutionArtifact::RenderedPo(rendered)),
        })
    }

    pub(super) fn run_internal_update_catalog_file(
        &self,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let fixture = self
            .merge_fixture
            .as_ref()
            .ok_or_else(|| "internal update-catalog requires merge fixture".to_owned())?;
        let mut last_rendered = None;
        let start = Instant::now();
        let mut bytes_processed = 0usize;
        let locale = fixture_locale(&self.fixture);
        let mode = fixture_catalog_mode(&self.fixture);
        for _ in 0..iterations {
            // Parse the same template.pot the competitors read and build the
            // extraction input from it inside the timed loop. This keeps the
            // catalog-update comparison strictly file-to-file instead of
            // handing ferrocat pre-structured messages.
            let template = parse_po(&fixture.template_pot)
                .map_err(|error| format!("update template parse failed: {error}"))?;
            let messages = extracted_messages_from_template(&template);
            let mut options = UpdateCatalogOptions::new("en", messages)
                .with_mode(mode)
                .with_existing(fixture.existing_po.as_str());
            if let Some(locale) = locale.as_deref() {
                options = options.with_locale(locale);
            }
            let updated = update_catalog(options)
                .map_err(|error| format!("update_catalog failed: {error}"))?;
            bytes_processed += updated.content.len();
            last_rendered = Some(updated.content);
        }
        let elapsed = start.elapsed();
        let rendered = last_rendered
            .ok_or_else(|| "internal update_catalog produced no rendered content".to_owned())?;
        let summary = {
            let parsed = parse_po(&rendered)
                .map_err(|error| format!("update_catalog output did not parse: {error}"))?;
            PoSemanticSummary::from_po_file(&parsed)
        };
        let digest = digest_summary(&summary)?;
        Ok(ExecutionResult {
            tool_version: INTERNAL_TOOL_VERSION.to_owned(),
            reported_digest: digest,
            elapsed_ns: elapsed.as_nanos(),
            baseline_elapsed_ns: None,
            bytes_processed: bytes_processed as u64,
            items_processed: summary
                .items
                .len()
                .checked_mul(iterations)
                .map(|value| value as u64),
            messages_processed: None,
            artifact: capture_artifacts.then_some(ExecutionArtifact::RenderedPo(rendered)),
        })
    }

    pub(super) fn run_internal_parse_icu(
        &self,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let messages = self
            .icu_messages
            .as_ref()
            .ok_or_else(|| "internal parse-icu requires ICU messages".to_owned())?;
        let total_bytes = messages.iter().map(String::len).sum::<usize>();
        // Time only the parse; summarize the parsed messages once outside the loop.
        let mut last_parsed = None;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut parsed = Vec::with_capacity(messages.len());
            for message in messages {
                parsed.push(
                    parse_icu(message).map_err(|error| {
                        format!("failed to parse ICU benchmark message: {error}")
                    })?,
                );
            }
            last_parsed = Some(parsed);
        }
        let elapsed = start.elapsed();
        let parsed =
            last_parsed.ok_or_else(|| "internal parse-icu produced no result".to_owned())?;
        let summary = IcuFixtureSummary {
            messages: parsed.iter().map(IcuMessageSummary::from_message).collect(),
        };
        let digest = digest_summary(&summary)?;
        Ok(ExecutionResult {
            tool_version: INTERNAL_TOOL_VERSION.to_owned(),
            reported_digest: digest,
            elapsed_ns: elapsed.as_nanos(),
            baseline_elapsed_ns: None,
            bytes_processed: (total_bytes * iterations) as u64,
            items_processed: None,
            messages_processed: Some((messages.len() * iterations) as u64),
            artifact: capture_artifacts.then_some(ExecutionArtifact::IcuSummary(summary)),
        })
    }

    pub(super) fn run_internal_combine_catalogs(
        &self,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let workflow = self.catalog_workflow()?;
        let inputs = [
            CatalogCombineInput::labeled(&workflow.content, "catalog-a.po"),
            CatalogCombineInput::labeled(&workflow.content, "catalog-b.po"),
        ];
        let mut last = None;
        let start = Instant::now();
        for _ in 0..iterations {
            last = Some(
                combine_catalogs(CombineCatalogOptions::new(&inputs, "en"))
                    .map_err(|error| format!("combine_catalogs failed: {error}"))?,
            );
        }
        let elapsed = start.elapsed();
        let result = last.ok_or_else(|| "combine_catalogs produced no result".to_owned())?;
        let parsed = parse_po(&result.content)
            .map_err(|error| format!("combined output did not parse as PO: {error}"))?;
        let digest = digest_summary(&PoSemanticSummary::from_po_file(&parsed))?;
        Ok(ExecutionResult {
            tool_version: INTERNAL_TOOL_VERSION.to_owned(),
            reported_digest: digest,
            elapsed_ns: elapsed.as_nanos(),
            baseline_elapsed_ns: None,
            bytes_processed: (workflow.content.len() * 2 * iterations) as u64,
            items_processed: None,
            messages_processed: Some((workflow.message_count() * 2 * iterations) as u64),
            artifact: capture_artifacts.then_some(ExecutionArtifact::RenderedPo(result.content)),
        })
    }

    pub(super) fn run_internal_audit_catalogs(
        &self,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let workflow = self.catalog_workflow()?;
        let catalogs = workflow.catalog_refs();
        let options = CatalogAuditOptions::new("en");
        let mut last = None;
        let start = Instant::now();
        for _ in 0..iterations {
            last = Some(
                audit_catalogs(&catalogs, &options)
                    .map_err(|error| format!("audit_catalogs failed: {error}"))?,
            );
        }
        let elapsed = start.elapsed();
        let report = last.ok_or_else(|| "audit_catalogs produced no result".to_owned())?;
        self.catalog_json_result(report, elapsed, iterations, capture_artifacts)
    }

    pub(super) fn run_internal_catalog_coverage(
        &self,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let workflow = self.catalog_workflow()?;
        let catalogs = workflow.catalog_refs();
        let options = CatalogCoverageOptions::new("en");
        let mut last = None;
        let start = Instant::now();
        for _ in 0..iterations {
            last = Some(
                measure_catalog_coverage(&catalogs, &options)
                    .map_err(|error| format!("measure_catalog_coverage failed: {error}"))?,
            );
        }
        let elapsed = start.elapsed();
        let report =
            last.ok_or_else(|| "measure_catalog_coverage produced no result".to_owned())?;
        self.catalog_json_result(report, elapsed, iterations, capture_artifacts)
    }

    pub(super) fn run_internal_review_catalogs(
        &self,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let workflow = self.catalog_workflow()?;
        let catalogs = workflow.catalog_refs();
        let options = CatalogReviewOptions::new("en");
        let mut last = None;
        let start = Instant::now();
        for _ in 0..iterations {
            last = Some(
                review_catalogs(&catalogs, &catalogs, &options)
                    .map_err(|error| format!("review_catalogs failed: {error}"))?,
            );
        }
        let elapsed = start.elapsed();
        let report = last.ok_or_else(|| "review_catalogs produced no result".to_owned())?;
        self.catalog_json_result(report, elapsed, iterations, capture_artifacts)
    }

    pub(super) fn run_internal_compile_catalog_artifact(
        &self,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let workflow = self.catalog_workflow()?;
        let catalogs = workflow.catalog_refs();
        let options = CompileCatalogArtifactOptions::new("de", "en");
        let mut last = None;
        let start = Instant::now();
        for _ in 0..iterations {
            last = Some(
                compile_catalog_artifact(&catalogs, &options)
                    .map_err(|error| format!("compile_catalog_artifact failed: {error}"))?,
            );
        }
        let elapsed = start.elapsed();
        let artifact =
            last.ok_or_else(|| "compile_catalog_artifact produced no result".to_owned())?;
        self.catalog_json_result(artifact, elapsed, iterations, capture_artifacts)
    }

    fn catalog_workflow(&self) -> Result<&CatalogWorkflowFixture, String> {
        self.catalog_workflow
            .as_ref()
            .ok_or_else(|| format!("{} requires catalog workflow fixtures", self.operation))
    }

    fn catalog_json_result(
        &self,
        value: impl Serialize,
        elapsed: std::time::Duration,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let workflow = self.catalog_workflow()?;
        let summary = serde_json::to_value(value)
            .map_err(|error| format!("failed to serialize catalog workflow result: {error}"))?;
        let digest = digest_summary(&summary)?;
        Ok(ExecutionResult {
            tool_version: INTERNAL_TOOL_VERSION.to_owned(),
            reported_digest: digest,
            elapsed_ns: elapsed.as_nanos(),
            baseline_elapsed_ns: None,
            bytes_processed: (workflow.input_bytes() * iterations) as u64,
            items_processed: None,
            messages_processed: Some((workflow.message_visits() * iterations) as u64),
            artifact: capture_artifacts.then_some(ExecutionArtifact::JsonSummary(summary)),
        })
    }

    pub(super) fn run_node_adapter(
        &self,
        workspace: &Path,
        scenario: &BenchmarkScenario,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let request = self.adapter_request(scenario, iterations, capture_artifacts);
        let script = workspace.join("benchmark").join("node").join("adapter.cjs");
        run_external_adapter(
            "node",
            &["--no-warnings", script.to_string_lossy().as_ref()],
            workspace,
            &request,
        )
    }

    pub(super) fn run_python_adapter(
        &self,
        workspace: &Path,
        scenario: &BenchmarkScenario,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let request = self.adapter_request(scenario, iterations, capture_artifacts);
        let script = workspace
            .join("benchmark")
            .join("python")
            .join("adapter.py");
        let python = preferred_python_program(workspace);
        let args = vec![script.into_os_string()];
        run_external_adapter(python.as_os_str(), &args, workspace, &request)
    }

    pub(super) fn run_php_adapter(
        &self,
        workspace: &Path,
        scenario: &BenchmarkScenario,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let request = self.adapter_request(scenario, iterations, capture_artifacts);
        let script = workspace.join("benchmark").join("php").join("adapter.php");
        let args = vec![script.into_os_string()];
        run_external_adapter("php", &args, workspace, &request)
    }

    pub(super) fn run_msgcat(
        &self,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let input = self
            .po_input_path
            .as_ref()
            .ok_or_else(|| "msgcat requires PO input path".to_owned())?;
        let capture_path = self.tempdir.path().join("msgcat-output.po");
        let start = Instant::now();
        let mut last_stdout = Vec::new();
        let mut bytes_processed = 0usize;
        for _ in 0..iterations {
            let output = Command::new("msgcat")
                .arg("--no-wrap")
                .arg(input)
                .output()
                .map_err(|error| format!("failed to launch msgcat: {error}"))?;
            if !output.status.success() {
                return Err(format!(
                    "msgcat failed: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
            bytes_processed += output.stdout.len();
            last_stdout = output.stdout;
        }
        let elapsed = start.elapsed();
        let rendered = String::from_utf8(last_stdout)
            .map_err(|error| format!("msgcat output was not valid UTF-8: {error}"))?;
        if capture_artifacts {
            fs::write(&capture_path, &rendered).map_err(|error| {
                format!(
                    "failed to persist msgcat output {}: {error}",
                    capture_path.display()
                )
            })?;
        }
        let summary = PoSemanticSummary::from_po_file(
            &parse_po(&rendered).map_err(|error| format!("msgcat output parse failed: {error}"))?,
        );
        let digest = digest_summary(&summary)?;
        Ok(ExecutionResult {
            tool_version: read_command_version("msgcat", &["--version"])?
                .lines()
                .next()
                .unwrap_or("msgcat")
                .to_owned(),
            reported_digest: digest,
            elapsed_ns: elapsed.as_nanos(),
            baseline_elapsed_ns: None,
            bytes_processed: bytes_processed as u64,
            items_processed: summary
                .items
                .len()
                .checked_mul(iterations)
                .map(|value| value as u64),
            messages_processed: None,
            artifact: capture_artifacts.then_some(ExecutionArtifact::RenderedPoPath(capture_path)),
        })
    }

    pub(super) fn run_msgmerge(
        &self,
        iterations: usize,
        capture_artifacts: bool,
    ) -> Result<ExecutionResult, String> {
        let existing = self
            .existing_po_path
            .as_ref()
            .ok_or_else(|| "msgmerge requires existing PO input".to_owned())?;
        let pot = self
            .pot_path
            .as_ref()
            .ok_or_else(|| "msgmerge requires a POT template path".to_owned())?;
        let capture_path = self.tempdir.path().join("msgmerge-output.po");
        let start = Instant::now();
        let mut last_stdout = Vec::new();
        let mut bytes_processed = 0usize;
        for _ in 0..iterations {
            let output = Command::new("msgmerge")
                .arg("--no-wrap")
                .arg("--quiet")
                .arg("--no-fuzzy-matching")
                .arg(existing)
                .arg(pot)
                .output()
                .map_err(|error| format!("failed to launch msgmerge: {error}"))?;
            if !output.status.success() {
                return Err(format!(
                    "msgmerge failed: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
            bytes_processed += output.stdout.len();
            last_stdout = output.stdout;
        }
        let elapsed = start.elapsed();
        let rendered = String::from_utf8(last_stdout)
            .map_err(|error| format!("msgmerge output was not valid UTF-8: {error}"))?;
        if capture_artifacts {
            fs::write(&capture_path, &rendered).map_err(|error| {
                format!(
                    "failed to persist msgmerge output {}: {error}",
                    capture_path.display()
                )
            })?;
        }
        let summary = PoSemanticSummary::from_po_file(
            &parse_po(&rendered)
                .map_err(|error| format!("msgmerge output parse failed: {error}"))?,
        );
        let digest = digest_summary(&summary)?;
        Ok(ExecutionResult {
            tool_version: read_command_version("msgmerge", &["--version"])?
                .lines()
                .next()
                .unwrap_or("msgmerge")
                .to_owned(),
            reported_digest: digest,
            elapsed_ns: elapsed.as_nanos(),
            baseline_elapsed_ns: None,
            bytes_processed: bytes_processed as u64,
            items_processed: summary
                .items
                .len()
                .checked_mul(iterations)
                .map(|value| value as u64),
            messages_processed: None,
            artifact: capture_artifacts.then_some(ExecutionArtifact::RenderedPoPath(capture_path)),
        })
    }

    pub(super) fn adapter_request(
        &self,
        scenario: &BenchmarkScenario,
        iterations: usize,
        capture_artifacts: bool,
    ) -> AdapterRequest {
        AdapterRequest {
            scenario_id: scenario.id.clone(),
            implementation: scenario.implementation.clone(),
            workload: scenario.workload.clone(),
            operation: scenario.operation.clone(),
            fixture: scenario.fixture.clone(),
            iterations,
            capture_artifacts,
            po_input_path: self
                .po_input_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            existing_po_path: self
                .existing_po_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            pot_path: self
                .pot_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            icu_messages_path: self
                .icu_messages_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            po_output_path: capture_artifacts.then(|| {
                self.tempdir
                    .path()
                    .join(format!("{}-output.po", scenario.implementation))
                    .to_string_lossy()
                    .into_owned()
            }),
        }
    }
}

#[derive(Debug)]
pub(super) struct CatalogWorkflowFixture {
    content: String,
    catalogs: Vec<NormalizedParsedCatalog>,
}

impl CatalogWorkflowFixture {
    fn from_fixture(fixture: &Fixture) -> Result<Self, String> {
        let parsed = parse_catalog(
            ParseCatalogOptions::new(fixture.content(), "en")
                .with_locale("en")
                .with_mode(CatalogMode::IcuPo),
        )
        .map_err(|error| format!("failed to parse catalog workflow fixture: {error}"))?;
        let catalogs = ["en", "de", "fr", "es", "it", "pl"]
            .into_iter()
            .map(|locale| {
                let mut catalog = parsed.clone();
                catalog.locale = Some(locale.to_owned());
                catalog.into_normalized_view().map_err(|error| {
                    format!("failed to normalize catalog workflow fixture: {error}")
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            content: fixture.content().to_owned(),
            catalogs,
        })
    }

    fn catalog_refs(&self) -> Vec<&NormalizedParsedCatalog> {
        self.catalogs.iter().collect()
    }

    fn message_count(&self) -> usize {
        self.catalogs
            .first()
            .map_or(0, |catalog| catalog.parsed_catalog().messages.len())
    }

    fn message_visits(&self) -> usize {
        self.message_count() * self.catalogs.len()
    }

    fn input_bytes(&self) -> usize {
        self.content.len() * self.catalogs.len()
    }
}

#[derive(Debug, Clone)]
pub(super) struct OwnedMergeFixture {
    pub(super) existing_po: String,
    pub(super) api_messages: Vec<ExtractedMessage>,
    pub(super) template_pot: String,
}

impl OwnedMergeFixture {
    pub(super) fn from_fixture(fixture: &MergeFixture) -> Self {
        Self {
            existing_po: fixture.existing_po().to_owned(),
            api_messages: fixture.api_extracted_messages().to_vec(),
            // Same template the competitors parse, so the merge comparison is
            // apples-to-apples: every tool reads existing.po and template.pot.
            template_pot: build_merge_pot(fixture),
        }
    }
}

/// Builds the catalog-layer extraction input from a freshly parsed template,
/// mirroring what a host extractor hands to `update_catalog`.
pub(super) fn extracted_messages_from_template(template: &PoFile) -> Vec<ExtractedMessage> {
    template
        .items
        .iter()
        .filter(|item| !item.obsolete)
        .map(|item| {
            let comments: Vec<String> = item.extracted_comments.iter().cloned().collect();
            let origin: Vec<CatalogOrigin> = item
                .references
                .iter()
                .map(|reference| parse_origin(reference))
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

pub(super) fn build_merge_pot(fixture: &MergeFixture) -> String {
    let mut out = String::new();
    out.push_str("msgid \"\"\n");
    out.push_str("msgstr \"\"\n");
    out.push_str("\"Project-Id-Version: ferrocat benchmark template\\n\"\n");
    out.push_str("\"Content-Type: text/plain; charset=UTF-8\\n\"\n\n");

    for message in fixture.extracted_messages() {
        for comment in &message.extracted_comments {
            out.push_str("#. ");
            out.push_str(comment);
            out.push('\n');
        }
        if !message.references.is_empty() {
            out.push_str("#: ");
            let mut first = true;
            for reference in &message.references {
                if !first {
                    out.push(' ');
                }
                first = false;
                out.push_str(reference);
            }
            out.push('\n');
        }
        if !message.flags.is_empty() {
            out.push_str("#, ");
            let mut first = true;
            for flag in &message.flags {
                if !first {
                    out.push_str(", ");
                }
                first = false;
                out.push_str(flag);
            }
            out.push('\n');
        }
        if let Some(context) = &message.msgctxt {
            push_po_keyword(&mut out, "msgctxt", context);
        }
        push_po_keyword(&mut out, "msgid", &message.msgid);
        if let Some(plural) = &message.msgid_plural {
            push_po_keyword(&mut out, "msgid_plural", plural);
            out.push_str("msgstr[0] \"\"\n");
            out.push_str("msgstr[1] \"\"\n");
        } else {
            out.push_str("msgstr \"\"\n");
        }
        out.push('\n');
    }

    out
}

pub(super) fn push_po_keyword(out: &mut String, keyword: &str, value: &str) {
    if !value.contains('\n') {
        out.push_str(keyword);
        out.push_str(" \"");
        out.push_str(&escape_po_text(value));
        out.push_str("\"\n");
        return;
    }

    out.push_str(keyword);
    out.push_str(" \"\"\n");
    let mut lines = value.split('\n').peekable();
    while let Some(line) = lines.next() {
        out.push('"');
        out.push_str(&escape_po_text(line));
        if lines.peek().is_some() {
            out.push_str("\\n");
        }
        out.push_str("\"\n");
    }
}

pub(super) fn escape_po_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            _ => out.push(ch),
        }
    }
    out
}

pub(super) fn load_fixture(name: &str) -> Result<Fixture, String> {
    fixture_by_name(name).ok_or_else(|| format!("unknown benchmark fixture: {name}"))
}

pub(super) fn load_icu_fixture(name: &str) -> Result<IcuFixture, String> {
    icu_fixture_by_name(name).ok_or_else(|| format!("unknown ICU fixture: {name}"))
}

pub(super) fn load_merge_fixture(name: &str) -> Result<MergeFixture, String> {
    merge_fixture_by_name(name).ok_or_else(|| format!("unknown merge fixture: {name}"))
}

pub(super) fn build_cli_baseline_po(locale: Option<&str>) -> String {
    let (language, plural_forms) = fixture_locale_metadata(locale.unwrap_or("de"));
    format!(
        concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Project-Id-Version: ferrocat benchmark baseline\\n\"\n",
            "\"Language: {language}\\n\"\n",
            "\"Content-Type: text/plain; charset=UTF-8\\n\"\n",
            "\"Content-Transfer-Encoding: 8bit\\n\"\n",
            "\"Plural-Forms: {plural_forms}\\n\"\n"
        ),
        language = language,
        plural_forms = plural_forms
    )
}

pub(super) fn build_cli_baseline_pot() -> String {
    concat!(
        "msgid \"\"\n",
        "msgstr \"\"\n",
        "\"Project-Id-Version: ferrocat benchmark template\\n\"\n",
        "\"Content-Type: text/plain; charset=UTF-8\\n\"\n",
        "\"Content-Transfer-Encoding: 8bit\\n\"\n"
    )
    .to_owned()
}

pub(super) fn fixture_locale(name: &str) -> Option<String> {
    if !name.starts_with("gettext-") {
        return Some("de".to_owned());
    }

    let mut parts = name.split('-');
    let _prefix = parts.next()?;
    let _family = parts.next()?;
    let locale = parts.next()?;
    Some(locale.to_owned())
}

pub(super) fn fixture_locale_metadata(locale: &str) -> (&'static str, &'static str) {
    match locale {
        "fr" => ("fr", "nplurals=2; plural=(n > 1);"),
        "pl" => (
            "pl",
            "nplurals=3; plural=(n == 1 ? 0 : n % 10 >= 2 && n % 10 <= 4 && (n % 100 < 12 || n % 100 > 14) ? 1 : 2);",
        ),
        "ar" => (
            "ar",
            "nplurals=6; plural=(n==0 ? 0 : n==1 ? 1 : n==2 ? 2 : n%100>=3 && n%100<=10 ? 3 : n%100>=11 && n%100<=99 ? 4 : 5);",
        ),
        _ => ("de", "nplurals=2; plural=(n != 1);"),
    }
}

pub(super) fn fixture_catalog_mode(name: &str) -> CatalogMode {
    if name.starts_with("gettext-") || name.starts_with("mixed-") {
        CatalogMode::GettextPo
    } else {
        CatalogMode::IcuPo
    }
}
