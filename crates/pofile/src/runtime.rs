//! Rust-only runtime compilation and formatting APIs.
//!
//! The crate root exposes the host-neutral, serializable surface that can be
//! mirrored by thin bindings. This module keeps the richer Rust runtime API for
//! callers that want direct formatting, host hooks, or tag rendering.

pub use crate::compile::{
    compile_catalog_runtime as compile_catalog, compile_icu_runtime as compile_icu,
    CompiledCatalog, CompiledMessage, DefaultFormatHost, FormatHost, MessageValue, MessageValues,
    TagHandler,
};
