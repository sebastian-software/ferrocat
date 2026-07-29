/*!
CLDR root ordering for catalog messages.

Lingui sorts catalogs with `new Intl.Collator("en-US")`, which resolves to the
unmodified CLDR root collation because English carries no tailoring of its own.
This module reproduces that order for the repertoire source messages normally
contain, using a generated table rather than a full Unicode collation
implementation.

The trade is deliberate. Linking ICU4X adds roughly 1.3 MB to every consumer;
the generated table covers Latin text, punctuation, symbols, and digits at a
small fraction of that cost. Since ordering only changes where an entry appears
in a catalog, a miss creates a line diff rather than an incorrect translation.

Known limits:

- Ligatures and digraphs (`ﬁ`, `Ǆ`) expand to several collation elements in the
  complete algorithm. A flat table cannot express that, so they sort by their
  own primary weight instead of as `fi` and `dz`.
- Characters outside the covered repertoire sort after it by code point. Root
  collation also places non-Latin scripts after Latin, but their internal order
  is only code-point order here.
*/

use super::catalog::CanonicalMessage;
use super::collation_table::{EXTRA, MARK_START, MARK_WEIGHTS, RANGE_START, ROWS, Row};

/// Sort key reproducing CLDR root order for the covered repertoire.
///
/// The three levels mirror the real algorithm: base characters first, then
/// diacritics, then case. They share one byte buffer to keep the hot comparison
/// to three slice comparisons while requiring only one allocation per key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CollationKey {
    bytes: Vec<u8>,
    primary_end: u32,
    secondary_end: u32,
}

impl CollationKey {
    fn primary(&self) -> &[u8] {
        &self.bytes[..self.primary_end as usize]
    }

    fn secondary(&self) -> &[u8] {
        &self.bytes[self.primary_end as usize..self.secondary_end as usize]
    }

    fn tertiary(&self) -> &[u8] {
        &self.bytes[self.secondary_end as usize..]
    }
}

impl Ord for CollationKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.primary()
            .cmp(other.primary())
            .then_with(|| self.secondary().cmp(other.secondary()))
            .then_with(|| self.tertiary().cmp(other.tertiary()))
    }
}

impl PartialOrd for CollationKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Marks an uncovered character in the primary level. The tag sits above every
/// generated table weight; the four following bytes retain code-point order.
const UNCOVERED_TAG: u8 = 0xFF;

fn is_combining(character: char) -> bool {
    ('\u{0300}'..='\u{036F}').contains(&character)
}

fn row(character: char) -> Option<&'static Row> {
    if let Some(offset) = u32::from(character).checked_sub(RANGE_START)
        && let Some(row) = usize::try_from(offset)
            .ok()
            .and_then(|index| ROWS.get(index))
    {
        return Some(row);
    }

    // Typographic characters sit far above the dense range, so they use a
    // compact sorted side table rather than padding the dense table.
    EXTRA
        .binary_search_by(|(key, _)| key.cmp(&character))
        .ok()
        .map(|index| &EXTRA[index].1)
}

struct Levels {
    /// Doubles as the finished key buffer; secondary and tertiary are appended.
    primary: Vec<u8>,
    secondary: Vec<u8>,
    tertiary: Vec<u8>,
    primary_position: u32,
}

impl Levels {
    fn push_uncovered(&mut self, character: char) {
        let base = character.to_lowercase().next().unwrap_or(character);
        self.primary.push(UNCOVERED_TAG);
        self.primary
            .extend_from_slice(&u32::from(base).to_be_bytes());
        self.primary_position = self.primary_position.saturating_add(1);
        self.tertiary.push(u8::from(character.is_uppercase()));
    }

    fn push_row(&mut self, row: &Row) {
        self.primary.push(row.primary);
        self.primary_position = self.primary_position.saturating_add(1);
        if row.secondary != 0 {
            self.push_secondary(row.secondary);
        }
        self.tertiary.push(u8::from(row.upper));
    }

    fn push_secondary(&mut self, weight: u8) {
        self.secondary
            .extend_from_slice(&u32::MAX.saturating_sub(self.primary_position).to_be_bytes());
        self.secondary.push(weight);
    }

    fn push_char(&mut self, character: char) {
        // Combining marks must be handled before lookup so decomposed and
        // precomposed accents produce the same key.
        if is_combining(character) {
            self.push_secondary(mark_weight(character));
            return;
        }
        match row(character) {
            Some(row) if row.primary != 0 => self.push_row(row),
            _ => self.push_uncovered(character),
        }
    }
}

