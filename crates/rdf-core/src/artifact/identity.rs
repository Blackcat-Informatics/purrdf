// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The artifact's INPUT BINDING: an ordered list of labelled components saying
//! which inputs a prepared product was compiled from, carried in the artifact
//! alongside the digest that seals them.
//!
//! # Why the components travel, not just their digest
//!
//! The obvious design — and the rejected one — is to fold the inputs into a
//! single 32-byte digest and store only that. It is smaller, it is trivially
//! constant-time to compare, and it answers exactly one question: *"do these
//! match?"* When the answer is no, which is the only time anyone looks, it has
//! nothing further to say. A caller holding a prepared product that will not
//! open against their inputs gets `identity digest mismatch` and no way to find
//! out whether the shapes graph moved, the vocabulary changed, or they simply
//! picked up last week's artifact.
//!
//! So the components travel WITH the digest and this type is DECODABLE. A
//! mismatch is reported through [`Identity::mismatches`] as
//! ``component `base` (position 0): artifact has "http://example.org/a", caller
//! supplied "http://example.org/b"`` — the thing the caller actually has to fix.
//! The digest remains the authority (it is what the container seals and what a
//! fast equality check uses); the components are what make its verdict
//! actionable. The failure prevented is an operational one: an unexplainable
//! refusal that a caller can only resolve by rebuilding everything and hoping.
//!
//! # Framing: injective, or the whole binding is worthless
//!
//! Components are encoded as a flat sequence of
//! `varint(label_len) ‖ label ‖ varint(value_len) ‖ value`, reusing
//! [`crate::ir::pack::bits`]'s LEB128 primitives rather than restating them.
//! BOTH the label and the value are length-prefixed, following the
//! `append_key_part` pattern the shapes crate's schema-compilation key uses.
//!
//! This is not stylistic. Concatenating label and value without prefixing each
//! makes the encoding non-injective: `{"a": "bc"}` and `{"ab": "c"}` both
//! flatten to `abc`, so two identities over genuinely different inputs collide
//! on one digest and each artifact opens against the other's inputs. That is a
//! forged binding produced by a formatting decision, and length-prefixing both
//! halves is the entire fix.
//!
//! The encoding is CANONICAL: [`Identity::to_bytes`] emits minimal LEB128, and
//! [`Identity::from_bytes`] re-encodes what it decoded and refuses any buffer
//! that is not byte-identical to it. Without that check a padded varint would
//! give a second admissible spelling of the same components — a second spelling
//! is a second digest, and a format with two spellings of one meaning cannot
//! say what it seals.
//!
//! # Labels are not a map
//!
//! The ORDERED sequence is the identity; labels need not be unique, and a
//! repeated label is a well-formed identity whose components stay distinct
//! because position distinguishes them. [`Identity::component`] returns the
//! FIRST occurrence and is a convenience for the common unique-label case, not
//! the definition of identity. Refusing duplicate labels was considered and
//! rejected: [`Identity::push`] is infallible by design (a caller assembling an
//! identity has no useful recovery at the push site), so the refusal would have
//! had to fire at build time, far from its cause, to forbid a shape that the
//! injective framing already handles correctly.

use std::fmt;

use sha2::{Digest, Sha256};

use crate::ir::pack::bits::{PackBitsError, read_varint, write_varint};

use super::container::ArtifactError;

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// One labelled component of an [`Identity`]: a human-meaningful label and the
/// opaque bytes it binds (typically a canonical digest, a version string or a
/// configuration marker).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdentityComponent {
    label: String,
    value: Vec<u8>,
}

impl IdentityComponent {
    /// The component's label, as supplied to [`Identity::push`].
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The component's opaque value bytes.
    #[must_use]
    pub fn value(&self) -> &[u8] {
        &self.value
    }
}

// ---------------------------------------------------------------------------
// Mismatch reporting
// ---------------------------------------------------------------------------

