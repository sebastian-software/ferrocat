//! Catalog conversion helpers.

use std::collections::BTreeMap;

use crate::po::PoItem;
use crate::references::{
    format_reference, parse_reference, FormatReferenceOptions, ReferenceError, SourceReference,
};

/// A single catalog entry.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CatalogEntry {
    /// Source message when the catalog key is not the `msgid`.
    pub message: Option<String>,
    /// Translation value or plural variants.
    pub translation: Option<CatalogTranslation>,
    /// Plural source string.
    pub plural_source: Option<String>,
    /// Message context.
    pub context: Option<String>,
    /// Translator comments.
    pub comments: Option<Vec<String>>,
    /// Extracted comments.
    pub extracted_comments: Option<Vec<String>>,
    /// Source references.
    pub origins: Option<Vec<SourceReference>>,
    /// Whether the entry is obsolete.
    pub obsolete: Option<bool>,
    /// Flags such as `fuzzy`.
    pub flags: Option<BTreeMap<String, bool>>,
}

/// Translation payload stored inside a [`CatalogEntry`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogTranslation {
    /// Singular translation.
    Singular(String),
    /// Plural translations.
    Plural(Vec<String>),
}

/// Catalog keyed by message ID or another caller-defined key.
pub type Catalog = BTreeMap<String, CatalogEntry>;

const MSGCTXT_MSGID_SEPARATOR: char = '\u{0004}';

/// Options for converting a catalog to PO items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogToItemsOptions {
    /// Include origin references in the converted items.
    pub include_origins: bool,
    /// Include line numbers when formatting references.
    pub include_line_numbers: bool,
    /// Number of plural forms to initialize for new items.
    pub nplurals: usize,
}

impl Default for CatalogToItemsOptions {
    fn default() -> Self {
        Self {
            include_origins: true,
            include_line_numbers: true,
            nplurals: 2,
        }
    }
}

/// Options for converting items back into a catalog.
pub struct ItemsToCatalogOptions {
    /// Strategy used to derive the catalog key for each item.
    pub key_strategy: CatalogKeyStrategy,
    /// Include parsed references as `origins`.
    pub include_origins: bool,
}

impl Default for ItemsToCatalogOptions {
    fn default() -> Self {
        Self {
            key_strategy: CatalogKeyStrategy::default(),
            include_origins: true,
        }
    }
}

/// Strategy used to derive a catalog key from a PO item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CatalogKeyStrategy {
    /// Use `msgid` directly as the catalog key.
    Msgid,
    /// Use `msgctxt + "\u{0004}" + msgid` when context exists, otherwise just `msgid`.
    #[default]
    ContextMsgid,
}

/// Convert a catalog into PO items.
#[must_use]
pub fn catalog_to_items(catalog: &Catalog, options: CatalogToItemsOptions) -> Vec<PoItem> {
    catalog
        .iter()
        .map(|(key, entry)| {
            let mut item = PoItem::new(options.nplurals);
            item.msgid = entry.message.clone().unwrap_or_else(|| key.clone());

            apply_translation(&mut item, entry);
            apply_optional_fields(&mut item, entry, options);
            item
        })
        .collect()
}

/// Convert PO items into a catalog.
pub fn items_to_catalog(
    items: &[PoItem],
    options: ItemsToCatalogOptions,
) -> Result<Catalog, ReferenceError> {
    let mut catalog = Catalog::new();

    for item in items {
        if item.msgid.is_empty() {
            continue;
        }

        let key = get_catalog_key(item, &options);
        let mut entry = CatalogEntry {
            translation: Some(if item.msgid_plural.is_some() {
                CatalogTranslation::Plural(item.msgstr.clone())
            } else {
                CatalogTranslation::Singular(item.msgstr.first().cloned().unwrap_or_default())
            }),
            ..CatalogEntry::default()
        };

        add_message_field(&mut entry, item, &key);
        add_metadata_fields(&mut entry, item, options.include_origins)?;
        catalog.insert(key, entry);
    }

    Ok(catalog)
}

