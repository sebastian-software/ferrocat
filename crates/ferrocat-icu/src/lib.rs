#![warn(missing_docs, rustdoc::broken_intra_doc_links)]
//! Compact, performance-oriented ICU `MessageFormat` parsing primitives.
//!
//! # Examples
//!
//! ```rust
//! use ferrocat_icu::{extract_variables, parse_icu};
//!
//! let message = parse_icu("Hello {name}, you have {count, plural, one {# item} other {# items}}.")?;
//! assert_eq!(extract_variables(&message), vec!["name", "count"]);
//! # Ok::<(), ferrocat_icu::IcuParseError>(())
//! ```
//!
//! ```rust
//! use ferrocat_icu::{
//!     IcuCompatibilityOptions, analyze_icu, compare_icu_messages, parse_icu,
//! };
//!
//! let source = parse_icu("Hello {name}, you have {count, number, integer} files.")?;
//! let translation = parse_icu("Hallo, du hast {count, number, integer} Dateien.")?;
//!
//! let source_analysis = analyze_icu(&source);
//! assert_eq!(source_analysis.arguments.len(), 2);
//!
//! let report = compare_icu_messages(
//!     &source,
//!     &translation,
//!     &IcuCompatibilityOptions::default(),
//! );
//! assert!(report.has_errors());
//! # Ok::<(), ferrocat_icu::IcuParseError>(())
//! ```
//!
//! ```rust
//! use ferrocat_icu::{
//!     MessageArgumentKind, MessageMetadataInput, normalize_message_metadata,
//! };
//!
//! let metadata = normalize_message_metadata(MessageMetadataInput::new(
//!     "{count, plural, one {One item} other {# items}}",
//! ))?;
//!
//! assert_eq!(
//!     metadata.args.get("count").map(|argument| argument.kind),
//!     Some(MessageArgumentKind::Number)
//! );
//! assert!(metadata.selectors.contains_key("count"));
//! # Ok::<(), ferrocat_icu::IcuParseError>(())
//! ```

mod analysis;
mod ast;
mod error;
mod metadata;
mod parser;
mod utils;

pub use analysis::{
    IcuAnalysis, IcuArgument, IcuArgumentKind, IcuCompatibilityOptions, IcuCompatibilityReport,
    IcuDiagnostic, IcuDiagnosticSeverity, IcuFormatter, IcuFormatterSupport, IcuPluralSummary,
    IcuSelectSummary, IcuStyleKind, IcuTagSummary, analyze_icu, compare_icu_messages,
    extract_argument_names, extract_tag_names, validate_icu_formatter_support,
};
pub use ast::{IcuMessage, IcuNode, IcuOption, IcuPluralKind};
pub use error::{IcuErrorKind, IcuParseError, IcuPosition};
pub use metadata::{
    MessageArgumentFormatMetadata, MessageArgumentKind, MessageArgumentMetadata,
    MessageArgumentMetadataInput, MessageFormatStyleKind, MessageMetadata,
    MessageMetadataDiagnostic, MessageMetadataInput, MessageMetadataValidationReport,
    MessageOriginMetadata, MessageSelectorKind, MessageSelectorMetadata,
    derive_message_metadata_from_icu, normalize_message_metadata, validate_message_metadata,
};
pub use parser::{IcuParserOptions, parse_icu, parse_icu_with_options};
pub use utils::{
    extract_variables, has_plural, has_select, has_selectordinal, has_tag, validate_icu,
};
