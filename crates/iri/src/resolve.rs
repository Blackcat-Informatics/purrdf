// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
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

    /// §5.3 component recomposition.
    fn recompose(&self) -> String {
        let mut out = String::new();
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
    pub fn resolve(&self, reference: &str) -> Result<Self> {
        if !self.has_scheme() {
            return Err(IriError::NonAbsoluteBase(self.as_str().to_owned()));
        }
        let base = Parts::of(self);
        // An EMPTY reference is the valid "same-document reference" (RFC-3986
        // §4.4 / §5.4.1 `"" = base`) — it is not a standalone IRI, so `parse`
        // (rightly) rejects it, but resolution must accept it as all-undefined.
        let r = if reference.is_empty() {
            Parts {
                scheme: None,
                authority: None,
                path: String::new(),
                query: None,
                fragment: None,
            }
        } else {
            Parts::of(&parse(reference)?)
        };

        let t = transform(&base, &r);
        // Recompose and re-parse so the returned Iri carries correct spans and is
        // itself validated (a resolution that produced something malformed is a
        // hard error, never a silently-returned bad IRI).
        parse(&t.recompose())
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
/// working `input` buffer (owned, because cases B/C rewrite its prefix) is drained
/// segment-by-segment into `out`.
pub(crate) fn remove_dot_segments(path: &str) -> String {
    let mut input = path.to_owned();
    let mut out = String::with_capacity(path.len());
    while !input.is_empty() {
        // A: leading "../" or "./" -> drop the prefix.
        if let Some(rest) = input.strip_prefix("../") {
            input = rest.to_owned();
        } else if let Some(rest) = input.strip_prefix("./") {
            input = rest.to_owned();
        }
        // B: "/./" -> "/"; exact "/." -> "/".
        else if let Some(rest) = input.strip_prefix("/./") {
            input = format!("/{rest}");
        } else if input == "/." {
            "/".clone_into(&mut input);
        }
        // C: "/../" -> "/" and pop last output segment; exact "/.." likewise.
        else if let Some(rest) = input.strip_prefix("/../") {
            pop_last_segment(&mut out);
            input = format!("/{rest}");
        } else if input == "/.." {
            pop_last_segment(&mut out);
            "/".clone_into(&mut input);
        }
        // D: input is exactly "." or ".." -> drop.
        else if input == "." || input == ".." {
            input.clear();
        }
        // E: move the first path segment (incl. any leading '/') to output.
        else {
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

/// Pop the trailing segment (and its preceding '/') from the output buffer — the
/// §5.2.4 case-C operation.
fn pop_last_segment(out: &mut String) {
    if let Some(slash) = out.rfind('/') {
        out.truncate(slash);
    } else {
        out.clear();
    }
}