/// One way a caller-supplied [`Identity`] differs from the one an artifact
/// carries — the actionable half of a refused admission (see the
/// [module docs](self)).
///
/// Every variant names the POSITION as well as the label, because an identity
/// is an ordered sequence: two components can share a label and still be
/// different components.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum IdentityMismatch {
    /// Both identities have a component at this position, but under different
    /// labels — the caller's identity has a different SHAPE, not merely
    /// different contents.
    LabelDiffers {
        /// The zero-based position of the disagreeing component.
        position: usize,
        /// The label the artifact carries there.
        artifact: String,
        /// The label the caller supplied there.
        caller: String,
    },
    /// Both identities agree on the label at this position but not the value —
    /// the ordinary "you compiled this from something else" report.
    ValueDiffers {
        /// The zero-based position of the disagreeing component.
        position: usize,
        /// The label both identities carry at this position.
        label: String,
        /// The value the artifact carries.
        artifact: Vec<u8>,
        /// The value the caller supplied.
        caller: Vec<u8>,
    },
    /// The artifact carries a component here that the caller's identity does
    /// not reach: the caller supplied FEWER components.
    MissingFromCaller {
        /// The zero-based position of the component the caller omitted.
        position: usize,
        /// The label the artifact carries there.
        label: String,
        /// The value the artifact carries there.
        artifact: Vec<u8>,
    },
    /// The caller supplied a component the artifact does not reach: the caller
    /// supplied MORE components.
    UnexpectedFromCaller {
        /// The zero-based position of the extra component.
        position: usize,
        /// The label the caller supplied there.
        label: String,
        /// The value the caller supplied there.
        caller: Vec<u8>,
    },
}

impl fmt::Display for IdentityMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LabelDiffers {
                position,
                artifact,
                caller,
            } => write!(
                f,
                "component at position {position}: artifact has label `{artifact}`, caller supplied label `{caller}`"
            ),
            Self::ValueDiffers {
                position,
                label,
                artifact,
                caller,
            } => write!(
                f,
                "component `{label}` (position {position}): artifact has {}, caller supplied {}",
                render_value(artifact),
                render_value(caller)
            ),
            Self::MissingFromCaller {
                position,
                label,
                artifact,
            } => write!(
                f,
                "component `{label}` (position {position}): artifact has {}, caller supplied nothing",
                render_value(artifact)
            ),
            Self::UnexpectedFromCaller {
                position,
                label,
                caller,
            } => write!(
                f,
                "component `{label}` (position {position}): artifact has nothing, caller supplied {}",
                render_value(caller)
            ),
        }
    }
}

/// Render a component value for a human: quoted when it is printable UTF-8,
/// lowercase hex otherwise. Identity values are usually digests (hex) or IRIs
/// and version strings (text), and showing a digest as mojibake helps nobody.
fn render_value(value: &[u8]) -> String {
    match std::str::from_utf8(value) {
        Ok(text) if !text.chars().any(char::is_control) => format!("\"{text}\""),
        _ => {
            use std::fmt::Write as _;
            let mut hex = String::with_capacity(value.len() * 2 + 2);
            hex.push_str("0x");
            for byte in value {
                let _ = write!(hex, "{byte:02x}");
            }
            hex
        }
    }
}

// ---------------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------------

/// The ordered, labelled input binding an artifact carries, together with the
/// SHA-256 digest of its canonical encoding.
///
/// See the [module docs](self) for the injective framing, why the components
/// travel rather than only their digest, and why labels are not a map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    components: Vec<IdentityComponent>,
    digest: [u8; 32],
}

