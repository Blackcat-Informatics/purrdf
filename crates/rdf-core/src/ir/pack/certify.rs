// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The certified-projection verifier (Task 7 of the succinct-pack-codec feature):
//! an INDEPENDENT check that a pack's stored RDFC-1.0 digest genuinely matches its
//! own contents, on top of the per-section SHA-256 integrity
//! [`PackView::from_bytes`] already enforces.
//!
//! # Why a second, independent digest recompute
//!
//! [`super::container::PackBuilder::build_bytes`] stores a SHA-256 digest of the SOURCE dataset's
//! RDFC-1.0 canonical N-Quads in the container header (see
//! [`super::container`]'s module docs, the `rdfc_digest` header field). That value
//! is trusted data written once at build time — nothing re-derives it from the
//! pack's own sections afterward. [`verify_pack`] closes that gap: it walks the
//! opened [`PackView`] through the [`crate::DatasetView`] seam, RE-INTERNS every
//! quad and RDF-1.2 side-table row into a fresh [`crate::RdfDatasetBuilder`],
//! canonicalizes THAT reconstruction exactly as
//! [`super::container::PackBuilder::build_bytes`] does,
//! and compares the two digests. Only a pack whose stored digest agrees with its
//! own decoded contents is a **certified read-only projection** of its source
//! dataset — a tampered `rdfc_digest` header field (not covered by any section's
//! SHA-256; see [`super::container`]'s directory layout) is caught here, not by
//! [`PackView::from_bytes`].
//!
//! # Reconstruction scope: base quads AND the RDF-1.2 overlay
//!
//! [`crate::ir::canon`]'s `collect_components` folds THREE sources into the
//! canonical form: the dataset's base quads, its reifier bindings, and its
//! statement annotations (see that module's `SUBSUME + EXTEND` doc section —
//! reifiers/annotations fold into the hash via reserved `urn:purrdf:rdfc:`
//! sentinel predicates/graphs, so the reifier COUNT and annotation PRESENCE are
//! observable in the canonical digest). A reconstruction that only replayed
//! [`DatasetView::quads`] and dropped the side-tables would digest a
//! *different*, lossy dataset and never match the stored certificate. This module
//! therefore also replays [`DatasetView::reifier_quads`] and
//! [`DatasetView::annotation_quads`] into the builder's dedicated
//! `push_reifier_in_graph`/`push_annotation_in_graph` side-table entry points
//! (NOT `push_quad` — the side-table rows are a distinct structural component,
//! not ordinary quads; see [`crate::ir::canon`]'s `Component` enum).

use sha2::{Digest, Sha256};

use crate::dataset_view::DatasetView;
use crate::{CanonHash, RdfDatasetBuilder, RdfLiteral, TermId, TermRef, TermValue};

use super::container::{PackError, PackView};

// ---------------------------------------------------------------------------
// PackDigest
// ---------------------------------------------------------------------------

/// A verified SHA-256 RDFC-1.0 digest: the output of [`verify_pack`] on success.
///
/// Distinct from a bare `[u8; 32]` so a caller cannot confuse an UNVERIFIED digest
/// (e.g. one merely read off [`PackView::rdfc_digest`] without recomputing it) with
/// one [`verify_pack`] has independently certified.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PackDigest([u8; 32]);

impl PackDigest {
    /// The raw 32 digest bytes.
    #[inline]
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The lowercase-hex rendering of the digest (64 chars).
    #[must_use]
    pub fn to_hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in &self.0 {
            use std::fmt::Write as _;
            let _ = write!(s, "{b:02x}");
        }
        s
    }
}

impl std::fmt::Debug for PackDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PackDigest").field(&self.to_hex()).finish()
    }
}

impl std::fmt::Display for PackDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

// ---------------------------------------------------------------------------
// Reconstruction: PackView -> RdfDatasetBuilder
// ---------------------------------------------------------------------------

