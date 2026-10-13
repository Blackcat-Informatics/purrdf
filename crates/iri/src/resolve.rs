// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RFC-3986 §5 reference resolution (strict mode) over [`Iri`] components.
//!
//! Implements the §5.2.2 "Transform References" algorithm, §5.2.3 "Merge Paths",
//! §5.2.4 "Remove Dot Segments", and §5.3 recomposition. The base must be
//! absolute (§5.2.1) — a relative base is a hard [`IriError::NonAbsoluteBase`].

use crate::error::{IriError, Result};
use crate::parse::Iri;
use crate::parse::{IriReadError, parse_owned_with_memory, parse_with_memory};
use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};
use std::borrow::Cow;

/// Owned component view used by the resolution algorithm. `None` = "undefined" in
/// RFC terms (distinct from an empty string, which is "defined but empty").
struct Parts<'a> {
    scheme: Option<&'a str>,
    authority: Option<&'a str>,
    path: Cow<'a, str>,
    query: Option<&'a str>,
    fragment: Option<&'a str>,
}

impl<'a> Parts<'a> {
    fn of(iri: &'a Iri) -> Self {
        Self {
            scheme: iri.scheme(),
            authority: iri.authority(),
            path: Cow::Borrowed(iri.path()),
            query: iri.query(),
            fragment: iri.fragment(),
        }
    }

    /// The all-undefined reference: RFC-3986 §4.4's same-document reference `""`,
    /// which `parse` cannot produce because it is not a standalone IRI.
    fn same_document() -> Self {
        Self {
            scheme: None,
            authority: None,
            path: Cow::Borrowed(""),
            query: None,
            fragment: None,
        }
    }

    /// §5.3 component recomposition.
    fn recompose<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> core::result::Result<String, StorageError> {
        // Exact-fit buffer from the known part lengths: one allocation per
        // recompose instead of the amortized-doubling reallocs of `String::new()`.
        let mut capacity = self.path.len();
        for (part, delimiter) in [
            (self.scheme, 1),
            (self.authority, 2),
            (self.query, 1),
            (self.fragment, 1),
        ] {
            if let Some(part) = part {
                capacity = capacity
                    .checked_add(part.len())
                    .and_then(|bytes| bytes.checked_add(delimiter))
                    .ok_or(StorageError::SizeOverflow)?;
            }
        }
        let mut out = String::new();
        memory.reserve_string(&mut out, capacity)?;
        if let Some(s) = &self.scheme {
            out.push_str(s);
            out.push(':');
        }
        if let Some(a) = &self.authority {
            out.push_str("//");
            out.push_str(a);
        }
        out.push_str(&self.path);
        if let Some(q) = &self.query {
            out.push('?');
            out.push_str(q);
        }
        if let Some(f) = &self.fragment {
            out.push('#');
            out.push_str(f);
        }
        Ok(out)
    }
}

impl Iri {
    /// Resolve `reference` against `self` as base, returning a new absolute
    /// [`Iri`] (RFC-3986 §5.2, strict). `self` must have a scheme.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let base = purrdf_iri::parse("http://example.org/a/b/c?q")?;
    ///
    /// // Relative path references merge and dot-normalize (RFC-3986 §5.4).
    /// assert_eq!(base.resolve("d")?.as_str(), "http://example.org/a/b/d");
    /// assert_eq!(base.resolve("../d")?.as_str(), "http://example.org/a/d");
    /// assert_eq!(base.resolve("/d")?.as_str(), "http://example.org/d");
    ///
    /// // The empty reference is the same-document reference (query kept).
    /// assert_eq!(base.resolve("")?.as_str(), "http://example.org/a/b/c?q");
    ///
    /// // An absolute reference replaces the base entirely.
    /// assert_eq!(
    ///     base.resolve("https://example.org/x")?.as_str(),
    ///     "https://example.org/x"
    /// );
    /// # Ok::<(), purrdf_iri::IriError>(())
    /// ```
    pub fn resolve(&self, reference: &str) -> Result<Self> {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        resident_result(self.resolve_with_memory(reference, &mut memory))
    }

