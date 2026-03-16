use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Deserialize)]
pub struct ConformanceManifest {
    pub suite: String,
    pub source: String,
    pub upstream_source: String,
    pub upstream_ref: String,
    pub license: String,
    pub scope: String,
    #[serde(default)]
    pub notes: Vec<String>,
    pub cases: Vec<ConformanceCase>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConformanceCase {
    pub id: String,
    pub capability: String,
    pub runner: String,
    pub expectation: Expectation,
    pub input: String,
    pub expected: Option<String>,
    #[serde(default)]
    pub companion_input: Option<String>,
    #[serde(default)]
    pub upstream_source: Option<String>,
    #[serde(default)]
    pub upstream_ref: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub fold_length: Option<usize>,
    #[serde(default)]
    pub compact_multiline: Option<bool>,
    #[serde(default)]
    pub locale: Option<String>,
    #[serde(default)]
    pub source_locale: Option<String>,
    #[serde(default)]
    pub item_start_index: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExpectedArtifact {
    PoParse(PoParseExpected),
    PoReject(PoRejectExpected),
    PoPluralHeader(PoPluralHeaderExpected),
    IcuParse(IcuParseExpected),
    IcuReject(IcuRejectExpected),
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PoParseExpected {
    #[serde(default)]
    pub item_count: Option<usize>,
    #[serde(default)]
    pub header_count: Option<usize>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub items: Vec<PoItemExpected>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PoItemExpected {
    pub msgid: String,
    #[serde(default)]
    pub msgctxt: Option<String>,
    #[serde(default)]
    pub msgid_plural: Option<String>,
    #[serde(default)]
    pub msgstr: Vec<String>,
    #[serde(default)]
    pub comments: Vec<String>,
    #[serde(default)]
    pub extracted_comments: Vec<String>,
    #[serde(default)]
    pub references: Vec<String>,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub obsolete: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PoRejectExpected {
    pub message_contains: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PoPluralHeaderExpected {
    #[serde(default)]
    pub raw_value: Option<String>,
    #[serde(default)]
    pub nplurals: Option<usize>,
    #[serde(default)]
    pub plural_expression: Option<String>,
    #[serde(default)]
    pub first_item_msgstr_len: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IcuParseExpected {
    #[serde(default)]
    pub node_kinds: Vec<String>,
    #[serde(default)]
    pub top_level_count: Option<usize>,
    #[serde(default)]
    pub first_literal: Option<String>,
    #[serde(default)]
    pub first_argument_name: Option<String>,
    #[serde(default)]
    pub first_plural_kind: Option<String>,
    #[serde(default)]
    pub first_plural_offset: Option<usize>,
    #[serde(default)]
    pub first_plural_option_count: Option<usize>,
    #[serde(default)]
    pub second_plural_kind: Option<String>,
    #[serde(default)]
    pub second_plural_option_count: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IcuRejectExpected {
    pub message_contains: String,
    #[serde(default)]
    pub line: Option<usize>,
    #[serde(default)]
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

pub fn manifest_dir() -> PathBuf {
    conformance_root().join("manifests")
}

pub fn fixture_dir() -> PathBuf {
    conformance_root().join("fixtures")
}

pub fn load_all_manifests() -> Result<Vec<ConformanceManifest>, ConformanceError> {
    let mut paths = fs::read_dir(manifest_dir())
        .map_err(|error| ConformanceError::new(format!("read manifest dir: {error}")))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| ConformanceError::new(format!("iterate manifest dir: {error}")))?;
    paths.sort();

    let mut manifests = Vec::with_capacity(paths.len());
    for path in paths {
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }
        manifests.push(load_manifest(&path)?);
    }
    Ok(manifests)
}

pub fn load_manifest(path: &Path) -> Result<ConformanceManifest, ConformanceError> {
    let raw = fs::read_to_string(path)
        .map_err(|error| ConformanceError::new(format!("read {}: {error}", path.display())))?;
    toml::from_str(&raw)
        .map_err(|error| ConformanceError::new(format!("parse {}: {error}", path.display())))
}

pub fn read_fixture_text(path: &str) -> Result<String, ConformanceError> {
    let full_path = fixture_dir().join(path);
    fs::read_to_string(&full_path)
        .map_err(|error| ConformanceError::new(format!("read {}: {error}", full_path.display())))
}

pub fn read_expected_artifact(path: &str) -> Result<ExpectedArtifact, ConformanceError> {
    let full_path = fixture_dir().join(path);
    let raw = fs::read_to_string(&full_path)
        .map_err(|error| ConformanceError::new(format!("read {}: {error}", full_path.display())))?;
    toml::from_str(&raw)
        .map_err(|error| ConformanceError::new(format!("parse {}: {error}", full_path.display())))
}
