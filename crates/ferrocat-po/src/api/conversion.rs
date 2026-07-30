//! Explicit PO/FCL catalog conversion.

use std::collections::BTreeMap;
use std::fs;
use std::mem;

use super::catalog::{
    OpaqueCapture, apply_header_defaults, parse_catalog_to_internal, sort_messages,
};
use super::export::export_catalog_content;
use super::file_io::atomic_write;
use super::{
    ApiError, CatalogConvertResult, CatalogFileConvertResult, CatalogFileFormat, CatalogMode,
    CatalogStorageFormat, CatalogUpdateInput, ConvertCatalogFileOptions, ConvertCatalogOptions,
    PlaceholderCommentMode, RenderOptions, UpdateCatalogOptions,
};

/// Converts catalog content between explicit source and target modes.
///
/// Ferrocat parses the source once into its canonical internal model, retains
/// translator comments and opaque flags, then renders the target format
/// deterministically. ICU-native PO and FCL modes are interchangeable.
/// Semantic changes between ICU-native and gettext-compatible modes are
/// rejected because they cannot be guaranteed lossless.
///
/// The lossless contract covers shared message-level data: identity,
/// translations, extracted and translator comments, origins and scopes,
/// opaque flags, obsolete state, and valid machine metadata. PO headers and
/// file-level comments have no FCL representation and are normalized at that
/// format boundary.
///
/// # Errors
///
/// Returns [`ApiError`] when locales or modes are invalid, source content
/// cannot be parsed, or the target cannot represent the source semantics.
///
/// # Examples
///
/// ```rust
/// use ferrocat_po::{CatalogMode, ConvertCatalogOptions, convert_catalog};
///
/// let po = "# translator note\n#, fuzzy\nmsgid \"Hello\"\nmsgstr \"Hallo\"\n";
/// let fcl = convert_catalog(ConvertCatalogOptions::new(
///     po,
///     "en",
///     CatalogMode::IcuPo,
///     CatalogMode::IcuFcl,
/// ))?;
///
/// assert!(fcl.content.contains("\ttc=translator note\tf=fuzzy"));
/// # Ok::<(), ferrocat_po::ApiError>(())
/// ```
pub fn convert_catalog(
    options: ConvertCatalogOptions<'_>,
) -> Result<CatalogConvertResult, ApiError> {
    super::validate_source_locale(options.source_locale)?;
    validate_locale(options.locale)?;
    validate_conversion_modes(options.source_mode, options.target_mode)?;

    // Parse without an override so a conflicting source declaration cannot be
    // hidden by the caller's expected locale.
    let catalog = parse_catalog_to_internal(
        options.content,
        None,
        options.source_locale,
        options.source_mode.semantics(),
        options.source_mode.plural_encoding(),
        false,
        options.source_mode.storage_format(),
        OpaqueCapture::Keep,
    )?;
    finish_catalog_conversion(options, catalog)
}

fn finish_catalog_conversion(
    options: ConvertCatalogOptions<'_>,
    mut catalog: super::catalog::Catalog,
) -> Result<CatalogConvertResult, ApiError> {
    let locale = resolve_locale(options.locale, catalog.locale.as_deref())?;
    catalog.locale.clone_from(&locale);
    let message_count = catalog.messages.len();
    let mut diagnostics = mem::take(&mut catalog.diagnostics);
    materialize_all_placeholder_comments(&mut catalog);

    match options.target_mode.storage_format() {
        CatalogStorageFormat::Po => {
            apply_header_defaults(
                &mut catalog.headers,
                locale.as_deref(),
                options.target_mode.semantics(),
                &mut diagnostics,
                &BTreeMap::new(),
            );
            sort_messages(&mut catalog.messages, options.order_by);
        }
        CatalogStorageFormat::Fcl => {
            // FCL intentionally has only source/locale/order header state. PO
            // document-level metadata is outside the cross-format contract.
            catalog.headers.clear();
            catalog.file_comments.clear();
            catalog.file_extracted_comments.clear();
        }
    }

    let render = RenderOptions {
        order_by: options.order_by,
        include_origins: true,
        // Conversion materializes every numeric and named hint into the
        // extracted-comment list above, so the shared exporter's presentation
        // policy must not generate a second copy.
        print_placeholders_in_comments: PlaceholderCommentMode::Disabled,
        custom_header_attributes: None,
        po_serialize: options.po_serialize,
    };
    let mut export_options =
        UpdateCatalogOptions::new(options.source_locale, CatalogUpdateInput::default())
            .with_mode(options.target_mode)
            .with_render(render);
    if let Some(locale) = locale.as_deref() {
        export_options = export_options.with_locale(locale);
    }
    // Rendering is completed before the result is returned, so file callers
    // can preserve an existing destination on every validation/export error.
    let content = export_catalog_content(
        &catalog,
        &export_options,
        locale.as_deref(),
        &mut diagnostics,
    )?;

    Ok(CatalogConvertResult {
        content,
        locale,
        source_mode: options.source_mode,
        target_mode: options.target_mode,
        message_count,
        diagnostics,
    })
}