    /// Resolve through the same RFC algorithm under original physical admission.
    ///
    /// Returned text and lexical errors remain charged to `memory` until their
    /// caller destroys them or transfers its original account into publication.
    ///
    /// # Errors
    /// Returns the original IRI error or typed physical storage refusal.
    pub fn resolve_with_memory<S: Admission + ?Sized>(
        &self,
        reference: &str,
        memory: &mut Memory<'_, S>,
    ) -> core::result::Result<Self, IriReadError> {
        if !self.has_scheme() {
            return Err(IriError::NonAbsoluteBase(memory.string(self.as_str())?).into());
        }
        // An EMPTY reference is the valid "same-document reference" (RFC-3986
        // §4.4 / §5.4.1 `"" = base`) — it is not a standalone IRI, so `parse`
        // (rightly) rejects it, but resolution must accept it as all-undefined.
        if reference.is_empty() {
            return self.transform_and_reparse(&Parts::same_document(), memory);
        }
        let parsed = parse_with_memory(reference, memory)?;
        let result = self.resolve_iri_with_memory(&parsed, memory);
        memory.release_string(parsed.text)?;
        result
    }

    /// [`resolve`](Self::resolve) for a reference the caller has **already parsed**.
    ///
    /// The string entry point is exactly this plus a parse, so the two cannot
    /// diverge. It exists because [`BaseScope`](crate::BaseScope) must inspect a
    /// reference's scheme before deciding whether the grammar resolves it at all,
    /// and re-parsing it afterwards would double the parse cost of every relative
    /// IRI in a Turtle document.
    pub(crate) fn resolve_iri(&self, r: &Self) -> Result<Self> {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        resident_result(self.resolve_iri_with_memory(r, &mut memory))
    }

    pub(crate) fn resolve_iri_with_memory<S: Admission + ?Sized>(
        &self,
        r: &Self,
        memory: &mut Memory<'_, S>,
    ) -> core::result::Result<Self, IriReadError> {
        if !self.has_scheme() {
            return Err(IriError::NonAbsoluteBase(memory.string(self.as_str())?).into());
        }
        self.transform_and_reparse(&Parts::of(r), memory)
    }

    /// §5.2.2 transform + §5.3 recomposition, then re-parse.
    ///
    /// Recomposing and re-parsing is what makes the returned `Iri` carry correct
    /// spans and be itself validated: a resolution that produced something malformed
    /// is a hard error, never a silently-returned bad IRI.
    fn transform_and_reparse<S: Admission + ?Sized>(
        &self,
        r: &Parts<'_>,
        memory: &mut Memory<'_, S>,
    ) -> core::result::Result<Self, IriReadError> {
        let transformed = transform(&Parts::of(self), r, memory)?;
        let text = transformed.recompose(memory);
        if let Cow::Owned(path) = transformed.path {
            memory.release_string(path)?;
        }
        parse_owned_with_memory(text?, memory)
    }
}

pub(crate) fn resident_result<T>(result: core::result::Result<T, IriReadError>) -> Result<T> {
    match result {
        Ok(value) => Ok(value),
        Err(IriReadError::Lexical(error)) => Err(error),
        Err(IriReadError::Storage(error)) => panic!("resident IRI storage failed: {error}"),
    }
}