/// Resolve a [`DatasetView`] id to its dataset-INDEPENDENT [`TermValue`],
/// recursing through a literal's datatype and a triple term's `(s, p, o)`
/// components. Mirrors the `to_value` helper `tests/paged_backend.rs` uses to
/// compare a `PagedDataset` against a plain `RdfDataset` by value — the same
/// by-value bridge lets this module compare a `PackView`'s reconstruction
/// against the pack's own claimed identity.
fn to_value<V: DatasetView>(v: &V, id: V::Id) -> TermValue {
    match v.resolve(id) {
        TermRef::Iri(s) => TermValue::iri(s),
        TermRef::Blank { label, scope } => TermValue::Blank {
            label: label.to_owned(),
            scope,
        },
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            let datatype = match v.resolve(datatype) {
                TermRef::Iri(s) => s.to_owned(),
                _ => unreachable!("a literal's datatype always resolves to an IRI"),
            };
            TermValue::Literal {
                lexical_form: lexical.to_owned(),
                datatype,
                language: language.map(str::to_owned),
                direction,
            }
        }
        TermRef::Triple { s, p, o } => TermValue::Triple {
            s: Box::new(to_value(v, s)),
            p: Box::new(to_value(v, p)),
            o: Box::new(to_value(v, o)),
        },
    }
}

/// Intern one dataset-independent value into a builder, recursing for triple
/// terms — the by-value inverse of [`to_value`], and the reconstruction step
/// that re-mints a builder-local [`TermId`] for a value read off the pack.
fn intern_value(b: &mut RdfDatasetBuilder, v: &TermValue) -> TermId {
    match v {
        TermValue::Iri(s) => b.intern_iri(s),
        TermValue::Blank { label, scope } => b.intern_blank(label, *scope),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => b.intern_literal(RdfLiteral {
            lexical_form: lexical_form.clone(),
            datatype: Some(datatype.clone()),
            language: language.clone(),
            direction: *direction,
        }),
        TermValue::Triple { s, p, o } => {
            let s = intern_value(b, s);
            let p = intern_value(b, p);
            let o = intern_value(b, o);
            b.intern_triple(s, p, o)
        }
    }
}

/// Resolve a view id straight to a freshly interned builder-local [`TermId`],
/// composing [`to_value`] and [`intern_value`] for the common one-shot case.
fn reintern<V: DatasetView>(b: &mut RdfDatasetBuilder, v: &V, id: V::Id) -> TermId {
    let value = to_value(v, id);
    intern_value(b, &value)
}

/// Independently reconstruct an `RdfDatasetBuilder` from `view`'s
/// [`DatasetView`] surface: every base quad, then every reifier binding, then
/// every statement annotation — the exact three components
/// [`crate::ir::canon::collect_components`] folds into the RDFC-1.0 digest (see
/// the [module docs](self)). Each blank's original `(label, scope)` (C0.2)
/// round-trips through [`to_value`]/[`intern_value`] unchanged; that choice is
/// moot for the digest either way, since RDFC-1.0 canonicalizes blank labels
/// away entirely (structure alone drives the canonical `_:c14nN` assignment).
fn reconstruct(view: &PackView<'_>) -> RdfDatasetBuilder {
    let mut b = RdfDatasetBuilder::new();

    for q in view.quads() {
        let s = reintern(&mut b, view, q.s);
        let p = reintern(&mut b, view, q.p);
        let o = reintern(&mut b, view, q.o);
        let g = q.g.map(|g| reintern(&mut b, view, g));
        b.push_quad(s, p, o, g);
    }

    // `reifier_quads` projects each `(reifier, triple, graph)` side-table row as a
    // virtual `reifier rdf:reifies triple` quad (predicate fixed at `rdf:reifies`);
    // push it back through the dedicated reifier entry point, NOT `push_quad` — see
    // the [module docs](self).
    for q in view.reifier_quads() {
        let reifier = reintern(&mut b, view, q.s);
        let triple = reintern(&mut b, view, q.o);
        let g = q.g.map(|g| reintern(&mut b, view, g));
        b.push_reifier_in_graph(reifier, triple, g);
    }

    for q in view.annotation_quads() {
        let reifier = reintern(&mut b, view, q.s);
        let p = reintern(&mut b, view, q.p);
        let o = reintern(&mut b, view, q.o);
        let g = q.g.map(|g| reintern(&mut b, view, g));
        b.push_annotation_in_graph(reifier, p, o, g);
    }

    b
}

