//! Stable machine-readable diagnostic codes emitted by `ferrocat-po`.
//!
//! The constants in this module are the canonical spellings used in
//! [`crate::Diagnostic::code`], [`crate::CatalogAuditDiagnostic::code`], and
//! [`crate::CompiledCatalogDiagnostic::code`]. CI, editor integrations, and
//! build tooling can match these constants instead of parsing human-readable
//! diagnostic messages.

/// Catalog audit diagnostic codes.
pub mod catalog {
    /// Audit did not receive the configured source locale catalog.
    pub const MISSING_SOURCE_LOCALE: &str = "catalog.missing_source_locale";
    /// Audit did not receive a requested target locale catalog.
    pub const MISSING_LOCALE: &str = "catalog.missing_locale";
    /// Catalog contains an obsolete entry.
    pub const OBSOLETE_ENTRY: &str = "catalog.obsolete_entry";
    /// Catalog entry carries a fuzzy flag.
    pub const FUZZY_FLAG: &str = "catalog.fuzzy_flag";
    /// Target locale is missing an active source message.
    pub const MISSING_TRANSLATION: &str = "catalog.missing_translation";
    /// Target locale has an empty translation for an active source message.
    pub const EMPTY_TRANSLATION: &str = "catalog.empty_translation";
    /// Target locale contains an active message missing from the source catalog.
    pub const EXTRA_TRANSLATION: &str = "catalog.extra_translation";
    /// Semantic metadata contains a duplicate message identity.
    pub const DUPLICATE_METADATA: &str = "catalog.duplicate_metadata";
    /// Semantic metadata refers to a message outside the active source catalog.
    pub const METADATA_UNKNOWN_MESSAGE: &str = "catalog.metadata_unknown_message";

    /// All catalog audit diagnostic codes emitted by this crate.
    pub const ALL: &[&str] = &[
        MISSING_SOURCE_LOCALE,
        MISSING_LOCALE,
        OBSOLETE_ENTRY,
        FUZZY_FLAG,
        MISSING_TRANSLATION,
        EMPTY_TRANSLATION,
        EXTRA_TRANSLATION,
        DUPLICATE_METADATA,
        METADATA_UNKNOWN_MESSAGE,
    ];
}

/// Catalog combine diagnostic codes.
pub mod combine {
    /// Combine resolved a conflicting translation according to the chosen strategy.
    pub const CONFLICT_RESOLVED: &str = "combine.conflict_resolved";

    /// All catalog combine diagnostic codes emitted by this crate.
    pub const ALL: &[&str] = &[CONFLICT_RESOLVED];
}

/// Runtime compilation diagnostic codes.
pub mod compile {
    /// Final runtime message failed ICU validation.
    pub const INVALID_ICU_MESSAGE: &str = "compile.invalid_icu_message";

    /// All runtime compilation diagnostic codes emitted by this crate.
    pub const ALL: &[&str] = &[INVALID_ICU_MESSAGE];
}

/// PO parse diagnostic codes emitted by high-level catalog helpers.
pub mod parse {
    /// `Plural-Forms` header has a plural expression but no parseable `nplurals`.
    pub const INVALID_PLURAL_FORMS_HEADER: &str = "parse.invalid_plural_forms_header";

    /// All PO parse diagnostic codes emitted by this crate.
    pub const ALL: &[&str] = &[INVALID_PLURAL_FORMS_HEADER];
}

/// Plural import/export diagnostic codes.
pub mod plural {
    /// Plural placeholder name could not be inferred and `count` was assumed.
    pub const ASSUMED_VARIABLE: &str = "plural.assumed_variable";
    /// No safe default `Plural-Forms` header is known for the locale.
    pub const MISSING_PLURAL_FORMS_HEADER: &str = "plural.missing_plural_forms_header";
    /// `Plural-Forms` header was completed from a safe locale default.
    pub const COMPLETED_PLURAL_FORMS_HEADER: &str = "plural.completed_plural_forms_header";
    /// ICU plural cannot be exported to gettext plural form safely.
    pub const UNSUPPORTED_GETTEXT_EXPORT: &str = "plural.unsupported_gettext_export";
    /// `nplurals` does not match the locale-derived plural category count.
    pub const NPLURALS_LOCALE_MISMATCH: &str = "plural.nplurals_locale_mismatch";
    /// `Plural-Forms` header declares `nplurals` but omits the expression.
    pub const MISSING_PLURAL_EXPRESSION: &str = "plural.missing_plural_expression";

    /// All plural diagnostic codes emitted by this crate.
    pub const ALL: &[&str] = &[
        ASSUMED_VARIABLE,
        MISSING_PLURAL_FORMS_HEADER,
        COMPLETED_PLURAL_FORMS_HEADER,
        UNSUPPORTED_GETTEXT_EXPORT,
        NPLURALS_LOCALE_MISMATCH,
        MISSING_PLURAL_EXPRESSION,
    ];
}

/// ICU syntax and compatibility diagnostic codes surfaced by catalog APIs.
pub mod icu {
    /// Catalog message is not valid ICU MessageFormat v1.
    pub const INVALID_SYNTAX: &str = "icu.invalid_syntax";
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

    /// All ICU diagnostic codes emitted or surfaced by this crate.
    pub const ALL: &[&str] = &[
        INVALID_SYNTAX,
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

/// Semantic message metadata diagnostic codes surfaced by catalog APIs.
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

    /// All semantic metadata diagnostic codes surfaced by this crate.
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