/// RFC-3986 §5.2.2 transform-references (strict mode: a reference scheme is never
/// ignored, even when equal to the base scheme).
fn transform<'a, S: Admission + ?Sized>(
    base: &Parts<'a>,
    r: &Parts<'a>,
    memory: &mut Memory<'_, S>,
) -> core::result::Result<Parts<'a>, StorageError> {
    if r.scheme.is_some() {
        return Ok(Parts {
            scheme: r.scheme,
            authority: r.authority,
            path: Cow::Owned(remove_dot_segments_with_memory(&r.path, memory)?),
            query: r.query,
            fragment: r.fragment,
        });
    }
    if r.authority.is_some() {
        return Ok(Parts {
            scheme: base.scheme,
            authority: r.authority,
            path: Cow::Owned(remove_dot_segments_with_memory(&r.path, memory)?),
            query: r.query,
            fragment: r.fragment,
        });
    }
    let (path, query) = if r.path.is_empty() {
        let q = if r.query.is_some() {
            r.query
        } else {
            base.query
        };
        let path = match &base.path {
            Cow::Borrowed(path) => Cow::Borrowed(*path),
            Cow::Owned(path) => Cow::Owned(memory.string(path)?),
        };
        (path, q)
    } else if r.path.starts_with('/') {
        (
            Cow::Owned(remove_dot_segments_with_memory(&r.path, memory)?),
            r.query,
        )
    } else {
        let merged = merge(base, &r.path, memory)?;
        let path = remove_dot_segments_with_memory(&merged, memory);
        memory.release_string(merged)?;
        (Cow::Owned(path?), r.query)
    };
    Ok(Parts {
        scheme: base.scheme,
        authority: base.authority,
        path,
        query,
        fragment: r.fragment,
    })
}

/// RFC-3986 §5.2.3 merge: combine a relative-reference path with the base path.
fn merge<S: Admission + ?Sized>(
    base: &Parts<'_>,
    ref_path: &str,
    memory: &mut Memory<'_, S>,
) -> core::result::Result<String, StorageError> {
    if base.authority.is_some() && base.path.is_empty() {
        let mut s = String::new();
        memory.reserve_string(
            &mut s,
            ref_path
                .len()
                .checked_add(1)
                .ok_or(StorageError::SizeOverflow)?,
        )?;
        s.push('/');
        s.push_str(ref_path);
        Ok(s)
    } else {
        match base.path.rfind('/') {
            Some(slash) => {
                let mut s = String::new();
                memory.reserve_string(
                    &mut s,
                    (slash + 1)
                        .checked_add(ref_path.len())
                        .ok_or(StorageError::SizeOverflow)?,
                )?;
                s.push_str(&base.path[..=slash]);
                s.push_str(ref_path);
                Ok(s)
            }
            None => memory.string(ref_path),
        }
    }
}

/// RFC-3986 §5.2.4 remove-dot-segments. The canonical iterative algorithm: a
/// working `input` cursor is drained segment-by-segment into `out`.
///
/// Borrowed cursor: every case-B/C rewrite (`"/./"++rest -> "/"++rest`,
/// `"/../"++rest -> "/"++rest`) is a suffix of the input that already begins with
/// `/`, so each transition is a slice — zero allocations per resolve where the
/// owned-buffer form paid O(segments) `String` reallocs/`drain`s.
pub(crate) fn remove_dot_segments(path: &str) -> String {
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    remove_dot_segments_with_memory(path, &mut memory).expect("resident path storage")
}

fn remove_dot_segments_with_memory<S: Admission + ?Sized>(
    path: &str,
    memory: &mut Memory<'_, S>,
) -> core::result::Result<String, StorageError> {
    let mut input: &str = path;
    let mut out = String::new();
    memory.reserve_string(&mut out, path.len())?;
    while !input.is_empty() {
        // A: leading "../" or "./" -> drop the prefix.
        if let Some(rest) = input.strip_prefix("../") {
            input = rest;
        } else if let Some(rest) = input.strip_prefix("./") {
            input = rest;
        }
        // B: "/./" -> "/"; exact "/." -> "/". `"/" ++ rest` is `&input[2..]`.
        else if input.starts_with("/./") {
            input = &input[2..];
        } else if input == "/." {
            input = "/";
        }
        // C: "/../" -> "/" and pop last output segment; exact "/.." likewise.
        // `"/" ++ rest` is `&input[3..]`.
        else if input.starts_with("/../") {
            pop_last_segment(&mut out);
            input = &input[3..];
        } else if input == "/.." {
            pop_last_segment(&mut out);
            input = "/";
        }
        // D: input is exactly "." or ".." -> drop.
        else if input == "." || input == ".." {
            input = "";
        }
        // E: move the first path segment (incl. any leading '/') to output.
        else {
            let start = usize::from(input.starts_with('/'));
            let seg_end = match input[start..].find('/') {
                Some(i) => start + i,
                None => input.len(),
            };
            out.push_str(&input[..seg_end]);
            input = &input[seg_end..];
        }
    }
    Ok(out)
}

