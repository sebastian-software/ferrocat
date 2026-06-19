/// Active PO keyword context for continuation lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PoLineContext {
    /// Continuations append to `msgid`.
    Id,
    /// Continuations append to `msgid_plural`.
    IdPlural,
    /// Continuations append to `msgstr` or `msgstr[n]`.
    Str,
    /// Continuations append to `msgctxt`.
    Ctxt,
}

/// Shared line-level parser state used by owned, borrowed, and merge parsing.
#[derive(Debug, Default)]
pub(crate) struct PoLineState {
    context: Option<PoLineContext>,
    plural_index: usize,
    obsolete_line_count: usize,
    content_line_count: usize,
    has_keyword: bool,
}

impl PoLineState {
    /// Resets keyword context and content counters for the next parsed item.
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    /// Records a PO keyword line and the active continuation target it creates.
    pub(crate) fn mark_keyword(
        &mut self,
        context: PoLineContext,
        plural_index: usize,
        obsolete: bool,
    ) {
        self.context = Some(context);
        self.plural_index = plural_index;
        self.obsolete_line_count += usize::from(obsolete);
        self.content_line_count += 1;
        self.has_keyword = true;
    }

    /// Records a quoted continuation line.
    pub(crate) fn mark_continuation(&mut self, obsolete: bool) {
        self.obsolete_line_count += usize::from(obsolete);
        self.content_line_count += 1;
    }

    /// Returns the current continuation context, when any keyword has set one.
    pub(crate) const fn context(&self) -> Option<PoLineContext> {
        self.context
    }

    /// Returns the current plural translation slot.
    pub(crate) const fn plural_index(&self) -> usize {
        self.plural_index
    }

    /// Returns whether the current parser item has seen at least one keyword.
    pub(crate) const fn has_keyword(&self) -> bool {
        self.has_keyword
    }

    /// Returns whether every content line in the current item was obsolete.
    pub(crate) const fn is_obsolete_item(&self) -> bool {
        self.content_line_count > 0 && self.obsolete_line_count >= self.content_line_count
    }
}
