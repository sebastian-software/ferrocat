//! Stable machine-readable diagnostic codes emitted by `ferrocat-icu`.
//!
//! The constants in this module are the canonical spellings used in
//! [`crate::IcuDiagnostic::code`] and [`crate::MessageMetadataDiagnostic::code`].
//! CI, editor integrations, and build tooling can match these constants instead
//! of parsing human-readable diagnostic messages.

// This block has a feature-gated fallback copy in
// crates/ferrocat-po/src/diagnostic_codes.rs; a test there keeps both in sync.
// sync(diagnostic-code-type): begin
use std::fmt;
use std::ops::Deref;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Stable machine-readable diagnostic code.
///
/// The wire format remains the plain code string. This wrapper makes diagnostic
/// code fields distinct from arbitrary human-readable strings while preserving
/// string comparisons through [`Self::as_str`], [`AsRef<str>`], and `PartialEq<&str>`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct DiagnosticCode(String);

impl DiagnosticCode {
    /// Creates a diagnostic code from its canonical string spelling.
    #[must_use]
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into())
    }

    /// Returns the canonical string spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl AsRef<str> for DiagnosticCode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for DiagnosticCode {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl From<&str> for DiagnosticCode {
    fn from(code: &str) -> Self {
        Self::new(code)
    }
}

impl From<String> for DiagnosticCode {
    fn from(code: String) -> Self {
        Self::new(code)
    }
}

impl PartialEq<&str> for DiagnosticCode {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<DiagnosticCode> for &str {
    fn eq(&self, other: &DiagnosticCode) -> bool {
        *self == other.as_str()
    }
}
// sync(diagnostic-code-type): end

/// ICU compatibility diagnostic codes.
pub mod icu {
    /// Formatter kind is not supported by the target runtime.
    pub const UNSUPPORTED_FORMATTER_KIND: &str = "icu.unsupported_formatter_kind";
    /// Formatter style is not supported by the target runtime.
    pub const UNSUPPORTED_FORMATTER_STYLE: &str = "icu.unsupported_formatter_style";
    /// Translation omits an argument used by the source message.
    pub const MISSING_ARGUMENT: &str = "icu.missing_argument";
    /// Translation changes an argument kind used by the source message.
    pub const ARGUMENT_KIND_CHANGED: &str = "icu.argument_kind_changed";
    /// Translation adds an argument that is not present in the source message.
    pub const EXTRA_ARGUMENT: &str = "icu.extra_argument";
    /// Translation changes a formatter style used by the source message.
    pub const FORMATTER_STYLE_CHANGED: &str = "icu.formatter_style_changed";
    /// Translation omits a rich-text tag used by the source message.
    pub const MISSING_TAG: &str = "icu.missing_tag";
    /// Translation adds a rich-text tag that is not present in the source message.
    pub const EXTRA_TAG: &str = "icu.extra_tag";
    /// Translation omits a selector from a source `select` argument.
    pub const MISSING_SELECT_SELECTOR: &str = "icu.missing_select_selector";
    /// Translation adds a selector to a source `select` argument.
    pub const EXTRA_SELECT_SELECTOR: &str = "icu.extra_select_selector";
    /// Translation changes a source plural offset.
    pub const PLURAL_OFFSET_CHANGED: &str = "icu.plural_offset_changed";
    /// Translation omits a selector from a source plural argument.
    pub const MISSING_PLURAL_SELECTOR: &str = "icu.missing_plural_selector";
    /// ICU formatter uses an opaque pattern style.
    pub const PATTERN_STYLE_DISCOURAGED: &str = "icu.pattern_style_discouraged";

    /// All ICU compatibility diagnostic codes emitted by this crate.
    pub const ALL: &[&str] = &[
        UNSUPPORTED_FORMATTER_KIND,
        UNSUPPORTED_FORMATTER_STYLE,
        MISSING_ARGUMENT,
        ARGUMENT_KIND_CHANGED,
        EXTRA_ARGUMENT,
        FORMATTER_STYLE_CHANGED,
        MISSING_TAG,
        EXTRA_TAG,
        MISSING_SELECT_SELECTOR,
        EXTRA_SELECT_SELECTOR,
        PLURAL_OFFSET_CHANGED,
        MISSING_PLURAL_SELECTOR,
        PATTERN_STYLE_DISCOURAGED,
    ];
}

/// Semantic message metadata diagnostic codes.
pub mod metadata {
    /// Metadata `msgid` is not valid ICU MessageFormat v1.
    pub const INVALID_MSGID: &str = "metadata.invalid_msgid";
    /// Metadata omits an argument parsed from `msgid`.
    pub const MISSING_ARGUMENT: &str = "metadata.missing_argument";
    /// Metadata declares an argument that is not used by `msgid`.
    pub const EXTRA_ARGUMENT: &str = "metadata.extra_argument";
    /// Metadata declares an argument kind that does not match `msgid`.
    pub const ARGUMENT_KIND_MISMATCH: &str = "metadata.argument_kind_mismatch";
    /// Metadata omits a rich-text tag parsed from `msgid`.
    pub const MISSING_TAG: &str = "metadata.missing_tag";
    /// Metadata declares a rich-text tag that is not used by `msgid`.
    pub const EXTRA_TAG: &str = "metadata.extra_tag";
    /// Metadata omits a selector parsed from `msgid`.
    pub const MISSING_SELECTOR: &str = "metadata.missing_selector";
    /// Metadata declares a selector that is not used by `msgid`.
    pub const EXTRA_SELECTOR: &str = "metadata.extra_selector";
    /// Metadata declares a selector kind that does not match `msgid`.
    pub const SELECTOR_KIND_MISMATCH: &str = "metadata.selector_kind_mismatch";
    /// Metadata omits a selector case parsed from `msgid`.
    pub const MISSING_SELECTOR_CASE: &str = "metadata.missing_selector_case";
    /// Metadata declares a selector case that is not used by `msgid`.
    pub const EXTRA_SELECTOR_CASE: &str = "metadata.extra_selector_case";
    /// Metadata declares a selector offset that does not match `msgid`.
    pub const SELECTOR_OFFSET_MISMATCH: &str = "metadata.selector_offset_mismatch";

    /// All semantic metadata diagnostic codes emitted by this crate.
    pub const ALL: &[&str] = &[
        INVALID_MSGID,
        MISSING_ARGUMENT,
        EXTRA_ARGUMENT,
        ARGUMENT_KIND_MISMATCH,
        MISSING_TAG,
        EXTRA_TAG,
        MISSING_SELECTOR,
        EXTRA_SELECTOR,
        SELECTOR_KIND_MISMATCH,
        MISSING_SELECTOR_CASE,
        EXTRA_SELECTOR_CASE,
        SELECTOR_OFFSET_MISMATCH,
    ];
}

#[cfg(test)]
mod tests {
    use super::DiagnosticCode;

    #[test]
    fn diagnostic_code_preserves_canonical_string_access() {
        let code = DiagnosticCode::from(String::from("icu.missing_argument"));

        assert_eq!(code.as_str(), "icu.missing_argument");
        assert_eq!(code.as_ref(), "icu.missing_argument");
        assert_eq!(&*code, "icu.missing_argument");
        assert_eq!(code.to_string(), "icu.missing_argument");
        assert_eq!(code, "icu.missing_argument");
        assert_eq!("icu.missing_argument", code);
    }
}