impl Default for Identity {
    /// The empty identity — see [`Identity::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl Identity {
    /// The empty identity: no components, and the digest of the empty byte
    /// string.
    ///
    /// An artifact whose codec binds no inputs is a real case (a product
    /// derived from nothing but its own literal contents), and it is spelled by
    /// an empty identity rather than by an absent one — the same totality law
    /// the section directory follows.
    #[must_use]
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            digest: Sha256::digest(b"").into(),
        }
    }

    /// Append a labelled component, recomputing the digest.
    ///
    /// Infallible by design: a caller assembling an identity has no useful
    /// recovery at the push site. See the [module docs](self) on why duplicate
    /// labels are permitted.
    pub fn push(&mut self, label: &str, value: &[u8]) -> &mut Self {
        self.components.push(IdentityComponent {
            label: label.to_owned(),
            value: value.to_vec(),
        });
        self.refresh();
        self
    }

    /// Append a labelled component, by value — the chaining form of
    /// [`push`](Self::push) for building an identity in one expression.
    #[must_use]
    pub fn with(mut self, label: &str, value: &[u8]) -> Self {
        self.push(label, value);
        self
    }

    /// The ordered components, exactly as encoded.
    #[must_use]
    pub fn components(&self) -> &[IdentityComponent] {
        &self.components
    }

    /// The value of the FIRST component carrying `label`, or `None`.
    ///
    /// A convenience for the common unique-label case; the ordered sequence,
    /// not this lookup, is the identity (see the [module docs](self)).
    #[must_use]
    pub fn component(&self, label: &str) -> Option<&[u8]> {
        self.components
            .iter()
            .find(|component| component.label == label)
            .map(IdentityComponent::value)
    }

    /// The SHA-256 digest of [`to_bytes`](Self::to_bytes) — the value the
    /// artifact header seals and a constant-size equality check compares.
    #[must_use]
    pub fn digest(&self) -> &[u8; 32] {
        &self.digest
    }

    /// The canonical encoding: for each component in order,
    /// `varint(label_len) ‖ label ‖ varint(value_len) ‖ value`.
    ///
    /// Injective over the component sequence — see the [module docs](self).
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for component in &self.components {
            write_varint(&mut out, component.label.len() as u64);
            out.extend_from_slice(component.label.as_bytes());
            write_varint(&mut out, component.value.len() as u64);
            out.extend_from_slice(&component.value);
        }
        out
    }

    /// Decode [`to_bytes`](Self::to_bytes)'s output, fail-closed.
    ///
    /// Refuses a non-UTF-8 label, a length prefix that runs past the end of the
    /// buffer, and — because a format with two spellings of one meaning cannot
    /// say what it seals — any buffer that does not re-encode to itself byte for
    /// byte (a padded LEB128 prefix being the case that matters).
    ///
    /// # Errors
    ///
    /// [`ArtifactError::Truncated`] if a length prefix promises bytes the buffer
    /// does not hold; [`ArtifactError::Malformed`] for a non-UTF-8 label, an
    /// over-long length prefix, or a non-canonical encoding.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ArtifactError> {
        let mut pos = 0usize;
        let mut components = Vec::new();
        while pos < bytes.len() {
            let label = read_part(bytes, &mut pos)?;
            let label = std::str::from_utf8(label)
                .map_err(|_| ArtifactError::Malformed("artifact identity: label is not UTF-8"))?
                .to_owned();
            let value = read_part(bytes, &mut pos)?.to_vec();
            components.push(IdentityComponent { label, value });
        }

        let decoded = Self::from_components(components);
        if decoded.to_bytes() != bytes {
            return Err(ArtifactError::Malformed(
                "artifact identity: encoding is not canonical",
            ));
        }
        Ok(decoded)
    }

    /// Every way `caller`'s identity differs from this one, in position order.
    ///
    /// Empty exactly when the two identities are equal — and therefore exactly
    /// when their digests are equal, the framing being injective.
    #[must_use]
    pub fn mismatches(&self, caller: &Self) -> Vec<IdentityMismatch> {
        let mut out = Vec::new();
        let width = self.components.len().max(caller.components.len());
        for position in 0..width {
            match (
                self.components.get(position),
                caller.components.get(position),
            ) {
                (Some(mine), Some(theirs)) => {
                    if mine.label == theirs.label {
                        if mine.value != theirs.value {
                            out.push(IdentityMismatch::ValueDiffers {
                                position,
                                label: mine.label.clone(),
                                artifact: mine.value.clone(),
                                caller: theirs.value.clone(),
                            });
                        }
                    } else {
                        out.push(IdentityMismatch::LabelDiffers {
                            position,
                            artifact: mine.label.clone(),
                            caller: theirs.label.clone(),
                        });
                    }
                }
                (Some(mine), None) => out.push(IdentityMismatch::MissingFromCaller {
                    position,
                    label: mine.label.clone(),
                    artifact: mine.value.clone(),
                }),
                (None, Some(theirs)) => out.push(IdentityMismatch::UnexpectedFromCaller {
                    position,
                    label: theirs.label.clone(),
                    caller: theirs.value.clone(),
                }),
                (None, None) => unreachable!("position is below both lengths' maximum"),
            }
        }
        out
    }

    /// Build an identity from an already-ordered component list, taking its
    /// digest once.
    fn from_components(components: Vec<IdentityComponent>) -> Self {
        let mut identity = Self {
            components,
            digest: [0u8; 32],
        };
        identity.refresh();
        identity
    }

    /// Recompute the cached digest from the current components.
    fn refresh(&mut self) {
        self.digest = Sha256::digest(self.to_bytes()).into();
    }
}

