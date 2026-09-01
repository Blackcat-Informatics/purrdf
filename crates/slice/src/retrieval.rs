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
//! §5.1.3 therefore belongs to whichever surface actually opened the file. Three do —
//! `purrdf-slice`, which walks a slice tree off disk; `purrdf-shapes`, whose shape-union
//! loader reads a repository's shape files; and `purrdf-cli`, which reads a named input.
//! The derivation lives once, here, in the only *library* crate in this workspace that
//! opens files, and the other two consume it (`purrdf-cli` through the `purrdf` umbrella
//! it already depends on). Nothing filesystem-shaped crosses into `purrdf-iri` or
//! `purrdf-rdf`, which is what keeps those two zero-dependency and wasm32-clean:
//! `std::fs::canonicalize` has no meaning on `wasm32-unknown-unknown`.
//!
//! # Platform path syntax is part of the derivation, not an afterthought
//!
//! A Windows path is a different language from an RFC-8089 IRI path and a POSIX path is
//! the same language, so [`file_iri_for_absolute_path`] splits by TARGET rather than by
//! guessing from the string's shape — on POSIX a `\` is an ordinary filename byte, and
//! rewriting it as a separator would corrupt a legal path. `cfg!` (not `#[cfg]`) keeps
//! both arms COMPILED everywhere, so the Windows derivation is unit-tested on a
//! Linux-only CI rather than merely believed.
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

    let iri = file_iri_for_absolute_path(text);
    BaseIri::parse(&iri).map_err(|error| {
        SliceError::InvalidPath(format!(
            "{}: the path has no usable file:// IRI ({error})",
            path.display()
        ))
    })
}

/// The RFC-8089 `file://` IRI of an ALREADY-ABSOLUTE platform path, touching no
/// filesystem.
///
/// Split from [`retrieval_base_iri`] because some callers need the string transformation
/// without the canonicalization — notably a diagnostic that must name the spelling which
/// *would* have worked for a path that does not exist yet.
///
/// Windows' path syntax is a different language from an RFC-8089 IRI path, and POSIX's is
/// the same language: on POSIX `\` is an ordinary filename byte, so rewriting it as a
/// separator would corrupt a legal path. The split is therefore by TARGET, not by
/// guessing from the string's shape, and `cfg!` (not `#[cfg]`) keeps both arms COMPILED
/// everywhere so the Windows derivation is unit-tested on a Linux-only CI.
#[must_use]
pub fn file_iri_for_absolute_path(text: &str) -> String {
    let (authority, path) = if cfg!(windows) {
        windows_file_iri_parts(text)
    } else {
        (String::new(), text.to_owned())
    };
    file_iri_from_parts(&authority, &path)
}

/// Assemble the two halves into a `file://` IRI, percent-encoding each under its own
/// RFC-3986 rule.
///
/// An EMPTY authority is the RFC-8089 local-file form (`file:///path`); a non-empty one is
/// the UNC host (`file://host/share/x`).
fn file_iri_from_parts(authority: &str, path: &str) -> String {
    format!(
        "file://{}{}",
        percent_encode(authority, b""),
        percent_encode(path, b":@/")
    )
}

