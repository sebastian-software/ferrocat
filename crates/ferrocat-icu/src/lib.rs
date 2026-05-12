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

mod analysis;
mod ast;
mod error;
mod parser;
mod utils;

pub use analysis::{
    IcuAnalysis, IcuArgument, IcuArgumentKind, IcuCompatibilityOptions, IcuCompatibilityReport,
    IcuDiagnostic, IcuDiagnosticSeverity, IcuFormatter, IcuPluralSummary, IcuSelectSummary,
    IcuStyleKind, IcuTagSummary, analyze_icu, compare_icu_messages, extract_argument_names,
    extract_tag_names,
};
pub use ast::{IcuMessage, IcuNode, IcuOption, IcuPluralKind};
pub use error::{IcuErrorKind, IcuParseError, IcuPosition};
pub use parser::{IcuParserOptions, parse_icu, parse_icu_with_options};
pub use utils::{
    extract_variables, has_plural, has_select, has_selectordinal, has_tag, validate_icu,
};
