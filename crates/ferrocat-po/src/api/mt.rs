use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::types::{ApiError, EffectiveTranslationRef};

/// PO metadata-comment key for the machine-managed integrity lock (`#@ lock …`).
pub(super) const PO_LOCK_KEY: &str = "lock";
/// PO metadata-comment key for AI provenance (`#@ ai …`).
pub(super) const PO_AI_KEY: &str = "ai";

const MACHINE_TRANSLATION_HASH_NAMESPACE: &[u8] = b"ferrocat:mt:v1";

/// Metadata for a catalog value that was set or managed by a machine.
///
/// See [ADR 0022](https://sebastian-software.github.io/ferrocat/architecture/adr/0022-machine-managed-value-integrity-and-ai-provenance).
/// The presence of this metadata marks the value as machine-managed (by an AI
/// engine, a translation memory system, a script, …). [`lock`](Self::lock) is an
/// integrity fingerprint of the value at that time: when it no longer matches the
/// current value, a human edited it, and high-level writers drop the metadata
/// instead of carrying a stale machine fact forward.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineMetadata {
    /// Integrity fingerprint of the value at the time a machine set it.
    pub lock: String,
    /// Optional AI provenance, when the machine was an AI engine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai: Option<AiProvenance>,
}

/// AI provenance for a machine-managed value.
///
/// Engine-agnostic: any producer (Palamedes, a gateway, a TMS with an AI step)
/// can fill it. Ferrocat understands it for AI-native reporting but treats the
/// model identifier as an opaque, free-form string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiProvenance {
    /// Opaque model identifier, for example `openai/gpt-5.5-high` or `opus-4-8`.
    ///
    /// Whether it carries a provider prefix is the producer's choice; Ferrocat
    /// does not parse it.
    pub model: String,
    /// Optional model confidence in the closed range `[0, 1]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
}

/// Computes the integrity-lock hash for a translation value.
///
/// The hash is SHA-256 over a versioned, length-delimited canonical translation
/// payload plus Ferrocat's fixed `ferrocat:mt:v1` namespace. The namespace is
/// intentionally not secret; the hash detects accidental or manual changes and is
/// not a cryptographic signature.
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

pub(super) fn validate_machine_metadata(metadata: &MachineMetadata) -> Result<(), ApiError> {
    if metadata.lock.trim().is_empty() {
        return Err(ApiError::InvalidArguments(
            "machine-managed lock must not be empty".to_owned(),
        ));
    }
    if let Some(ai) = &metadata.ai {
        if ai.model.trim().is_empty() {
            return Err(ApiError::InvalidArguments(
                "ai model must not be empty".to_owned(),
            ));
        }
        if ai
            .confidence
            .is_some_and(|confidence| !(0.0..=1.0).contains(&confidence))
        {
            return Err(ApiError::InvalidArguments(
                "ai confidence must be between 0 and 1".to_owned(),
            ));
        }
    }
    Ok(())
}

/// Serializes an [`AiProvenance`] to the shared `model[:confidence]` descriptor
/// used by both PO and FCL.
pub(super) fn format_ai_descriptor(ai: &AiProvenance) -> String {
    match ai.confidence {
        Some(confidence) => format!("{}:{confidence}", ai.model),
        None => ai.model.clone(),
    }
}

/// Parses the shared `model[:confidence]` descriptor.
///
/// The model identifier is free-form, so the confidence is taken from after the
/// final `:` only when it is a valid `[0, 1]` decimal; otherwise the whole string
/// is the model (mirroring the origin `file:line` heuristic). This keeps model
/// ids that contain `/` or `:` intact and needs no escaping.
pub(super) fn parse_ai_descriptor(value: &str) -> AiProvenance {
    if let Some((model, suffix)) = value.rsplit_once(':')
        && let Some(confidence) = parse_confidence(suffix)
    {
        return AiProvenance {
            model: model.to_owned(),
            confidence: Some(confidence),
        };
    }
    AiProvenance {
        model: value.to_owned(),
        confidence: None,
    }
}

fn parse_confidence(value: &str) -> Option<f32> {
    value
        .parse::<f32>()
        .ok()
        .filter(|confidence| (0.0..=1.0).contains(confidence))
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
        AiProvenance, MachineMetadata, format_ai_descriptor, machine_translation_hash,
        parse_ai_descriptor, validate_machine_metadata,
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
    fn ai_descriptor_roundtrips_and_keeps_free_form_model() {
        // model with provider slash + confidence
        let ai = AiProvenance {
            model: "openai/gpt-5.5-high".to_owned(),
            confidence: Some(0.93),
        };
        let rendered = format_ai_descriptor(&ai);
        assert_eq!(rendered, "openai/gpt-5.5-high:0.93");
        assert_eq!(parse_ai_descriptor(&rendered), ai);

        // no confidence, both directions
        let bare = AiProvenance {
            model: "grok-4".to_owned(),
            confidence: None,
        };
        assert_eq!(format_ai_descriptor(&bare), "grok-4");
        assert_eq!(parse_ai_descriptor("grok-4"), bare);

        // a non-numeric or out-of-range suffix is part of the model, not confidence
        assert_eq!(
            parse_ai_descriptor("vendor:model-x"),
            AiProvenance {
                model: "vendor:model-x".to_owned(),
                confidence: None,
            }
        );
        assert_eq!(
            parse_ai_descriptor("weird-model:1.5"),
            AiProvenance {
                model: "weird-model:1.5".to_owned(),
                confidence: None,
            }
        );
    }

    #[test]
    fn validate_rejects_bad_metadata() {
        assert!(
            validate_machine_metadata(&MachineMetadata {
                lock: " ".to_owned(),
                ai: None,
            })
            .is_err()
        );
        assert!(
            validate_machine_metadata(&MachineMetadata {
                lock: "abc".to_owned(),
                ai: Some(AiProvenance {
                    model: "".to_owned(),
                    confidence: None,
                }),
            })
            .is_err()
        );
        assert!(
            validate_machine_metadata(&MachineMetadata {
                lock: "abc".to_owned(),
                ai: Some(AiProvenance {
                    model: "m".to_owned(),
                    confidence: Some(1.5),
                }),
            })
            .is_err()
        );
        assert!(
            validate_machine_metadata(&MachineMetadata {
                lock: "abc".to_owned(),
                ai: Some(AiProvenance {
                    model: "m".to_owned(),
                    confidence: Some(0.5),
                }),
            })
            .is_ok()
        );
    }

    #[test]
    fn base64_url_encodes_every_remainder() {
        // The hash always feeds 16 bytes (1-byte remainder); cover the encoder's
        // other tail cases directly so the helper is fully exercised.
        assert_eq!(super::base64_url_no_pad(b""), "");
        assert_eq!(super::base64_url_no_pad(b"f"), "Zg");
        assert_eq!(super::base64_url_no_pad(b"fo"), "Zm8");
        assert_eq!(super::base64_url_no_pad(b"foo"), "Zm9v");
    }
}