/// Pop the trailing segment (and its preceding '/') from the output buffer — the
/// §5.2.4 case-C operation.
fn pop_last_segment(out: &mut String) {
    if let Some(slash) = out.rfind('/') {
        out.truncate(slash);
    } else {
        out.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::remove_dot_segments;
    use crate::parse::parse;

    /// The pre-cursor owned-buffer form of §5.2.4, kept verbatim as the oracle
    /// the borrowed-slice rewrite must match byte-for-byte.
    fn remove_dot_segments_owned_reference(path: &str) -> String {
        let mut input = path.to_owned();
        let mut out = String::with_capacity(path.len());
        while !input.is_empty() {
            if let Some(rest) = input.strip_prefix("../") {
                input = rest.to_owned();
            } else if let Some(rest) = input.strip_prefix("./") {
                input = rest.to_owned();
            } else if let Some(rest) = input.strip_prefix("/./") {
                input = format!("/{rest}");
            } else if input == "/." {
                "/".clone_into(&mut input);
            } else if let Some(rest) = input.strip_prefix("/../") {
                super::pop_last_segment(&mut out);
                input = format!("/{rest}");
            } else if input == "/.." {
                super::pop_last_segment(&mut out);
                "/".clone_into(&mut input);
            } else if input == "." || input == ".." {
                input.clear();
            } else {
                let start = usize::from(input.starts_with('/'));
                let seg_end = match input[start..].find('/') {
                    Some(i) => start + i,
                    None => input.len(),
                };
                out.push_str(&input[..seg_end]);
                input.drain(..seg_end);
            }
        }
        out
    }

    #[test]
    fn cursor_remove_dot_segments_matches_owned_reference() {
        for path in [
            "",
            "/",
            ".",
            "..",
            "/.",
            "/..",
            "./",
            "../",
            "/./",
            "/../",
            "a",
            "/a",
            "a/",
            "/a/",
            "/a/b/c/./../../g",
            "mid/content=5/../6",
            "/b/c/../../../g",
            "../../a/./b/../c",
            "/a/./b/./c/.",
            "/a/../../b/..",
            "a/b/c/..",
            "./a/../b/./c/../d",
            "/./a/./b/./",
            "/../a",
            "..a/b..",
            "/.a/..b/.../a..",
            "/a//b/../c",
            "//a/../b",
            "/ü/../ö/./ä",
        ] {
            assert_eq!(
                remove_dot_segments(path),
                remove_dot_segments_owned_reference(path),
                "path = {path:?}"
            );
        }
    }

    /// [`Iri::resolve_iri`] must be [`Iri::resolve`] minus the parse, or the base
    /// layer that calls it would be a second resolver free to drift.
    #[test]
    fn resolve_iri_matches_the_string_entry_point() {
        let base = parse("http://a/b/c/d;p?q").expect("base parses");
        for reference in [
            "g",
            "./g",
            "../g",
            "/g",
            "//g",
            "?y",
            "#s",
            "g;x?y#s",
            "g:h",
            "http:g",
            "http://a/b/../c",
            "g/../h",
        ] {
            let parsed = parse(reference).expect("reference parses");
            assert_eq!(
                base.resolve_iri(&parsed).map(|iri| iri.as_str().to_owned()),
                base.resolve(reference).map(|iri| iri.as_str().to_owned()),
                "ref = {reference:?}"
            );
        }
    }
}