// ---------------------------------------------------------------------------
// verify_pack / pack_digest
// ---------------------------------------------------------------------------

/// Open, structurally verify, and CERTIFY a pack: [`PackView::from_bytes`] first
/// (magic/version/every section's SHA-256/each submodule's own structural
/// validation), then independently reconstruct the dataset the pack claims to
/// encode and recompute its RDFC-1.0 SHA-256 digest, then compare that recompute
/// to the pack's own stored `rdfc_digest` header field.
///
/// A pack that passes both checks is a **certified read-only projection**: its
/// contents are byte-intact (per-section integrity) AND its claimed dataset
/// identity is genuinely reproducible from those contents (the RDFC-1.0 recompute)
/// — not merely a header field nothing else corroborates.
///
/// # Errors
///
/// - Any [`PackError`] [`PackView::from_bytes`] can return (bad magic, unsupported
///   version, truncation, a section's SHA-256 failing its stored digest, or a
///   submodule's own structural validation failing).
/// - [`PackError::CanonBudgetExceeded`] if the reconstructed dataset's RDFC-1.0
///   canonicalization exceeds its call budget (a pathologically symmetric blank
///   graph — untrusted input fails closed here rather than hanging or panicking).
/// - [`PackError::RdfcDigestMismatch`] if the independently recomputed digest
///   disagrees with the pack's stored `rdfc_digest` header field — the pack's
///   contents do not actually canonicalize to the identity it claims.
pub fn verify_pack(bytes: &[u8]) -> Result<PackDigest, PackError> {
    let view = PackView::from_bytes(bytes)?;

    let builder = reconstruct(&view);
    let reconstructed = builder
        .freeze()
        .expect("a pack reconstruction only ever pushes structurally valid rows");

    let canonicalized = crate::try_canonicalize_with(&reconstructed, CanonHash::Sha256)
        .map_err(|_| PackError::CanonBudgetExceeded)?;
    let computed: [u8; 32] = Sha256::digest(canonicalized.nquads.as_bytes()).into();

    let expected = view.rdfc_digest();
    if computed != expected {
        return Err(PackError::RdfcDigestMismatch { expected, computed });
    }

    Ok(PackDigest(expected))
}

/// Read a pack's stored RDFC-1.0 digest AFTER structural validation, without the
/// (more expensive) independent recompute [`verify_pack`] performs.
///
/// [`PackView::from_bytes`] still runs in full (magic/version/every section's
/// SHA-256/each submodule's structural validation), so the returned digest is
/// backed by a structurally sound pack — it is simply not yet CORROBORATED
/// against the pack's own decoded contents the way [`verify_pack`]'s
/// [`PackDigest`] is. Prefer this for a cheap "is this pack well-formed and what
/// does it claim its identity is" probe; prefer [`verify_pack`] whenever the
/// caller actually trusts the returned digest as the pack's certified identity.
///
/// # Errors
///
/// Any [`PackError`] [`PackView::from_bytes`] can return.
pub fn pack_digest(bytes: &[u8]) -> Result<[u8; 32], PackError> {
    Ok(PackView::from_bytes(bytes)?.rdfc_digest())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_digest_to_hex_is_64_lowercase_hex_chars() {
        let digest = PackDigest([0xab; 32]);
        let hex = digest.to_hex();
        assert_eq!(hex.len(), 64);
        assert!(
            hex.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "to_hex must be lowercase hex: {hex}"
        );
        assert_eq!(hex, "ab".repeat(32));
    }

    #[test]
    fn pack_digest_debug_and_display_agree_with_to_hex() {
        let digest = PackDigest([0x0f; 32]);
        assert_eq!(format!("{digest}"), digest.to_hex());
        assert!(format!("{digest:?}").contains(&digest.to_hex()));
    }
}