/// Split a canonicalized WINDOWS path into the `(authority, path)` halves of its RFC-8089
/// `file://` IRI.
///
/// `std::fs::canonicalize` returns the EXTENDED-LENGTH form on Windows — `\\?\C:\dir\x.ttl`
/// for a drive path and `\\?\UNC\host\share\x.ttl` for a share — and neither prefix is part
/// of the name the IRI denotes. Left in place they percent-encode into one opaque authority
/// component (`file://%5C%5C%3F%5CC%3A%5C…`), which is not a local-file IRI at all and
/// against which a relative reference resolves under a fabricated authority. So the prefix
/// is stripped, a UNC host becomes the IRI's authority, and `\` becomes `/`.
///
/// A drive path's IRI path is `/C:/dir/x.ttl`: the leading `/` is required (RFC-3986
/// `path-abempty` after an authority), and the drive letter is an ordinary first segment.
fn windows_file_iri_parts(text: &str) -> (String, String) {
    let unc = text
        .strip_prefix(r"\\?\UNC\")
        .or_else(|| text.strip_prefix(r"\\.\UNC\"));
    if let Some(share) = unc {
        return split_unc_share(share);
    }
    let stripped = text
        .strip_prefix(r"\\?\")
        .or_else(|| text.strip_prefix(r"\\.\"));
    if let Some(local) = stripped {
        return (String::new(), absolute_iri_path(local));
    }
    // A UNC path that never went through `canonicalize` keeps its `\\host\share` spelling.
    if let Some(share) = text.strip_prefix(r"\\") {
        return split_unc_share(share);
    }
    (String::new(), absolute_iri_path(text))
}

/// Split `host\share\rest` into the IRI's authority and its path.
///
/// A share with no path component (`\\host\share`) yields `/share`, which is the whole IRI
/// path — never an empty one, which would make the IRI name the host rather than the file.
fn split_unc_share(share: &str) -> (String, String) {
    let end = share.find(['\\', '/']).unwrap_or(share.len());
    let (host, rest) = share.split_at(end);
    (host.to_owned(), absolute_iri_path(rest))
}

/// Rewrite a Windows path tail as an ABSOLUTE, slash-separated IRI path.
fn absolute_iri_path(text: &str) -> String {
    let slashed = text.replace('\\', "/");
    if slashed.starts_with('/') {
        slashed
    } else {
        format!("/{slashed}")
    }
}

/// Percent-encode one component of a `file://` IRI.
///
/// RFC-3986 §2.3 `unreserved` and §2.2 `sub-delims` survive verbatim in every component;
/// `extra` names what this component additionally keeps (`path-abempty` keeps `:`, `@` and
/// the `/` separators, a `reg-name` authority keeps neither). Everything else — space, `#`,
/// `?`, `%` and every non-ASCII byte — is percent-encoded, so the result round-trips as a
/// URI rather than re-parsing as a query or a fragment.
fn percent_encode(text: &str, extra: &[u8]) -> String {
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
            )
            || extra.contains(&byte);
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

    // ── Platform translation ───────────────────────────────────────────────────
    //
    // These exercise `windows_file_iri_parts` DIRECTLY rather than through
    // `file_iri_for_absolute_path`, so the Windows derivation is tested on a Linux-only
    // CI. That is the whole reason the target split is `cfg!` and not `#[cfg]`.

    /// The extended-length DRIVE form `canonicalize` returns on Windows becomes an
    /// RFC-8089 local-file IRI, not one opaque authority component.
    #[test]
    fn an_extended_length_drive_path_becomes_a_local_file_iri() {
        let (authority, path) = windows_file_iri_parts(r"\\?\C:\dir\x.ttl");
        assert_eq!(authority, "");
        assert_eq!(path, "/C:/dir/x.ttl");
        assert_eq!(
            file_iri_from_parts(&authority, &path),
            "file:///C:/dir/x.ttl"
        );
    }

    /// A plain drive path (one that never went through `canonicalize`) resolves the same way.
    #[test]
    fn a_plain_drive_path_becomes_the_same_local_file_iri() {
        let (authority, path) = windows_file_iri_parts(r"C:\dir\x.ttl");
        assert_eq!(
            file_iri_from_parts(&authority, &path),
            "file:///C:/dir/x.ttl"
        );
    }

    /// A UNC share becomes an AUTHORITY-bearing IRI: the host is the authority and the share
    /// is the first path segment.
    #[test]
    fn a_unc_share_puts_the_host_in_the_authority() {
        for text in [r"\\?\UNC\host\share\x.ttl", r"\\host\share\x.ttl"] {
            let (authority, path) = windows_file_iri_parts(text);
            assert_eq!(authority, "host", "{text}");
            assert_eq!(path, "/share/x.ttl", "{text}");
            assert_eq!(
                file_iri_from_parts(&authority, &path),
                "file://host/share/x.ttl",
                "{text}"
            );
        }
    }

    /// A share named with no further path still denotes the share, never the bare host.
    #[test]
    fn a_bare_unc_share_keeps_the_share_as_the_path() {
        let (authority, path) = windows_file_iri_parts(r"\\?\UNC\host\share");
        assert_eq!(file_iri_from_parts(&authority, &path), "file://host/share");
    }

    /// The backslash is a SEPARATOR only on the Windows derivation. A POSIX path carrying one
    /// is an ordinary filename byte, and the POSIX arm never calls the Windows split — so the
    /// byte percent-encodes rather than splitting the path.
    #[test]
    fn a_posix_path_encodes_a_backslash_rather_than_splitting_on_it() {
        assert_eq!(
            file_iri_from_parts("", r"/home/a\b.ttl"),
            "file:///home/a%5Cb.ttl"
        );
    }

    /// Every byte a `file://` IRI cannot carry literally is percent-encoded, in both halves,
    /// and the authority keeps LESS than the path (no `:`, `@` or `/`).
    #[test]
    fn each_component_encodes_under_its_own_rule() {
        assert_eq!(
            percent_encode("a b#c?d%e\u{e9}/f:g@h", b":@/"),
            "a%20b%23c%3Fd%25e%C3%A9/f:g@h"
        );
        assert_eq!(percent_encode("a/b:c", b""), "a%2Fb%3Ac");
    }

    // ── Filesystem derivation ──────────────────────────────────────────────────

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

    /// The canonicalizing derivation and the pure one agree on a real file: the former is
    /// the latter applied to a canonicalized path, not a second transformation.
    #[test]
    fn the_two_entry_points_agree_on_a_canonicalized_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.ttl");
        std::fs::write(&path, b"# empty\n").expect("write");

        let canonical = std::fs::canonicalize(&path).expect("canonicalize");
        assert_eq!(
            retrieval_base_iri(&path).expect("retrieval IRI").as_str(),
            file_iri_for_absolute_path(canonical.to_str().expect("UTF-8 temp path"))
        );
    }
}
