// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RFC-3986 §5 reference resolution (strict mode) over [`Iri`] components.
//!
//! Implements the §5.2.2 "Transform References" algorithm, §5.2.3 "Merge Paths",
//! §5.2.4 "Remove Dot Segments", and §5.3 recomposition. The base must be
//! absolute (§5.2.1) — a relative base is a hard [`IriError::NonAbsoluteBase`].

use crate::error::{IriError, Result};
use crate::parse::{Iri, parse};

/// Owned component view used by the resolution algorithm. `None` = "undefined" in
/// RFC terms (distinct from an empty string, which is "defined but empty").
struct Parts {
    scheme: Option<String>,
    authority: Option<String>,
    path: String,
    query: Option<String>,
    fragment: Option<String>,
}

impl Parts {
    fn of(iri: &Iri) -> Self {
        Self {
            scheme: iri.scheme().map(str::to_owned),
            authority: iri.authority().map(str::to_owned),
            path: iri.path().to_owned(),
            query: iri.query().map(str::to_owned),
            fragment: iri.fragment().map(str::to_owned),
        }
    }

    /// The all-undefined reference: RFC-3986 §4.4's same-document reference `""`,
    /// which `parse` cannot produce because it is not a standalone IRI.
    fn same_document() -> Self {
        Self {
            scheme: None,
            authority: None,
            path: String::new(),
            query: None,
            fragment: None,
        }
    }

    /// §5.3 component recomposition.
    fn recompose(&self) -> String {
        // Exact-fit buffer from the known part lengths: one allocation per
        // recompose instead of the amortized-doubling reallocs of `String::new()`.
        let capacity = self.scheme.as_ref().map_or(0, |s| s.len() + 1)
            + self.authority.as_ref().map_or(0, |a| a.len() + 2)
            + self.path.len()
            + self.query.as_ref().map_or(0, |q| q.len() + 1)
            + self.fragment.as_ref().map_or(0, |f| f.len() + 1);
        let mut out = String::with_capacity(capacity);
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
        out
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
        if !self.has_scheme() {
            return Err(IriError::NonAbsoluteBase(self.as_str().to_owned()));
        }
        // An EMPTY reference is the valid "same-document reference" (RFC-3986
        // §4.4 / §5.4.1 `"" = base`) — it is not a standalone IRI, so `parse`
        // (rightly) rejects it, but resolution must accept it as all-undefined.
        if reference.is_empty() {
            return self.transform_and_reparse(&Parts::same_document());
        }
        self.resolve_iri(&parse(reference)?)
    }

    /// [`resolve`](Self::resolve) for a reference the caller has **already parsed**.
    ///
    /// The string entry point is exactly this plus a parse, so the two cannot
    /// diverge. It exists because [`BaseScope`](crate::BaseScope) must inspect a
    /// reference's scheme before deciding whether the grammar resolves it at all,
    /// and re-parsing it afterwards would double the parse cost of every relative
    /// IRI in a Turtle document.
    pub(crate) fn resolve_iri(&self, r: &Self) -> Result<Self> {
        if !self.has_scheme() {
            return Err(IriError::NonAbsoluteBase(self.as_str().to_owned()));
        }
        self.transform_and_reparse(&Parts::of(r))
    }

    /// §5.2.2 transform + §5.3 recomposition, then re-parse.
    ///
    /// Recomposing and re-parsing is what makes the returned `Iri` carry correct
    /// spans and be itself validated: a resolution that produced something malformed
    /// is a hard error, never a silently-returned bad IRI.
    fn transform_and_reparse(&self, r: &Parts) -> Result<Self> {
        parse(&transform(&Parts::of(self), r).recompose())
    }
}

/// RFC-3986 §5.2.2 transform-references (strict mode: a reference scheme is never
/// ignored, even when equal to the base scheme).
fn transform(base: &Parts, r: &Parts) -> Parts {
    if r.scheme.is_some() {
        return Parts {
            scheme: r.scheme.clone(),
            authority: r.authority.clone(),
            path: remove_dot_segments(&r.path),
            query: r.query.clone(),
            fragment: r.fragment.clone(),
        };
    }
    if r.authority.is_some() {
        return Parts {
            scheme: base.scheme.clone(),
            authority: r.authority.clone(),
            path: remove_dot_segments(&r.path),
            query: r.query.clone(),
            fragment: r.fragment.clone(),
        };
    }
    let (path, query) = if r.path.is_empty() {
        let q = if r.query.is_some() {
            r.query.clone()
        } else {
            base.query.clone()
        };
        (base.path.clone(), q)
    } else if r.path.starts_with('/') {
        (remove_dot_segments(&r.path), r.query.clone())
    } else {
        let merged = merge(base, &r.path);
        (remove_dot_segments(&merged), r.query.clone())
    };
    Parts {
        scheme: base.scheme.clone(),
        authority: base.authority.clone(),
        path,
        query,
        fragment: r.fragment.clone(),
    }
}

/// RFC-3986 §5.2.3 merge: combine a relative-reference path with the base path.
fn merge(base: &Parts, ref_path: &str) -> String {
    if base.authority.is_some() && base.path.is_empty() {
        let mut s = String::with_capacity(ref_path.len() + 1);
        s.push('/');
        s.push_str(ref_path);
        s
    } else {
        match base.path.rfind('/') {
            Some(slash) => {
                let mut s = base.path[..=slash].to_owned();
                s.push_str(ref_path);
                s
            }
            None => ref_path.to_owned(),
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
    let mut input: &str = path;
    let mut out = String::with_capacity(path.len());
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
    out
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