/// Read one `varint(len) ‖ bytes` part at `*pos`, advancing past it.
fn read_part<'a>(bytes: &'a [u8], pos: &mut usize) -> Result<&'a [u8], ArtifactError> {
    let len = read_varint(bytes, pos).map_err(|error| match error {
        PackBitsError::Truncated { .. } => ArtifactError::Truncated,
        PackBitsError::Malformed(_) => {
            ArtifactError::Malformed("artifact identity: length prefix exceeds 64 bits")
        }
    })?;
    let len = usize::try_from(len)
        .map_err(|_| ArtifactError::Malformed("artifact identity: length exceeds usize"))?;
    let end = pos.checked_add(len).ok_or(ArtifactError::Malformed(
        "artifact identity: span overflows",
    ))?;
    let part = bytes.get(*pos..end).ok_or(ArtifactError::Truncated)?;
    *pos = end;
    Ok(part)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Identity {
        Identity::new()
            .with("base", b"https://example.org/shapes")
            .with("shapes-rdfc", &[0xde, 0xad, 0xbe, 0xef])
            .with("empty", b"")
    }

    #[test]
    fn identity_components_decode_to_what_was_encoded() {
        let identity = sample();
        let bytes = identity.to_bytes();
        let decoded = Identity::from_bytes(&bytes).expect("canonical identity decodes");

        assert_eq!(decoded, identity);
        assert_eq!(decoded.digest(), identity.digest());
        assert_eq!(decoded.components().len(), 3);
        assert_eq!(decoded.components()[0].label(), "base");
        assert_eq!(
            decoded.components()[0].value(),
            b"https://example.org/shapes"
        );
        assert_eq!(decoded.components()[1].value(), &[0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(decoded.components()[2].label(), "empty");
        assert_eq!(decoded.components()[2].value(), b"");
        assert_eq!(
            decoded.component("shapes-rdfc"),
            Some(&[0xde, 0xad, 0xbe, 0xef][..])
        );
        assert_eq!(decoded.component("absent"), None);
    }

    #[test]
    fn identity_is_injective_across_field_boundaries() {
        // The exact collision length-prefixing exists to prevent: without a
        // prefix on BOTH halves, each of these flattens to `abc`.
        let left = Identity::new().with("a", b"bc");
        let right = Identity::new().with("ab", b"c");

        assert_ne!(left.to_bytes(), right.to_bytes());
        assert_ne!(left.digest(), right.digest());

        // Same total bytes, different split across the component boundary.
        let split_a = Identity::new().with("x", b"y").with("z", b"w");
        let split_b = Identity::new().with("xy", b"").with("zw", b"");
        assert_ne!(split_a.digest(), split_b.digest());
    }

    #[test]
    fn empty_identity_round_trips() {
        let identity = Identity::new();
        assert_eq!(identity.to_bytes(), Vec::<u8>::new());
        assert_eq!(identity.digest(), &<[u8; 32]>::from(Sha256::digest(b"")));
        assert_eq!(Identity::from_bytes(&[]).expect("empty decodes"), identity);
    }

    #[test]
    fn identity_digest_tracks_every_push() {
        let mut identity = Identity::new();
        let before = *identity.digest();
        identity.push("base", b"https://example.org/a");
        let after = *identity.digest();
        assert_ne!(before, after);
        identity.push("base", b"https://example.org/a");
        assert_ne!(
            after,
            *identity.digest(),
            "a repeated component still counts"
        );
    }

    #[test]
    fn duplicate_labels_stay_distinct_components() {
        let identity = Identity::new()
            .with("graph", b"https://example.org/g1")
            .with("graph", b"https://example.org/g2");
        assert_eq!(identity.components().len(), 2);
        assert_eq!(
            identity.component("graph"),
            Some(&b"https://example.org/g1"[..])
        );
        let decoded = Identity::from_bytes(&identity.to_bytes()).expect("decodes");
        assert_eq!(decoded, identity);
    }

    #[test]
    fn identity_mismatch_names_the_component() {
        let artifact = Identity::new().with("base", b"https://example.org/a");
        let caller = Identity::new().with("base", b"https://example.org/b");

        let mismatches = artifact.mismatches(&caller);
        assert_eq!(mismatches.len(), 1);
        assert_eq!(
            mismatches[0].to_string(),
            "component `base` (position 0): artifact has \"https://example.org/a\", \
             caller supplied \"https://example.org/b\""
        );
    }

    #[test]
    fn identity_mismatch_reports_shape_differences() {
        let artifact = Identity::new()
            .with("base", b"https://example.org/a")
            .with("vocab", b"https://example.org/v");
        let caller = Identity::new().with("base", b"https://example.org/a");
        let mismatches = artifact.mismatches(&caller);
        assert_eq!(
            mismatches,
            vec![IdentityMismatch::MissingFromCaller {
                position: 1,
                label: "vocab".to_owned(),
                artifact: b"https://example.org/v".to_vec(),
            }]
        );
        assert!(
            mismatches[0]
                .to_string()
                .contains("caller supplied nothing")
        );

        let reversed = caller.mismatches(&artifact);
        assert!(matches!(
            reversed.as_slice(),
            [IdentityMismatch::UnexpectedFromCaller { position: 1, .. }]
        ));

        let relabelled = Identity::new().with("other", b"https://example.org/a");
        assert!(matches!(
            caller.mismatches(&relabelled).as_slice(),
            [IdentityMismatch::LabelDiffers { position: 0, .. }]
        ));
    }

    #[test]
    fn equal_identities_report_no_mismatch() {
        // The valid neighbour of every mismatch above: identical inputs are
        // reported as identical, not merely as "digest equal".
        let identity = sample();
        assert_eq!(identity.mismatches(&sample()), Vec::new());
        assert_eq!(identity.digest(), sample().digest());
    }

    #[test]
    fn refuses_non_canonical_length_prefix() {
        let identity = Identity::new().with("a", b"b");
        let canonical = identity.to_bytes();
        assert_eq!(canonical, vec![1, b'a', 1, b'b']);
        // A padded LEB128 `1` (0x81 0x00) decodes to the same value, so without
        // the canonical-form check this would be a second spelling.
        let padded = vec![0x81, 0x00, b'a', 1, b'b'];
        assert!(matches!(
            Identity::from_bytes(&padded),
            Err(ArtifactError::Malformed(_))
        ));
        // Valid neighbour: the canonical spelling of the very same components.
        assert_eq!(
            Identity::from_bytes(&canonical).expect("canonical decodes"),
            identity
        );
    }

    #[test]
    fn refuses_truncated_identity() {
        let identity = sample();
        let bytes = identity.to_bytes();
        let truncated = &bytes[..bytes.len() - 1];
        assert!(matches!(
            Identity::from_bytes(truncated),
            Err(ArtifactError::Truncated)
        ));
        // Valid neighbour: one more byte and it is the whole thing again.
        assert!(Identity::from_bytes(&bytes).is_ok());
    }

    #[test]
    fn refuses_non_utf8_label() {
        let mut bytes = Vec::new();
        write_varint(&mut bytes, 2);
        bytes.extend_from_slice(&[0xff, 0xfe]);
        write_varint(&mut bytes, 0);
        assert!(matches!(
            Identity::from_bytes(&bytes),
            Err(ArtifactError::Malformed(_))
        ));
        // Valid neighbour: the same framing with a UTF-8 label decodes.
        let mut ok = Vec::new();
        write_varint(&mut ok, 2);
        ok.extend_from_slice(b"ab");
        write_varint(&mut ok, 0);
        assert_eq!(
            Identity::from_bytes(&ok).expect("utf-8 label decodes"),
            Identity::new().with("ab", b"")
        );
    }

    #[test]
    fn renders_binary_values_as_hex_and_text_as_text() {
        assert_eq!(
            render_value(b"https://example.org/a"),
            "\"https://example.org/a\""
        );
        assert_eq!(render_value(&[0x00, 0xff]), "0x00ff");
        assert_eq!(render_value(b""), "\"\"");
    }
}
