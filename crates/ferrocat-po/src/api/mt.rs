use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::types::{ApiError, EffectiveTranslationRef};

pub(super) const PO_MACHINE_TRANSLATION_KEY: &str = "ferrocat-mt";

const MACHINE_TRANSLATION_HASH_NAMESPACE: &[u8] = b"ferrocat:mt:v1";

/// Machine-translation metadata attached to one translated catalog entry.
///
/// The metadata is translation-side provenance. When the stored [`hash`](Self::hash)
/// no longer matches the current translation, high-level catalog writers drop
/// the metadata instead of carrying stale machine-translation facts forward.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineTranslationMetadata {
    /// Model identifier used to produce the translation.
    ///
    /// Callers may include provider information here, for example
    /// `openai/gpt-5.5-high`.
    pub model: String,
    /// Time when the machine-generated translation was last modified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
    /// Optional model confidence from 0 to 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<u8>,
    /// Stable change-detection hash for the current translation payload.
    pub hash: String,
}

/// Computes the machine-translation change-detection hash for a translation.
///
/// The hash is SHA-256 over a versioned, length-delimited canonical translation
/// payload plus Ferrocat's fixed `ferrocat:mt:v1` namespace. The namespace is
/// intentionally not secret; the hash detects accidental or manual translation
/// changes and is not a cryptographic signature.
///
/// The emitted value is the first 128 bits encoded as unpadded Base64URL.
///
/// # Examples
///
/// ```rust
/// use ferrocat_po::{EffectiveTranslationRef, machine_translation_hash};
///
/// let hash = machine_translation_hash(EffectiveTranslationRef::Singular("Hallo"));
/// assert!(!hash.is_empty());
/// ```
#[must_use]
pub fn machine_translation_hash(translation: EffectiveTranslationRef<'_>) -> String {
    let mut payload = Vec::new();
    payload.extend_from_slice(MACHINE_TRANSLATION_HASH_NAMESPACE);
    match translation {
        EffectiveTranslationRef::Singular(value) => {
            push_hash_component(&mut payload, "singular");
            push_hash_component(&mut payload, value);
        }
        EffectiveTranslationRef::Plural(values) => {
            push_hash_component(&mut payload, "plural");
            let count = u32::try_from(values.len()).expect("plural translation count exceeds u32");
            payload.extend_from_slice(&count.to_be_bytes());
            for (category, value) in values {
                push_hash_component(&mut payload, category);
                push_hash_component(&mut payload, value);
            }
        }
    }
    let digest = Sha256::digest(&payload);
    base64_url_no_pad(&digest[..16])
}

pub(super) fn validate_machine_translation_metadata(
    metadata: &MachineTranslationMetadata,
) -> Result<(), ApiError> {
    if metadata.model.trim().is_empty() {
        return Err(ApiError::InvalidArguments(
            "machine translation model must not be empty".to_owned(),
        ));
    }
    if metadata.hash.trim().is_empty() {
        return Err(ApiError::InvalidArguments(
            "machine translation hash must not be empty".to_owned(),
        ));
    }
    if metadata
        .confidence
        .is_some_and(|confidence| confidence > 100)
    {
        return Err(ApiError::InvalidArguments(
            "machine translation confidence must be between 0 and 100".to_owned(),
        ));
    }
    Ok(())
}

pub(super) fn parse_po_machine_translation_metadata(
    value: &str,
) -> Result<MachineTranslationMetadata, ApiError> {
    let pairs = parse_key_value_pairs(value)?;
    let mut seen = BTreeSet::new();
    let mut model = None;
    let mut modified = None;
    let mut confidence = None;
    let mut hash = None;

    for (key, value) in pairs {
        if !seen.insert(key.clone()) {
            return Err(ApiError::InvalidArguments(format!(
                "duplicate machine translation metadata key {key:?}"
            )));
        }
        match key.as_str() {
            "model" => model = Some(value),
            "modified" => modified = Some(value),
            "confidence" => {
                let parsed = value.parse::<u8>().map_err(|_| {
                    ApiError::InvalidArguments(format!(
                        "invalid machine translation confidence value {value:?}"
                    ))
                })?;
                confidence = Some(parsed);
            }
            "hash" => hash = Some(value),
            other => {
                return Err(ApiError::InvalidArguments(format!(
                    "unknown machine translation metadata key {other:?}"
                )));
            }
        }
    }

    let metadata = MachineTranslationMetadata {
        model: model.ok_or_else(|| {
            ApiError::InvalidArguments("machine translation metadata is missing `model`".to_owned())
        })?,
        modified,
        confidence,
        hash: hash.ok_or_else(|| {
            ApiError::InvalidArguments("machine translation metadata is missing `hash`".to_owned())
        })?,
    };
    validate_machine_translation_metadata(&metadata)?;
    Ok(metadata)
}