fn mark_weight(character: char) -> u8 {
    u32::from(character)
        .checked_sub(MARK_START)
        .and_then(|offset| usize::try_from(offset).ok())
        .and_then(|index| MARK_WEIGHTS.get(index))
        .copied()
        .unwrap_or(u8::MAX)
}

const PREFIX_BYTES: usize = 32;
pub(super) type CollationPrefix = [u8; PREFIX_BYTES];

/// Packs the first primary weights of `text` into a comparison prefix.
pub(super) fn collation_prefix(text: &str) -> CollationPrefix {
    let mut packed: CollationPrefix = [0; PREFIX_BYTES];
    let mut written = 0;

    let mut push = |byte: u8, written: &mut usize| {
        if *written < PREFIX_BYTES {
            packed[*written] = byte;
            *written += 1;
        }
    };

    if text.is_ascii() {
        for &byte in text.as_bytes() {
            if written == PREFIX_BYTES {
                break;
            }
            let index = usize::from(byte).wrapping_sub(RANGE_START as usize);
            match ROWS.get(index) {
                Some(row) if row.primary != 0 => push(row.primary, &mut written),
                _ => push_uncovered_prefix(char::from(byte), &mut push, &mut written),
            }
        }
    } else {
        for character in text.chars() {
            if written == PREFIX_BYTES {
                break;
            }
            if is_combining(character) {
                continue;
            }
            match row(character) {
                Some(found) if found.primary != 0 => push(found.primary, &mut written),
                _ => push_uncovered_prefix(character, &mut push, &mut written),
            }
        }
    }

    packed
}

fn push_uncovered_prefix(
    character: char,
    push: &mut impl FnMut(u8, &mut usize),
    written: &mut usize,
) {
    let base = character.to_lowercase().next().unwrap_or(character);
    push(UNCOVERED_TAG, written);
    for byte in u32::from(base).to_be_bytes() {
        push(byte, written);
    }
}

pub(super) fn collation_key(text: &str) -> CollationKey {
    if text.is_empty() {
        return CollationKey {
            bytes: Vec::new(),
            primary_end: 0,
            secondary_end: 0,
        };
    }

    let mut levels = Levels {
        primary: Vec::with_capacity(text.len() * 2 + 8),
        secondary: Vec::new(),
        tertiary: Vec::with_capacity(text.len()),
        primary_position: 0,
    };

    if text.is_ascii() {
        // Source messages are overwhelmingly ASCII. Direct dense-table lookup
        // avoids UTF-8 decoding and side-table searches on that hot path.
        for &byte in text.as_bytes() {
            let index = usize::from(byte).wrapping_sub(RANGE_START as usize);
            match ROWS.get(index) {
                Some(row) if row.primary != 0 => levels.push_row(row),
                _ => levels.push_uncovered(char::from(byte)),
            }
        }
    } else {
        for character in text.chars() {
            levels.push_char(character);
        }
    }

    let Levels {
        mut primary,
        secondary,
        tertiary,
        primary_position: _,
    } = levels;
    let primary_end = primary.len();
    primary.extend_from_slice(&secondary);
    let secondary_end = primary.len();
    primary.extend_from_slice(&tertiary);

    CollationKey {
        bytes: primary,
        primary_end: u32::try_from(primary_end).unwrap_or(u32::MAX),
        secondary_end: u32::try_from(secondary_end).unwrap_or(u32::MAX),
    }
}

/// Full catalog key. Raw identity fields only resolve ties that root collation
/// considers equal, keeping output deterministic across input orders.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct CollatedMessageKey {
    message: CollationKey,
    context: CollationKey,
}

fn collated_message_key(message: &CanonicalMessage) -> CollatedMessageKey {
    CollatedMessageKey {
        message: collation_key(&message.msgid),
        context: collation_key(message.msgctxt.as_deref().unwrap_or("")),
    }
}

