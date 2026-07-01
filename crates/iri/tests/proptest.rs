// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Property tests: parse is verbatim-faithful, normalization is idempotent, and
//! resolving a reference against an absolute base yields an absolute IRI.

use proptest::prelude::*;
use purrdf_iri::parse;

/// A strategy generating syntactically valid http(s) IRIs from constrained parts,
/// so we exercise the parser/normalizer over a broad-but-legal input space.
fn valid_iri() -> impl Strategy<Value = String> {
    let scheme = prop::sample::select(vec!["http", "https", "ftp"]);
    let host = "[a-z][a-z0-9]{0,8}(\\.[a-z][a-z0-9]{0,8}){0,3}";
    let segs = prop::collection::vec("[a-z0-9._~-]{1,6}", 0..4);
    let frag = prop::option::of("[a-z0-9]{0,6}");
    (scheme, host, segs, frag).prop_map(|(s, h, segs, frag)| {
        let mut out = format!("{s}://{h}");
        for seg in segs {
            out.push('/');
            out.push_str(&seg);
        }
        if let Some(f) = frag {
            out.push('#');
            out.push_str(&f);
        }
        out
    })
}

proptest! {
    /// The parser never rewrites its input: `as_str()` is byte-identical.
    #[test]
    fn parse_is_verbatim(s in valid_iri()) {
        let iri = parse(&s).expect("generated IRI must parse");
        prop_assert_eq!(iri.as_str(), s.as_str());
    }

    /// Normalization is idempotent.
    #[test]
    fn normalize_idempotent(s in valid_iri()) {
        let n1 = parse(&s).unwrap().normalize();
        let n2 = n1.normalize();
        prop_assert_eq!(n1.as_str(), n2.as_str());
    }

    /// Resolving any relative path-segment against an absolute base stays absolute.
    #[test]
    fn resolution_preserves_absoluteness(s in valid_iri(), rel in "[a-z0-9]{1,5}(/[a-z0-9]{1,5}){0,3}") {
        let base = parse(&s).unwrap();
        // The generated base always has a scheme, so resolution must succeed.
        let resolved = base.resolve(&rel).expect("absolute base resolves");
        prop_assert!(resolved.has_scheme());
    }
}