pub(super) fn format_po_machine_translation_metadata(
    metadata: &MachineTranslationMetadata,
) -> String {
    let mut parts = Vec::with_capacity(4);
    parts.push(format!("model={}", quote_po_value(&metadata.model)));
    if let Some(modified) = &metadata.modified {
        parts.push(format!("modified={}", quote_po_value(modified)));
    }
    if let Some(confidence) = metadata.confidence {
        parts.push(format!("confidence={confidence}"));
    }
    parts.push(format!("hash={}", quote_po_value(&metadata.hash)));
    parts.join(" ")
}

fn parse_key_value_pairs(input: &str) -> Result<Vec<(String, String)>, ApiError> {
    let mut values = Vec::new();
    let bytes = input.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }

        let key_start = index;
        while index < bytes.len() && bytes[index] != b'=' && !bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index == key_start || index >= bytes.len() || bytes[index] != b'=' {
            return Err(ApiError::InvalidArguments(
                "invalid machine translation metadata key-value syntax".to_owned(),
            ));
        }
        let key = &input[key_start..index];
        index += 1;

        let value = if index < bytes.len() && bytes[index] == b'"' {
            index += 1;
            let mut value = String::new();
            loop {
                if index >= bytes.len() {
                    return Err(ApiError::InvalidArguments(
                        "unterminated quoted machine translation metadata value".to_owned(),
                    ));
                }
                match bytes[index] {
                    b'"' => {
                        index += 1;
                        break;
                    }
                    b'\\' => {
                        index += 1;
                        if index >= bytes.len() {
                            return Err(ApiError::InvalidArguments(
                                "unterminated escape in machine translation metadata value"
                                    .to_owned(),
                            ));
                        }
                        match bytes[index] {
                            b'"' => value.push('"'),
                            b'\\' => value.push('\\'),
                            b'n' => value.push('\n'),
                            b'r' => value.push('\r'),
                            b't' => value.push('\t'),
                            other => value.push(char::from(other)),
                        }
                        index += 1;
                    }
                    _ => {
                        let rest = &input[index..];
                        let ch = rest
                            .chars()
                            .next()
                            .expect("index is within UTF-8 string bounds");
                        value.push(ch);
                        index += ch.len_utf8();
                    }
                }
            }
            value
        } else {
            let value_start = index;
            while index < bytes.len() && !bytes[index].is_ascii_whitespace() {
                index += 1;
            }
            input[value_start..index].to_owned()
        };

        values.push((key.to_owned(), value));
    }

    Ok(values)
}