/// Collates `order`, a list of indices into `messages`, by message identity.
///
/// Only indices move during the sort, so the pass stays independent of how
/// large [`CanonicalMessage`] is. A no-allocation prefix pass does the bulk of
/// the work; full keys are built only for entries whose prefixes collide.
pub(super) fn collate_indices(messages: &[CanonicalMessage], order: &mut [usize]) {
    if order.len() < 2 {
        return;
    }

    // Pairing each prefix with its index keeps the sort's comparisons on one
    // contiguous run instead of chasing indices back into a side table.
    let mut keyed: Vec<(CollationPrefix, usize)> = order
        .iter()
        .map(|&index| (collation_prefix(&messages[index].msgid), index))
        .collect();
    keyed.sort_unstable_by_key(|&(prefix, _)| prefix);
    for (slot, &(_, index)) in order.iter_mut().zip(keyed.iter()) {
        *slot = index;
    }

    let mut start = 0;
    while start < keyed.len() {
        let prefix = keyed[start].0;
        let mut end = start + 1;
        while end < keyed.len() && keyed[end].0 == prefix {
            end += 1;
        }
        if end - start > 1 {
            let mut run: Vec<(CollatedMessageKey, usize)> = order[start..end]
                .iter()
                .map(|&index| (collated_message_key(&messages[index]), index))
                .collect();
            run.sort_by(|left, right| {
                let left_message = &messages[left.1];
                let right_message = &messages[right.1];
                left.0
                    .cmp(&right.0)
                    .then_with(|| left_message.msgid.cmp(&right_message.msgid))
                    .then_with(|| left_message.msgctxt.cmp(&right_message.msgctxt))
                    .then_with(|| {
                        left_message
                            .obsolete
                            .is_some()
                            .cmp(&right_message.obsolete.is_some())
                    })
            });
            for (slot, (_, index)) in order[start..end].iter_mut().zip(run) {
                *slot = index;
            }
        }
        start = end;
    }
}

/// Rearranges `messages` so position `p` holds the message at `order[p]`.
///
/// Messages travel through a scratch buffer instead of being swapped along
/// permutation cycles: a swap copies a whole [`CanonicalMessage`] three times
/// and most entries take part in several swaps, while moving out and back in
/// relocates every message exactly once. `Option` provides the holes that
/// taking messages out of the buffer in permuted order needs.
pub(super) fn apply_order(messages: &mut Vec<CanonicalMessage>, order: &[usize]) {
    debug_assert_eq!(
        messages.len(),
        order.len(),
        "order must cover every message"
    );
    if order
        .iter()
        .enumerate()
        .all(|(position, &source)| position == source)
    {
        return;
    }

    // `drain` keeps the allocation, so refilling below does not reallocate.
    let mut taken: Vec<Option<CanonicalMessage>> = messages.drain(..).map(Some).collect();
    messages.extend(order.iter().map(|&source| {
        taken[source]
            .take()
            .expect("a collated order names every message exactly once")
    }));
}

/// Sorts messages with a no-allocation prefix pass and full keys only for
/// entries whose prefixes collide.
pub(super) fn sort_messages_collated(messages: &mut Vec<CanonicalMessage>) {
    if messages.len() < 2 {
        return;
    }

    let mut order: Vec<usize> = (0..messages.len()).collect();
    collate_indices(messages, &mut order);
    apply_order(messages, &order);
}

#[cfg(test)]
mod tests {
    use super::{CanonicalMessage, collation_key, collation_prefix, sort_messages_collated};
    use crate::api::catalog::CanonicalTranslation;

    fn sorted(mut items: Vec<&str>) -> Vec<&str> {
        items.sort_by(|left, right| {
            collation_key(left)
                .cmp(&collation_key(right))
                .then_with(|| left.cmp(right))
        });
        items
    }

    #[test]
    fn matches_root_order_for_case_accents_and_punctuation() {
        assert_eq!(
            sorted(vec![
                "Zebra",
                "über",
                "Uber",
                "éclair",
                "eclair",
                "Apple",
                "apple",
                "Álgebra",
                "<0>Continue</0>",
                "{count, plural, one {#} other {#}}",
            ]),
            vec![
                "{count, plural, one {#} other {#}}",
                "<0>Continue</0>",
                "Álgebra",
                "apple",
                "Apple",
                "eclair",
                "éclair",
                "Uber",
                "über",
                "Zebra",
            ]
        );
    }

    #[test]
    fn treats_precomposed_and_decomposed_spellings_alike() {
        assert_eq!(collation_key("café"), collation_key("cafe\u{0301}"));
    }

    #[test]
    fn preserves_secondary_accent_information() {
        assert_ne!(collation_key("resume"), collation_key("résumé"));
        assert_eq!(
            collation_key("resume").primary(),
            collation_key("résumé").primary()
        );
    }

