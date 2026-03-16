//! Compact, performance-oriented ICU MessageFormat parsing primitives.

mod ast;
mod error;
mod parser;
mod utils;

pub use ast::{IcuMessage, IcuNode, IcuOption, IcuPluralKind};
pub use error::{IcuErrorKind, IcuParseError, IcuPosition};
pub use parser::{IcuParserOptions, parse_icu, parse_icu_with_options};
pub use utils::{extract_variables, has_plural, has_select, has_selectordinal, has_tag, validate_icu};
