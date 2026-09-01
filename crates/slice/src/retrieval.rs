// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The RFC-8089 `file://` **retrieval IRI** of an on-disk document — the workspace's
//! one implementation of RFC-3986 §5.1.3.
//!
//! # Why this lives here
//!
//! [`purrdf_iri::BaseScope`] implements §5.1.1 (an in-document `@base` / `BASE` /
//! `xml:base` directive) and §5.1.2 (a base the caller supplied), and hard-fails per
//! §5.1.4 when neither is present. It deliberately implements NEITHER half of §5.1.3,
//! because a crate that is handed BYTES has no retrieval IRI to fall back on: inventing
//! one there would break byte determinism, diverge across the surfaces that genuinely
//! have no retrieval IRI (stdin, wasm, the C ABI), and leak local filesystem paths into
//! published RDF.
//!
//! §5.1.3 therefore belongs to whichever surface actually opened the file. In this
//! workspace exactly two do — `purrdf-slice`, which walks a slice tree off disk, and
//! `purrdf-cli`, which reads a named input — so the derivation lives once, here, in the
//! only *library* crate that already reads the filesystem, and `purrdf-cli` consumes it
//! through the `purrdf` umbrella it already depends on for exactly this reason. Nothing
//! filesystem-shaped crosses into `purrdf-iri` or `purrdf-rdf`, precisely as
//! `purrdf_iri::base`'s module documentation states.
//!
//! # What it does and does not change
//!
//! The base is consulted only when a document actually carries a **relative** IRI
//! reference. A document whose every IRI is absolute parses to the same dataset, and
//! therefore to the same canonical bytes and the same semantic digest, whether or not a
//! base was in scope — so supplying the retrieval IRI does not make an all-absolute
//! artifact's committed output depend on where the tree was checked out.
//!
//! A document that DOES carry a relative reference is genuinely under-determined without
//! its retrieval IRI: RFC-3986 says that reference denotes something different depending
//! on where the document was retrieved from, and that is a property of RDF, not of this
//! code. The alternative is not a machine-independent answer, it is [`IriError::NoBase`]
//! — a refusal to read a file whose base was sitting in the caller's hand.
//!
//! [`IriError::NoBase`]: purrdf_iri::IriError::NoBase

use std::fmt::Write as _;
use std::path::Path;

use purrdf_iri::BaseIri;

use crate::error::SliceError;

/// The RFC-8089 `file://` IRI of `path` — the document's retrieval IRI (RFC-3986
/// §5.1.3), validated as an absolute base.
///
/// The path is **canonicalized** first, so the IRI depends on neither the process
/// working directory nor on `.`/`..` segments in the argument, and a symlinked artifact
/// and its target agree. A path that cannot be expressed as a `file://` IRI — because it
/// does not exist, or is not valid UTF-8 — is a hard error, never a silent fall back to
/// "no base".
///
/// # Errors
///
/// [`SliceError::InvalidPath`] when `path` cannot be canonicalized, is not valid UTF-8,
/// or does not yield a parseable absolute IRI.
pub fn retrieval_base_iri(path: &Path) -> Result<BaseIri, SliceError> {
    let absolute = std::fs::canonicalize(path).map_err(|error| {
        SliceError::InvalidPath(format!(
            "{}: cannot derive the document's file:// retrieval IRI ({error})",
            path.display()
        ))
    })?;
    let text = absolute.to_str().ok_or_else(|| {
        SliceError::InvalidPath(format!(
            "{}: the path is not valid UTF-8, so it has no file:// IRI",
            path.display()
        ))
    })?;

    // An empty authority is the RFC-8089 local-file form: `file:///path`.
    let iri = format!("file://{}", percent_encode_path(text));
    BaseIri::parse(&iri).map_err(|error| {
        SliceError::InvalidPath(format!(
            "{}: the path has no usable file:// IRI ({error})",
            path.display()
        ))
    })
}

/// Percent-encode an absolute filesystem path into RFC-3986 §3.3 `path-abempty`.
///
/// `unreserved` / `sub-delims` / `:` / `@` survive verbatim, as do the `/` separators;
/// everything else — space, `#`, `?`, `%`, and every non-ASCII byte — is percent-encoded,
/// so the result round-trips as a URI instead of re-parsing as a query or a fragment.
fn percent_encode_path(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len() + 8);
    for &byte in text.as_bytes() {
        let keep = byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'-' | b'.'
                    | b'_'
                    | b'~'
                    | b'!'
                    | b'$'
                    | b'&'
                    | b'\''
                    | b'('
                    | b')'
                    | b'*'
                    | b'+'
                    | b','
                    | b';'
                    | b'='
                    | b':'
                    | b'@'
                    | b'/'
            );
        if keep {
            encoded.push(byte as char);
        } else {
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_real_file_yields_an_absolute_file_iri() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.ttl");
        std::fs::write(&path, b"# empty\n").expect("write");

        let base = retrieval_base_iri(&path).expect("a real file has a retrieval IRI");
        assert!(
            base.as_str().starts_with("file:///"),
            "expected an RFC-8089 local-file IRI, got {base}"
        );
        assert!(
            base.as_str().ends_with("/doc.ttl"),
            "the IRI must name the document, got {base}"
        );
        // The retrieval IRI is a usable base: `<>` is the document itself.
        assert_eq!(base.resolve("").expect("resolve").as_str(), base.as_str());
    }

    #[test]
    fn dot_segments_and_the_working_directory_do_not_leak_in() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join("sub")).expect("mkdir");
        let path = dir.path().join("sub").join("doc.ttl");
        std::fs::write(&path, b"# empty\n").expect("write");

        let direct = retrieval_base_iri(&path).expect("direct path");
        let indirect = retrieval_base_iri(&dir.path().join("sub").join(".").join("doc.ttl"))
            .expect("dot-segment path");
        assert_eq!(direct.as_str(), indirect.as_str());
        assert!(
            !direct.as_str().contains("/./"),
            "canonicalization must remove dot segments, got {direct}"
        );
    }

    #[test]
    fn a_space_bearing_name_is_percent_encoded() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("a doc#1.ttl");
        std::fs::write(&path, b"# empty\n").expect("write");

        let base = retrieval_base_iri(&path).expect("percent-encoded retrieval IRI");
        assert!(
            base.as_str().ends_with("/a%20doc%231.ttl"),
            "space and '#' must be percent-encoded, got {base}"
        );
        // The `#` really is encoded, not carried as a fragment delimiter.
        assert_eq!(base.as_iri().fragment(), None);
    }

    #[test]
    fn a_missing_file_is_a_hard_error_not_a_silent_no_base() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("nope.ttl");
        match retrieval_base_iri(&missing) {
            Err(SliceError::InvalidPath(message)) => {
                assert!(
                    message.contains("nope.ttl"),
                    "the error must name the path, got: {message}"
                );
            }
            other => panic!("expected InvalidPath, got {other:?}"),
        }
    }
}
