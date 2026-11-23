// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use purrdf_hash::blake3::Hasher;
use purrdf_lex::cbor::{Limits, borrowed_prefix, head};

use super::{DIGEST_LENGTH, FrameEnvelope, FrameField, WireError};

type Digest = [u8; DIGEST_LENGTH];

/// The link state under P1 §5.2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkState {
    /// The stored identifier agrees with the identity.
    Linked,
    /// The next frame confirms the computed identifier.
    Healed,
    /// Neither link check succeeded.
    Broken,
}

/// Identifiers and shape diagnostics obtained from a frame's link check.
///
/// The caller uses the effective identifier as a predecessor when present
/// and combines the link state with later payload and content findings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct LinkCheck {
    /// The link state before any checks of the payload.
    pub state: LinkState,
    /// The exact-length identifier bytes obtained from the stored field.
    pub stored_identifier: Option<Digest>,
    /// The identifier when all identity fields have the required types.
    pub computed_identifier: Option<Digest>,
    /// The identifier this ordinary frame contributes to the chain.
    pub effective_identifier: Option<Digest>,
    /// The first ill-typed field of the identity, if any.
    pub identity_error: Option<WireError>,
    /// A wire grammar violation in the stored identifier field.
    pub identifier_error: Option<WireError>,
}

impl FrameEnvelope<'_> {
    /// Verify the identifier against the supplied predecessor under P1 §5.
    ///
    /// Supply a witness only when the immediately following item is a frame
    /// whose framing is final. An opaque item, discarded frame, or damaged
    /// region between items prevents healing.
    #[must_use]
    pub fn check_link(&self, predecessor: &Digest, witness: Option<&Self>) -> LinkCheck {
        let identity_shape = self.check_identity_shape();
        let computed = identity_shape
            .is_ok()
            .then(|| self.compute_identifier(predecessor));
        let identifier = self.read_field(FrameField::Identifier).fixed_bytes();
        let stored = identifier
            .as_ref()
            .ok()
            .map(|bytes| **bytes)
            .or_else(|| permissive_identifier(self.field(FrameField::Identifier)));
        let transparent = self.field(FrameField::Transparent) == [0xf5];

        let state = if stored.is_some() && stored == computed {
            LinkState::Linked
        } else if !transparent
            && stored.is_some()
            && witness
                .zip(computed.as_ref())
                .is_some_and(|(next, computed)| {
                    next.check_identity_shape().is_ok()
                        && permissive_identifier(next.field(FrameField::Identifier))
                            == Some(next.compute_identifier(computed))
                })
        {
            LinkState::Healed
        } else {
            LinkState::Broken
        };
        let effective = if transparent {
            None
        } else if state == LinkState::Healed {
            computed
        } else {
            stored.or(computed)
        };
        LinkCheck {
            state,
            stored_identifier: stored,
            computed_identifier: computed,
            effective_identifier: effective,
            identity_error: identity_shape.err(),
            identifier_error: identifier.err(),
        }
    }

    fn compute_identifier(&self, predecessor: &Digest) -> Digest {
        let mut hasher = frame_hasher(predecessor);
        hasher.update(self.identity_fields());
        *hasher.finalize().as_bytes()
    }
}

pub(super) fn frame_hasher(predecessor: &Digest) -> Hasher {
    let mut hasher = Hasher::new_derive_key("gts2 frame");
    // E(identity) starts with a seven-field record and a 32-byte digest.
    hasher.update(&[0x87, 0x58, 0x20]);
    hasher.update(predecessor);
    hasher
}

/// P1 §3.4 uses an identifier of the digest length regardless of its head.
fn permissive_identifier(mut field: &[u8]) -> Option<Digest> {
    if let Some(bytes) = head::read_bytes(&mut field) {
        return bytes.try_into().ok();
    }
    if field.first() != Some(&0x5f) {
        return None;
    }

    // The structural envelope already delimited this byte string. Its
    // children are definite chunks with no container nesting.
    let (item, _) = borrowed_prefix(field, Limits { max_depth: 0 }).ok()?;
    let mut digest = [0; DIGEST_LENGTH];
    let mut remaining = &mut digest[..];
    for chunk in item.byte_chunks()? {
        let (output, tail) = remaining.split_at_mut_checked(chunk.len())?;
        output.copy_from_slice(chunk);
        remaining = tail;
    }
    remaining.is_empty().then_some(digest)
}
