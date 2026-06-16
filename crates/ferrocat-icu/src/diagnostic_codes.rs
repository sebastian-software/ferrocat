//! Stable machine-readable diagnostic codes emitted by `ferrocat-icu`.
//!
//! The constants in this module are the canonical spellings used in
//! [`crate::IcuDiagnostic::code`] and [`crate::MessageMetadataDiagnostic::code`].
//! CI, editor integrations, and build tooling can match these constants instead
//! of parsing human-readable diagnostic messages.

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