/// Merge two catalogs, preferring values from `updates`.
#[must_use]
pub fn merge_catalogs(base: &Catalog, updates: &Catalog) -> Catalog {
    let mut merged = base.clone();

    for (key, update) in updates {
        if let Some(existing) = merged.get_mut(key) {
            existing.message = update.message.clone().or_else(|| existing.message.clone());
            existing.translation = update
                .translation
                .clone()
                .or_else(|| existing.translation.clone());
            existing.plural_source = update
                .plural_source
                .clone()
                .or_else(|| existing.plural_source.clone());
            existing.context = update.context.clone().or_else(|| existing.context.clone());
            existing.comments = update
                .comments
                .clone()
                .or_else(|| existing.comments.clone());
            existing.extracted_comments = update
                .extracted_comments
                .clone()
                .or_else(|| existing.extracted_comments.clone());
            existing.origins = update.origins.clone().or_else(|| existing.origins.clone());
            existing.obsolete = update.obsolete.or(existing.obsolete);

            let mut flags = existing.flags.clone().unwrap_or_default();
            if let Some(update_flags) = &update.flags {
                flags.extend(update_flags.clone());
            }
            existing.flags = (!flags.is_empty()).then_some(flags);
        } else {
            merged.insert(key.clone(), update.clone());
        }
    }

    merged
}

fn apply_translation(item: &mut PoItem, entry: &CatalogEntry) {
    match &entry.translation {
        None => {
            item.msgstr = if entry.plural_source.is_some() {
                vec![String::new(), String::new()]
            } else {
                vec![String::new()]
            };
            item.msgid_plural = entry.plural_source.clone();
        }
        Some(CatalogTranslation::Singular(text)) => item.msgstr = vec![text.clone()],
        Some(CatalogTranslation::Plural(texts)) => {
            item.msgstr = texts.clone();
            item.msgid_plural = entry.plural_source.clone();
        }
    }
}

fn apply_optional_fields(item: &mut PoItem, entry: &CatalogEntry, options: CatalogToItemsOptions) {
    item.msgctxt = entry.context.clone();

    if let Some(comments) = &entry.comments {
        item.comments = comments.clone();
    }
    if let Some(comments) = &entry.extracted_comments {
        item.extracted_comments = comments.clone();
    }
    if options.include_origins {
        if let Some(origins) = &entry.origins {
            item.references = origins
                .iter()
                .map(|reference| {
                    format_reference(
                        reference,
                        FormatReferenceOptions {
                            include_line_numbers: options.include_line_numbers,
                        },
                    )
                })
                .collect();
        }
    }
    if entry.obsolete.unwrap_or(false) {
        item.obsolete = true;
    }
    if let Some(flags) = &entry.flags {
        item.flags = flags.clone();
    }
}

fn get_catalog_key(item: &PoItem, options: &ItemsToCatalogOptions) -> String {
    match options.key_strategy {
        CatalogKeyStrategy::Msgid => item.msgid.clone(),
        CatalogKeyStrategy::ContextMsgid => item
            .msgctxt
            .as_ref()
            .filter(|value| !value.is_empty())
            .map_or_else(
                || item.msgid.clone(),
                |context| format!("{context}{MSGCTXT_MSGID_SEPARATOR}{}", item.msgid),
            ),
    }
}

fn add_message_field(entry: &mut CatalogEntry, item: &PoItem, key: &str) {
    if item.msgid != key {
        entry.message = Some(item.msgid.clone());
    }
    entry.plural_source = item.msgid_plural.clone();
    entry.context = item.msgctxt.clone();
}

fn add_metadata_fields(
    entry: &mut CatalogEntry,
    item: &PoItem,
    include_origins: bool,
) -> Result<(), ReferenceError> {
    if !item.comments.is_empty() {
        entry.comments = Some(item.comments.clone());
    }
    if !item.extracted_comments.is_empty() {
        entry.extracted_comments = Some(item.extracted_comments.clone());
    }

    if include_origins && !item.references.is_empty() {
        entry.origins = Some(
            item.references
                .iter()
                .map(|reference| parse_reference(reference))
                .collect::<Result<Vec<_>, _>>()?,
        );
    }

    if item.obsolete {
        entry.obsolete = Some(true);
    }
    if !item.flags.is_empty() {
        entry.flags = Some(item.flags.clone());
    }

    Ok(())
}
