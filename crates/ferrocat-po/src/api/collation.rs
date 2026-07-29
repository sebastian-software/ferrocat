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
use super::collation_table::{EXTRA, RANGE_START, ROWS, Row};

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
}

impl Levels {
    fn push_uncovered(&mut self, character: char) {
        let base = character.to_lowercase().next().unwrap_or(character);
        self.primary.push(UNCOVERED_TAG);
        self.primary
            .extend_from_slice(&u32::from(base).to_be_bytes());
        self.tertiary.push(u8::from(character.is_uppercase()));
    }

    fn push_row(&mut self, row: &Row) {
        self.primary.push(row.primary);
        if row.secondary != 0 {
            push_mark(&mut self.secondary, row.secondary);
        }
        self.tertiary.push(u8::from(row.upper));
    }

    fn push_char(&mut self, character: char) {
        // Combining marks must be handled before lookup so decomposed and
        // precomposed accents produce the same key.
        if is_combining(character) {
            push_mark(&mut self.secondary, u32::from(character));
            return;
        }
        match row(character) {
            Some(row) if row.primary != 0 => self.push_row(row),
            _ => self.push_uncovered(character),
        }
    }
}

/// Combining marks are confined to U+0300..=U+036F, so their block offset fits
/// in a byte and retains their relative order.
fn push_mark(out: &mut Vec<u8>, mark: u32) {
    out.push(u8::try_from(mark.saturating_sub(0x0300)).unwrap_or(u8::MAX));
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

/// Sorts messages with a no-allocation prefix pass and full keys only for
/// entries whose prefixes collide.
pub(super) fn sort_messages_collated(messages: &mut [CanonicalMessage]) {
    if messages.len() < 2 {
        return;
    }

    let prefixes: Vec<CollationPrefix> = messages
        .iter()
        .map(|message| collation_prefix(&message.msgid))
        .collect();
    let mut order: Vec<usize> = (0..messages.len()).collect();
    order.sort_unstable_by_key(|&index| prefixes[index]);

    let mut start = 0;
    while start < order.len() {
        let prefix = prefixes[order[start]];
        let mut end = start + 1;
        while end < order.len() && prefixes[order[end]] == prefix {
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

    // `order` maps each destination to a source. Invert it, then apply the
    // permutation in place so catalog-sized messages are never copied.
    let mut destination = vec![0_usize; order.len()];
    for (position, &source) in order.iter().enumerate() {
        destination[source] = position;
    }
    for position in 0..destination.len() {
        while destination[position] != position {
            let target = destination[position];
            messages.swap(position, target);
            destination.swap(position, target);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{collation_key, collation_prefix};

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
