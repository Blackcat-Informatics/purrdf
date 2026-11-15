// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RFC-3986 §6.2.2 syntax-based normalization.
//!
//! Three sub-steps, all syntax-based (no scheme-specific knowledge — §6.2.3
//! scheme-based normalization is deliberately out of scope and would be a separate,
//! per-scheme concern; we do not half-implement it):
//!
//! * §6.2.2.1 **Case** — scheme + host lower-cased; percent-encoding hex digits
//!   upper-cased.
//! * §6.2.2.2 **Percent-encoding** — `%XX` triplets that encode an *unreserved*
//!   character are decoded to that character.
//! * §6.2.2.3 **Path segment** — `remove_dot_segments` applied to a path that
//!   resolution would treat the same way: any path under a scheme or an
//!   authority, and an absolute path. The rootless path of a reference with
//!   neither is not resolved yet, so only the dot segments that mean the same
//!   against every base collapse (`a/./b`, `a/b/../c`); `../a`, `..` and `.`
//!   keep the segments a base must interpret.
//!
//! The steps interact in one place. §6.2.2.2 decodes `%2E`, and a segment that
//! decodes to `.` or `..` then reads as a dot segment, which resolution of the
//! undecoded text does not see. In the unresolved case such a segment therefore
//! keeps its encoding, as `%2E` or `%2E%2E`, and only a literal `.` or `..` is a
//! dot segment. Everywhere else it is decoded first and removed as a dot
//! segment.
//!
//! **Guarantee and its scope.** For a reference `r` with no scheme, no authority
//! and a path that does not begin with `/`, and any absolute base `b`,
//! `b.resolve(r)` and `b.resolve(r.normalize())` name the same IRI: they are
//! equal once both are normalized, and byte-equal when `r` holds no
//! percent-encoded unreserved character and no lower-case hex digit. The
//! guarantee does not extend to a path under a scheme, an authority or a
//! leading `/` that holds a percent-encoded dot segment: `/a/%2e%2e/../b`
//! normalizes to `/b` and resolves to `/a/b`.
//!
//! Normalization is idempotent: `n.normalize() == n.normalize().normalize()`.

use purrdf_lex::percent;

use crate::parse::{Iri, parse};
use crate::resolve::{collapse_unresolved_dot_segments, recomposition_guard, remove_dot_segments};

impl Iri {
    /// Produce a syntax-normalized copy (RFC-3986 §6.2.2). The result is itself
    /// parsed/validated; for every IRI `parse` accepts, the result has the same
    /// scheme and authority presence and is a fixed point of `normalize`.
    ///
    /// A reference with no scheme, no authority and a path that does not begin
    /// with `/` keeps its meaning: resolving the result against an absolute base
    /// names the IRI the original resolves to. It therefore keeps the leading
    /// dot segments only a base can interpret, and a segment that would decode
    /// to `.` or `..` stays percent-encoded (`%2E`, `%2E%2E`). Under a scheme,
    /// an authority or a leading `/`, a percent-encoded dot segment is decoded
    /// and removed, which resolution of the undecoded text would not do.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Case (§6.2.2.1), percent-encoding (§6.2.2.2), dot segments (§6.2.2.3).
    /// let iri = purrdf_iri::parse("HTTP://EXAMPLE.org/a/./b/../c/%7Ename")?;
    /// let norm = iri.normalize();
    /// assert_eq!(norm.as_str(), "http://example.org/a/c/~name");
    ///
    /// // Normalization is idempotent.
    /// assert_eq!(norm.normalize(), norm);
    ///
    /// // Interior dot segments of a relative reference collapse; leading ones stay.
    /// assert_eq!(purrdf_iri::parse("a/./b/../c")?.normalize().as_str(), "a/c");
    /// assert_eq!(purrdf_iri::parse("../a")?.normalize().as_str(), "../a");
    /// // There, an encoded dot segment is an ordinary segment and stays encoded.
    /// assert_eq!(purrdf_iri::parse("a/%2e/../b")?.normalize().as_str(), "a/b");
    /// # Ok::<(), purrdf_iri::IriError>(())
    /// ```
    #[must_use]
    pub fn normalize(&self) -> Self {
        let mut out = String::with_capacity(self.as_str().len());

        if let Some(scheme) = self.scheme() {
            out.push_str(&scheme.to_ascii_lowercase());
            out.push(':');
        }
        if let Some(auth) = self.authority() {
            out.push_str("//");
            out.push_str(&normalize_authority(auth));
        }
        let raw = self.path();
        // A rootless path under no scheme and no authority has not been resolved:
        // its leading dot segments belong to the base it will meet.
        let unresolved = self.scheme().is_none()
            && self.authority().is_none()
            && !raw.is_empty()
            && !raw.starts_with('/');
        let path = if unresolved {
            collapse_unresolved_dot_segments(&decode_unresolved_segments(raw))
        } else {
            remove_dot_segments(&percent::normalize(raw))
        };
        out.push_str(recomposition_guard(
            self.scheme().is_some(),
            self.authority().is_some(),
            &path,
        ));
        out.push_str(&path);
        if let Some(q) = self.query() {
            out.push('?');
            out.push_str(&percent::normalize(q));
        }
        if let Some(frag) = self.fragment() {
            out.push('#');
            out.push_str(&percent::normalize(frag));
        }

        // Re-parse. Every step keeps the text a valid IRI of the same shape: ASCII
        // lower-casing, decoding unreserved characters, dot-segment removal and the
        // `recomposition_guard` prefixes. The text is never empty: a scheme, an
        // authority, a rooted path and an unresolved path each leave at least one
        // character, and an empty path parsed only with a query or fragment. A
        // parse failure here is therefore a genuine bug, not user input - hence
        // `expect`.
        parse(&out).expect("normalized IRI must re-parse")
    }
}

/// §6.2.2.2 for an unresolved rootless path, segment by segment (`/` is reserved,
/// so decoding never moves a segment boundary). Resolution reads a segment such
/// as `%2e` or `.%2E` as an ordinary one, so a segment that decodes to `.` or `..`
/// without being spelled that way is written fully encoded instead: decoding it
/// would turn it into a dot segment and change what the reference resolves to.
/// `%2E` and `%2E%2E` are fixed points of this step.
fn decode_unresolved_segments(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for (index, segment) in path.split('/').enumerate() {
        if index > 0 {
            out.push('/');
        }
        let decoded = percent::normalize(segment);
        match decoded.as_ref() {
            "." if segment != "." => out.push_str("%2E"),
            ".." if segment != ".." => out.push_str("%2E%2E"),
            other => out.push_str(other),
        }
    }
    out
}

/// Case-normalize the host (lower-case) without disturbing a case-significant
/// userinfo, then percent-normalize. Userinfo (before `@`) keeps its case; the
/// host + port (after `@`) is lower-cased for the host portion.
fn normalize_authority(auth: &str) -> String {
    let (userinfo, host_port) = match auth.find('@') {
        Some(at) => (Some(&auth[..at]), &auth[at + 1..]),
        None => (None, auth),
    };
    let mut out = String::with_capacity(auth.len());
    if let Some(ui) = userinfo {
        out.push_str(&percent::normalize(ui));
        out.push('@');
    }
    // Lower-case the host (and the port, which is digits-only so case is moot).
    out.push_str(&percent::normalize(&host_port.to_ascii_lowercase()));
    out
}