/// Converts a catalog file and atomically replaces the output path.
///
/// Source and target formats are inferred independently from the two paths
/// unless supplied explicitly. Reading, validation, parsing, and rendering all
/// finish before the atomic replacement, so the input and output may safely be
/// the same path.
///
/// # Errors
///
/// Returns [`ApiError`] when a format cannot be inferred, a mode disagrees with
/// its format, the source cannot be read or converted, or the target cannot be
/// replaced. Conversion failures leave an existing output file unchanged.
pub fn convert_catalog_file(
    options: ConvertCatalogFileOptions<'_>,
) -> Result<CatalogFileConvertResult, ApiError> {
    super::validate_source_locale(options.source_locale)?;
    validate_locale(options.locale)?;
    if options.input_path.as_os_str().is_empty() {
        return Err(ApiError::InvalidArguments(
            "input_path must not be empty".to_owned(),
        ));
    }
    if options.output_path.as_os_str().is_empty() {
        return Err(ApiError::InvalidArguments(
            "output_path must not be empty".to_owned(),
        ));
    }

    let source_format = match options.source_format {
        Some(format) => format,
        None => CatalogFileFormat::infer_from_path(options.input_path)?,
    };
    let target_format = match options.target_format {
        Some(format) => format,
        None => CatalogFileFormat::infer_from_path(options.output_path)?,
    };
    let source_mode = mode_for_format("source", source_format, options.source_mode)?;
    let target_mode = mode_for_format("target", target_format, options.target_mode)?;
    validate_conversion_modes(source_mode, target_mode)?;

    let content = fs::read_to_string(options.input_path)
        .map_err(|error| ApiError::io_with_path(options.input_path, error))?;
    let converted = convert_catalog(
        ConvertCatalogOptions::new(&content, options.source_locale, source_mode, target_mode)
            .with_optional_locale(options.locale)
            .with_order_by(options.order_by)
            .with_po_serialize_options(options.po_serialize),
    )?;

    atomic_write(options.output_path, &converted.content)?;

    Ok(CatalogFileConvertResult {
        output_path: options.output_path.to_path_buf(),
        source_format,
        target_format,
        source_mode,
        target_mode,
        locale: converted.locale,
        message_count: converted.message_count,
        diagnostics: converted.diagnostics,
    })
}

impl<'a> ConvertCatalogOptions<'a> {
    fn with_optional_locale(mut self, locale: Option<&'a str>) -> Self {
        self.locale = locale;
        self
    }
}

fn validate_locale(locale: Option<&str>) -> Result<(), ApiError> {
    if locale.is_some_and(|locale| locale.trim().is_empty()) {
        return Err(ApiError::InvalidArguments(
            "locale must not be empty".to_owned(),
        ));
    }
    Ok(())
}

/// Moves every parsed placeholder hint back to canonical extracted-comment
/// text. The normal update exporter intentionally emits only numeric names and
/// limits their presentation; conversion has a stricter preservation contract.
fn materialize_all_placeholder_comments(catalog: &mut super::catalog::Catalog) {
    for message in &mut catalog.messages {
        let mut comment = String::new();
        let mut generated = Vec::new();
        for (name, values) in &message.placeholders {
            for value in values {
                comment.clear();
                super::export::write_placeholder_comment(&mut comment, name, value);
                if !message.comments.contains(&comment) && !generated.contains(&comment) {
                    generated.push(comment.clone());
                }
            }
        }
        message.comments.extend(generated);
        message.placeholders.clear();
    }
}

fn resolve_locale(
    expected: Option<&str>,
    declared: Option<&str>,
) -> Result<Option<String>, ApiError> {
    validate_locale(declared)?;
    if let (Some(expected), Some(declared)) = (expected, declared)
        && expected != declared
    {
        return Err(ApiError::InvalidArguments(format!(
            "catalog locale {declared:?} did not match expected locale {expected:?}"
        )));
    }
    Ok(expected.or(declared).map(str::to_owned))
}

fn validate_conversion_modes(
    source_mode: CatalogMode,
    target_mode: CatalogMode,
) -> Result<(), ApiError> {
    if source_mode.semantics() != target_mode.semantics() {
        return Err(ApiError::Unsupported(format!(
            "catalog conversion from {source_mode:?} to {target_mode:?} changes semantic mode"
        )));
    }
    Ok(())
}

fn mode_for_format(
    side: &str,
    format: CatalogFileFormat,
    mode: Option<CatalogMode>,
) -> Result<CatalogMode, ApiError> {
    let mode = mode.unwrap_or_else(|| format.default_mode());
    if mode.storage_format() != format.default_mode().storage_format() {
        return Err(ApiError::InvalidArguments(format!(
            "{side} mode {mode:?} does not match {side} format {format:?}"
        )));
    }
    Ok(mode)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::PoVec;

    use super::super::PluralSource;
    use super::super::catalog::{CanonicalMessage, CanonicalTranslation, Catalog};
    use super::*;

    #[test]
    fn conversion_propagates_target_export_errors() {
        let catalog = Catalog {
            messages: vec![CanonicalMessage {
                msgid: "item".to_owned(),
                msgctxt: None,
                translation: CanonicalTranslation::Plural {
                    source: PluralSource {
                        one: Some("item".to_owned()),
                        other: "items".to_owned(),
                    },
                    translation_by_category: BTreeMap::from([(
                        "one".to_owned(),
                        "Artikel".to_owned(),
                    )]),
                    variable: "count".to_owned(),
                },
                comments: Vec::new(),
                opaque: None,
                origins: PoVec::new(),
                placeholders: BTreeMap::new(),
                obsolete: None,
                machine: None,
            }],
            ..Catalog::default()
        };

        let error = finish_catalog_conversion(
            ConvertCatalogOptions::new("", "en", CatalogMode::GettextPo, CatalogMode::GettextPo)
                .with_locale("de"),
            catalog,
        )
        .expect_err("missing other category");

        assert!(matches!(
            error,
            ApiError::Unsupported(message) if message.contains("other")
        ));
    }
}
