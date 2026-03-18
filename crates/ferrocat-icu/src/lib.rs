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

mod ast;
mod error;
mod parser;
mod utils;

pub use ast::{IcuMessage, IcuNode, IcuOption, IcuPluralKind};
pub use error::{IcuErrorKind, IcuParseError, IcuPosition};
pub use parser::{IcuParserOptions, parse_icu, parse_icu_with_options};
pub use utils::{
    extract_variables, has_plural, has_select, has_selectordinal, has_tag, validate_icu,
};
