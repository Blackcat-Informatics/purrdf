// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The **IR-boundary absoluteness invariant**: no relative IRI reference is ever
//! representable as an IRI term inside this kernel.
//!
//! # Why this lives at the term table and not at the codecs
//!
//! An IRI term in the IR carries no base: the frozen dataset is a set of resolved,
//! dataset-independent identities, and there is no `@base` alongside it to resolve a
//! reference against later. A relative reference interned verbatim is therefore not
//! "a term awaiting resolution" — it is a term whose identity is *unknowable*, which
//! then serializes as invalid N-Triples/N-Quads and compares unequal to the very
//! resource it was meant to name.
//!
//! Enforcing that at each codec seam means enforcing it N times, once per codec, with
//! N chances to disagree — and leaves every non-codec ingress (GTS import, pack
//! rehydration, the paged dictionary, SPARQL `INSERT DATA`, projection lifts, the
//! language bindings' quad constructors) free to mint one anyway. So the check lives
//! here, at the store-once term tables, where **every** ingress necessarily passes.
//!
//! # The rule has exactly one owner
//!
//! [`check_absolute`] delegates to [`purrdf_iri::BaseScope::resolve`] on an **empty**
//! scope, which is the literal truth of the situation: the IR has no base in scope,
//! ever. That is the same function every base-aware codec calls, so the IR boundary
//! and the codecs cannot drift apart, and the failure carries the workspace's shared
//! [`IriError::diagnostic_code`] spelling (`iri-relative-no-base` for a scheme-less
//! reference) rather than a code invented here.
//!
//! # Miss-path only
//!
//! The term tables are **store-once**: a repeated intern of an already-interned string
//! is a hash lookup that returns the existing id and never reaches this module. Callers
//! must therefore invoke [`check_absolute`] only from the MISS branch, so validation
//! cost is paid once per *distinct* IRI rather than once per intern. Every call site in
//! this crate sits inside a miss branch for that reason; see
//! `crates/rdf-core/benches/ir_layout.rs` (`intern_absoluteness` group) for the
//! measured hit/miss split.

use purrdf_iri::{BaseScope, IriError};

/// Reject `iri` unless it is an **absolute** IRI reference (RFC-3987 grammar plus a
/// scheme).
///
/// Returns `Ok(())` for an absolute IRI, and otherwise the typed [`IriError`] that
/// [`BaseScope::resolve`] produces for the same string with no base in scope:
///
/// * a scheme-less reference (including the empty same-document reference `<>`) is
///   [`IriError::NoBase`], code `iri-relative-no-base`;
/// * a reference that is not well-formed at all keeps its own precise parse error
///   (`iri-bad-scheme`, `iri-disallowed-char`, …), so a malformed IRI is never
///   misreported as a missing base — which would send the caller off to add a `@base`
///   that cannot help.
///
/// # Performance
///
/// Call this **only on the miss path** of a store-once interner. It is O(len) in the
/// IRI and allocates the parsed [`purrdf_iri::Iri`] once; interning an
/// already-interned string must not reach it at all.
///
/// # Errors
///
/// [`IriError`] as described above.
pub(crate) fn check_absolute(iri: &str) -> Result<(), IriError> {
    // `BaseScope::empty()` is a `Vec::new()` — no allocation — and its `resolve` is
    // exactly the "no base is in scope" arm of RFC-3986 §5.1.4. Reusing it (rather
    // than re-spelling "parse, then test `has_scheme`") is what keeps this invariant
    // and the codecs' base handling provably the same rule.
    BaseScope::empty().resolve(iri).map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_iris_pass() {
        for iri in [
            "http://example.org/x",
            "https://example.org/a/b?q=1#f",
            "urn:uuid:0b7f0a1e-0000-4000-8000-000000000000",
            "blake3:0123456789abcdef",
            "file:///tmp/x",
            // RFC-3987: non-ASCII code points are permitted verbatim.
            "http://example.org/caf\u{e9}",
        ] {
            assert!(check_absolute(iri).is_ok(), "{iri} must be accepted");
        }
    }

    #[test]
    fn relative_references_are_rejected_with_the_shared_code() {
        for iri in ["foo", "/abs/path", "../up", "./here", "?q=1", "#frag"] {
            let err = check_absolute(iri).expect_err("a relative reference is rejected");
            assert_eq!(err.diagnostic_code(), "iri-relative-no-base", "{iri}");
            assert!(matches!(err, IriError::NoBase { .. }), "{iri}");
        }
    }

    /// The empty IRI (`<>` — the same-document reference) is the defect this
    /// invariant was written for, and it is a MISSING BASE, not merely "empty".
    #[test]
    fn empty_iri_is_a_missing_base_not_an_empty_string() {
        let err = check_absolute("").expect_err("the empty IRI is relative");
        assert_eq!(err.diagnostic_code(), "iri-relative-no-base");
        assert_eq!(
            err,
            IriError::NoBase {
                reference: String::new()
            }
        );
    }

    /// A malformed IRI keeps its own diagnostic: telling the caller to "add a base"
    /// would be a lie, because no base can fix a disallowed character.
    #[test]
    fn malformed_iris_keep_their_own_diagnostic() {
        let err = check_absolute("http://example.org/a b").expect_err("space is disallowed");
        assert_eq!(err.diagnostic_code(), "iri-disallowed-char");
    }
}