fn quote_po_value(value: &str) -> String {
    if !value
        .bytes()
        .any(|byte| byte.is_ascii_whitespace() || matches!(byte, b'"' | b'\\'))
    {
        return value.to_owned();
    }

    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn push_hash_component(out: &mut Vec<u8>, value: &str) {
    let value_len =
        u32::try_from(value.len()).expect("machine translation hash component exceeds u32");
    out.extend_from_slice(&value_len.to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn base64_url_no_pad(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity((bytes.len() * 4).div_ceil(3));
    let mut index = 0;

    while index + 3 <= bytes.len() {
        let chunk = (u32::from(bytes[index]) << 16)
            | (u32::from(bytes[index + 1]) << 8)
            | u32::from(bytes[index + 2]);
        out.push(ALPHABET[((chunk >> 18) & 0x3f) as usize] as char);
        out.push(ALPHABET[((chunk >> 12) & 0x3f) as usize] as char);
        out.push(ALPHABET[((chunk >> 6) & 0x3f) as usize] as char);
        out.push(ALPHABET[(chunk & 0x3f) as usize] as char);
        index += 3;
    }

    match bytes.len() - index {
        1 => {
            let chunk = u32::from(bytes[index]) << 16;
            out.push(ALPHABET[((chunk >> 18) & 0x3f) as usize] as char);
            out.push(ALPHABET[((chunk >> 12) & 0x3f) as usize] as char);
        }
        2 => {
            let chunk = (u32::from(bytes[index]) << 16) | (u32::from(bytes[index + 1]) << 8);
            out.push(ALPHABET[((chunk >> 18) & 0x3f) as usize] as char);
            out.push(ALPHABET[((chunk >> 12) & 0x3f) as usize] as char);
            out.push(ALPHABET[((chunk >> 6) & 0x3f) as usize] as char);
        }
        _ => {}
    }

    out
}

#[cfg(test)]
mod tests {
    use super::{
        MachineTranslationMetadata, format_po_machine_translation_metadata,
        machine_translation_hash, parse_po_machine_translation_metadata,
    };
    use crate::api::EffectiveTranslationRef;
    use std::collections::BTreeMap;

    #[test]
    fn hash_is_stable_for_singular_and_plural_translations() {
        let singular = machine_translation_hash(EffectiveTranslationRef::Singular("Hallo"));
        assert_eq!(
            singular,
            machine_translation_hash(EffectiveTranslationRef::Singular("Hallo"))
        );
        assert_ne!(
            singular,
            machine_translation_hash(EffectiveTranslationRef::Singular("Hello"))
        );

        let plural = BTreeMap::from([
            ("one".to_owned(), "Eine Datei".to_owned()),
            ("other".to_owned(), "{count} Dateien".to_owned()),
        ]);
        let repeated = BTreeMap::from([
            ("one".to_owned(), "Eine Datei".to_owned()),
            ("other".to_owned(), "{count} Dateien".to_owned()),
        ]);
        assert_eq!(
            machine_translation_hash(EffectiveTranslationRef::Plural(&plural)),
            machine_translation_hash(EffectiveTranslationRef::Plural(&repeated))
        );
        assert_eq!(singular.len(), 22);
    }

    #[test]
    fn po_machine_translation_metadata_roundtrips_quoted_values() {
        let metadata = MachineTranslationMetadata {
            model: "openai/gpt 5.5".to_owned(),
            modified: Some("2026-05-12T10:30:00Z".to_owned()),
            confidence: Some(95),
            hash: "abc\\def".to_owned(),
        };
        let rendered = format_po_machine_translation_metadata(&metadata);
        assert_eq!(
            parse_po_machine_translation_metadata(&rendered).expect("parse rendered"),
            metadata
        );
    }

    #[test]
    fn po_machine_translation_metadata_rejects_invalid_inputs() {
        for (input, expected) in [
            (
                "model= hash=abc",
                "machine translation model must not be empty",
            ),
            (
                "model=gpt hash=",
                "machine translation hash must not be empty",
            ),
            (
                "model=gpt confidence=101 hash=abc",
                "machine translation confidence must be between 0 and 100",
            ),
            (
                "model=gpt confidence=high hash=abc",
                "invalid machine translation confidence value \"high\"",
            ),
            (
                "model=gpt model=other hash=abc",
                "duplicate machine translation metadata key \"model\"",
            ),
            (
                "model=gpt provider=openai hash=abc",
                "unknown machine translation metadata key \"provider\"",
            ),
            (
                "model=gpt",
                "machine translation metadata is missing `hash`",
            ),
            (
                "hash=abc",
                "machine translation metadata is missing `model`",
            ),
            (
                "model",
                "invalid machine translation metadata key-value syntax",
            ),
            (
                "model=\"gpt",
                "unterminated quoted machine translation metadata value",
            ),
            (
                "model=\"gpt\\",
                "unterminated escape in machine translation metadata value",
            ),
        ] {
            assert_eq!(
                parse_po_machine_translation_metadata(input)
                    .expect_err("expected invalid machine translation metadata")
                    .to_string(),
                expected
            );
        }
    }

    #[test]
    fn po_machine_translation_metadata_handles_escape_sequences() {
        let parsed = parse_po_machine_translation_metadata(
            "model=\"gpt\\nmini\" modified=\"line\\rnext\\ttab\" hash=\"a\\\\b\"",
        )
        .expect("parse escaped metadata");

        assert_eq!(parsed.model, "gpt\nmini");
        assert_eq!(parsed.modified.as_deref(), Some("line\rnext\ttab"));
        assert_eq!(parsed.hash, "a\\b");

        assert_eq!(
            format_po_machine_translation_metadata(&parsed),
            "model=\"gpt\\nmini\" modified=\"line\\rnext\\ttab\" hash=\"a\\\\b\""
        );
    }
}
