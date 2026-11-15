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
//! For a relative reference `r` and an absolute base `b`, `b.resolve(r)` and
//! `b.resolve(r.normalize())` name the same IRI.
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
    /// A relative reference keeps its meaning: resolving the result against an
    /// absolute base gives the IRI the original resolves to, so a scheme-less
    /// reference with no authority keeps the leading dot segments only a base
    /// can interpret.
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
        let decoded = percent::normalize(self.path());
        // A rootless path under no scheme and no authority has not been resolved:
        // its leading dot segments belong to the base it will meet.
        let unresolved = self.scheme().is_none()
            && self.authority().is_none()
            && !decoded.is_empty()
            && !decoded.starts_with('/');
        let path = if unresolved {
            collapse_unresolved_dot_segments(&decoded)
        } else {
            remove_dot_segments(&decoded)
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
