mod cases;

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expectation {
    Pass,
    Reject,
    KnownGap,
}

impl Expectation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Reject => "reject",
            Self::KnownGap => "known_gap",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConformanceManifest {
    pub suite: String,
    pub source: String,
    pub upstream_source: String,
    pub upstream_ref: String,
    pub license: String,
    pub scope: String,
    pub notes: Vec<String>,
    pub cases: Vec<ConformanceCase>,
}

impl ConformanceManifest {
    pub fn new(
        suite: impl Into<String>,
        source: impl Into<String>,
        upstream_source: impl Into<String>,
        upstream_ref: impl Into<String>,
        license: impl Into<String>,
        scope: impl Into<String>,
        cases: Vec<ConformanceCase>,
    ) -> Self {
        Self {
            suite: suite.into(),
            source: source.into(),
            upstream_source: upstream_source.into(),
            upstream_ref: upstream_ref.into(),
            license: license.into(),
            scope: scope.into(),
            notes: Vec::new(),
            cases,
        }
    }

    pub fn with_notes(mut self, notes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.notes = notes.into_iter().map(Into::into).collect();
        self
    }
}

#[derive(Debug, Clone)]
pub struct ConformanceCase {
    pub id: String,
    pub capability: String,
    pub runner: String,
    pub expectation: Expectation,
    pub input: String,
    pub expected: Option<String>,
    pub expected_inline: Option<ExpectedArtifact>,
    pub companion_input: Option<String>,
    pub upstream_source: Option<String>,
    pub upstream_ref: Option<String>,
    pub notes: Option<String>,
    pub fold_length: Option<usize>,
    pub compact_multiline: Option<bool>,
    pub locale: Option<String>,
    pub source_locale: Option<String>,
    pub item_start_index: Option<usize>,
}

impl ConformanceCase {
    pub fn new(
        id: impl Into<String>,
        capability: impl Into<String>,
        runner: impl Into<String>,
        expectation: Expectation,
        input: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            capability: capability.into(),
            runner: runner.into(),
            expectation,
            input: input.into(),
            expected: None,
            expected_inline: None,
            companion_input: None,
            upstream_source: None,
            upstream_ref: None,
            notes: None,
            fold_length: None,
            compact_multiline: None,
            locale: None,
            source_locale: None,
            item_start_index: None,
        }
    }

    pub fn with_expected_fixture(mut self, path: impl Into<String>) -> Self {
        self.expected = Some(path.into());
        self
    }

    pub fn with_expected_artifact(mut self, artifact: ExpectedArtifact) -> Self {
        self.expected_inline = Some(artifact);
        self
    }

    pub fn with_companion_input(mut self, path: impl Into<String>) -> Self {
        self.companion_input = Some(path.into());
        self
    }

    pub fn source(mut self, source: impl Into<String>, reference: impl Into<String>) -> Self {
        self.upstream_source = Some(source.into());
        self.upstream_ref = Some(reference.into());
        self
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    pub fn with_fold_length(mut self, fold_length: usize) -> Self {
        self.fold_length = Some(fold_length);
        self
    }

    pub fn with_compact_multiline(mut self, compact_multiline: bool) -> Self {
        self.compact_multiline = Some(compact_multiline);
        self
    }

    pub fn with_locale(
        mut self,
        locale: impl Into<String>,
        source_locale: impl Into<String>,
    ) -> Self {
        self.locale = Some(locale.into());
        self.source_locale = Some(source_locale.into());
        self
    }

    pub fn with_item_start_index(mut self, item_start_index: usize) -> Self {
        self.item_start_index = Some(item_start_index);
        self
    }

    pub fn expected_fixture_path(&self) -> Option<&str> {
        self.expected.as_deref()
    }

    pub fn expected_artifact(&self) -> Result<ExpectedArtifact, ConformanceError> {
        self.expected_inline.clone().ok_or_else(|| {
            ConformanceError::new(format!("case {} is missing expected artifact", self.id))
        })
    }
}

#[derive(Debug, Clone)]
pub enum ExpectedArtifact {
    PoParse(PoParseExpected),
    PoReject(PoRejectExpected),
    PoPluralHeader(PoPluralHeaderExpected),
    IcuParse(IcuParseExpected),
    IcuReject(IcuRejectExpected),
}

#[derive(Debug, Clone, Default)]
pub struct PoParseExpected {
    pub item_count: Option<usize>,
    pub header_count: Option<usize>,
    pub headers: BTreeMap<String, String>,
    pub items: Vec<PoItemExpected>,
}

#[derive(Debug, Clone, Default)]
pub struct PoItemExpected {
    pub msgid: String,
    pub msgctxt: Option<String>,
    pub msgid_plural: Option<String>,
    pub msgstr: Vec<String>,
    pub comments: Vec<String>,
    pub extracted_comments: Vec<String>,
    pub references: Vec<String>,
    pub flags: Vec<String>,
    pub obsolete: bool,
}

#[derive(Debug, Clone)]
pub struct PoRejectExpected {
    pub message_contains: String,
}

#[derive(Debug, Clone, Default)]
pub struct PoPluralHeaderExpected {
    pub raw_value: Option<String>,
    pub nplurals: Option<usize>,
    pub plural_expression: Option<String>,
    pub first_item_msgstr_len: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct IcuParseExpected {
    pub node_kinds: Vec<String>,
    pub top_level_count: Option<usize>,
    pub first_literal: Option<String>,
    pub first_argument_name: Option<String>,
    pub first_plural_kind: Option<String>,
    pub first_plural_offset: Option<usize>,
    pub first_plural_option_count: Option<usize>,
    pub second_plural_kind: Option<String>,
    pub second_plural_option_count: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct IcuRejectExpected {
    pub message_contains: String,
    pub line: Option<usize>,
    pub min_column: Option<usize>,
}

#[derive(Debug)]
pub struct ConformanceError {
    message: String,
}

impl ConformanceError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ConformanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ConformanceError {}

pub fn strings<const N: usize>(values: [&str; N]) -> Vec<String> {
    values.into_iter().map(str::to_owned).collect()
}

pub fn headers<const N: usize>(pairs: [(&str, &str); N]) -> BTreeMap<String, String> {
    pairs
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate directory parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

pub fn conformance_root() -> PathBuf {
    workspace_root().join("conformance")
}

pub fn fixture_dir() -> PathBuf {
    conformance_root().join("fixtures")
}

pub fn load_all_manifests() -> Result<Vec<ConformanceManifest>, ConformanceError> {
    Ok(cases::all_manifests())
}

pub fn read_fixture_text(path: &str) -> Result<String, ConformanceError> {
    let full_path = fixture_dir().join(path);
    fs::read_to_string(&full_path)
        .map_err(|error| ConformanceError::new(format!("read {}: {error}", full_path.display())))
}