    #[test]
    fn matches_intl_oracle_for_accent_ranks_and_positions() {
        // Golden orders from `new Intl.Collator("en-US")`; the raw comparison
        // only resolves canonical-equivalence ties.
        assert_eq!(
            sorted(vec!["ā", "ą", "ã", "ä", "å", "â", "ă", "à", "á", "a"]),
            vec!["a", "á", "à", "ă", "â", "å", "ä", "ã", "ą", "ā"]
        );
        assert_eq!(
            sorted(vec![
                "äaa", "åaa", "àaa", "áaa", "aäa", "aåa", "aàa", "aáa", "aaä", "aaå", "aaà", "aaá",
                "aaa",
            ]),
            vec![
                "aaa", "aaá", "aaà", "aaå", "aaä", "aáa", "aàa", "aåa", "aäa", "áaa", "àaa", "åaa",
                "äaa",
            ]
        );

        assert_eq!(collation_key("áa"), collation_key("a\u{0301}a"));
        assert_eq!(collation_key("aà"), collation_key("aa\u{0300}"));
        assert!(collation_key(" Á") < collation_key(" À"));
        assert!(collation_key(" Å") < collation_key(" Ä"));
    }

    #[test]
    fn context_order_uses_the_same_intl_accent_weights() {
        let mut messages = ["àa", "áa", "aà", "aá"]
            .into_iter()
            .map(|context| CanonicalMessage {
                msgid: "same".to_owned(),
                msgctxt: Some(context.to_owned()),
                translation: CanonicalTranslation::Singular {
                    value: String::new(),
                },
                comments: Vec::new(),
                origins: Default::default(),
                placeholders: Default::default(),
                obsolete: None,
                machine: None,
            })
            .collect::<Vec<_>>();

        sort_messages_collated(&mut messages);

        assert_eq!(
            messages
                .iter()
                .filter_map(|message| message.msgctxt.as_deref())
                .collect::<Vec<_>>(),
            vec!["aá", "aà", "áa", "àa"]
        );
    }

    #[test]
    fn orders_typographic_characters_and_uncovered_scripts() {
        assert_eq!(
            sorted(vec![
                "日本語",
                "Zebra",
                "“Quoted”",
                "Em — dash",
                "Ellipsis…",
            ]),
            vec!["“Quoted”", "Ellipsis…", "Em — dash", "Zebra", "日本語"]
        );
    }

    #[test]
    fn empty_text_has_an_empty_key_and_prefix() {
        assert!(collation_key("").bytes.is_empty());
        assert_eq!(collation_prefix(""), [0; 32]);
    }

    #[test]
    fn prefix_handles_capacity_controls_and_combining_marks() {
        let thirty_two = "abcdefghijklmnopqrstuvwxyzABCDEF";
        assert_eq!(
            collation_prefix(thirty_two),
            collation_prefix("abcdefghijklmnopqrstuvwxyzABCDEFG")
        );
        assert_eq!(
            collation_prefix(&"é".repeat(32)),
            collation_prefix(&"é".repeat(33))
        );

        assert_ne!(collation_prefix("\u{001F}"), collation_prefix(""));
        assert_eq!(collation_prefix("\u{0301}"), collation_prefix(""));
        assert_eq!(
            collation_prefix("\u{0301}a"),
            collation_prefix("a"),
            "combining marks do not contribute primary prefix weights"
        );
    }

    #[test]
    fn prefix_order_never_disagrees_with_full_key() {
        let corpus = [
            "",
            "a",
            "ab",
            "Save changes",
            "Save changes?",
            "save changes",
            "“Save changes”",
            "{count, plural, one {# item} other {# items}}",
            "<0>Save</0> changes",
            "Álgebra",
            "über",
            "Uber",
            "café",
            "cafe\u{0301}",
            "日本語",
            "!Alert",
            "A very long message that shares its opening words with another one",
            "A very long message that shares its opening words with something else",
        ];

        for left in corpus {
            for right in corpus {
                let prefixes = collation_prefix(left).cmp(&collation_prefix(right));
                if !prefixes.is_eq() {
                    assert_eq!(
                        prefixes,
                        collation_key(left).cmp(&collation_key(right)),
                        "prefix order disagrees for {left:?} vs {right:?}"
                    );
                }
            }
        }
    }
}
