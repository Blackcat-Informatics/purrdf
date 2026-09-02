// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The crate's determinism claim, reduced to one number that can be computed on
//! two targets and compared.
//!
//! # Why this module exists
//!
//! This crate claims that a native answer and a `wasm32-unknown-unknown` answer
//! are **bit-identical**. That claim has an argument behind it — every
//! computation is integer arithmetic, Rust specifies integer arithmetic
//! completely and identically on every target, and the crate root denies
//! `clippy::float_arithmetic` so there is no second path — but an argument is not
//! evidence. A reader cannot check an argument by running it, and the failure
//! mode this crate exists to prevent is precisely the one that produces no
//! symptom.
//!
//! So the claim is made checkable. [`digest`] runs a fixed corpus through every
//! consumer-visible output path this crate has — the WKT writer, the GeoJSON
//! writer, the DE-9IM matrix, the measures, and the `xsd:double` rendering that
//! is the crate's single floating-point boundary — and folds the resulting
//! **bytes** into one `u64`. `crates/geo/tests/determinism.rs` asserts that
//! number natively; `scripts/check-geo-determinism.sh` builds the same function
//! for `wasm32-unknown-unknown`, runs it under Node, and asserts the same number.
//! Two targets, one constant, no reasoning in between.
//!
//! # Why the digest is over bytes, and over these bytes
//!
//! Byte identity of the *serialized* answer is the only claim that covers the
//! coordinate lexical forms, the matrix renderings and the double renderings all
//! at once, and it is the artefact a downstream cache, diff or signature would
//! actually key on. A digest over internal values would pass while the renderer
//! that a consumer actually sees diverged.
//!
//! The corpus is small and hand-written rather than generated, because the point
//! is that a human can read it and see that it exercises the paths that could
//! diverge: a coordinate with more precision than a `double` can hold, a value
//! whose decimal expansion is infinite, an irrational length, a boundary case for
//! the topological predicates, and a rounding tie.
//!
//! # The hash is hand-rolled, deliberately
//!
//! FNV-1a, written out below in six lines. Not `ahash` (explicitly not
//! version-stable, so its output cannot address content), not
//! [`std::hash::DefaultHasher`] (SipHash with an unspecified, version-dependent
//! implementation). A digest that is compared across two builds must be a
//! function of the bytes and of nothing else, and the only way to be sure of that
//! is to be able to read the whole hash.

use crate::geom::{Crs, GeometryLiteral};
use crate::measure;
use crate::topology::relate;
use crate::{construct, geojson, wkt};

/// The coordinate reference system the corpus is expressed in.
///
/// `example.org`, like every other fixture in this crate: PurRDF mints no
/// vocabulary IRIs, and a corpus that named a real OGC system would be asserting
/// something about that system rather than about this crate's arithmetic.
const CORPUS_CRS: &str = "http://example.org/crs/planar";

/// The maximum fraction digits the corpus renders coordinates at.
///
/// Deliberately wide — far wider than a `double` can represent — so that a
/// coordinate whose exactness this crate preserves shows up in the digest. A
/// narrow scale would round the interesting cases away before they were hashed.
const CORPUS_SCALE: u32 = 40;

/// The corpus: WKT lexical forms chosen for the paths that could diverge between
/// two targets, each with a note on which one it is there for.
const CORPUS: &[&str] = &[
    // A plain point, the base case.
    "POINT(1 2)",
    // A coordinate with more significant digits than an f64 can hold. If the
    // ingest path ever passed through a float, this would round and the digest
    // would move.
    "POINT(0.12345678901234567890123456789012345678 -83.38632838293847562819)",
    // Three spellings of one number. The exact model makes them one geometry, so
    // they must contribute identical bytes.
    "POINT(1.5 0)",
    "POINT(1.50 0)",
    "POINT(15e-1 0)",
    // A value with no finite binary expansion.
    "POINT(0.1 0.2)",
    // A rounding tie at the corpus scale, exercising the half-to-even rule in the
    // decimal renderer.
    "POINT(0.5 2.5)",
    // A unit square: an exact area, an exact perimeter, and a ring whose winding
    // the orientation tests must agree on.
    "POLYGON((0 0,1 0,1 1,0 1,0 0))",
    // The same square wound the other way. Area is orientation-independent, so
    // this pins that too.
    "POLYGON((0 0,0 1,1 1,1 0,0 0))",
    // A square with a hole: subtraction of exact areas.
    "POLYGON((0 0,4 0,4 4,0 4,0 0),(1 1,2 1,2 2,1 2,1 1))",
    // An IRRATIONAL length: the unit diagonal is sqrt(2), computed by exact
    // integer square root at a fixed internal scale and summed as integers. This
    // is the single most important corpus member — it is the path where a
    // float-based implementation would diverge between targets.
    "LINESTRING(0 0,1 1)",
    // Several irrational segments summed. A float sum would depend on the order;
    // an integer sum does not, and this pins that it does not.
    "LINESTRING(0 0,1 1,3 2,4 6,-2 -3)",
    // A 3-4-5 triangle: a perfect square under the root, so the integer square
    // root is EXACT and the digest carries an exactly-representable irrational.
    "LINESTRING(0 0,3 4)",
    // Z and M ordinates, which the standard says are ignored in calculations but
    // are carried by the model and rendered by the writer.
    "POINT Z (1 2 3)",
    "POINT ZM (1 2 3 4)",
    // Empties of several kinds.
    "POINT EMPTY",
    "POLYGON EMPTY",
    "GEOMETRYCOLLECTION EMPTY",
    // A nested collection, so the traversal order reaches the digest.
    "GEOMETRYCOLLECTION(POINT(1 2),LINESTRING(0 0,1 1),POLYGON((0 0,1 0,1 1,0 0)))",
    // A multi-geometry, for the same reason.
    "MULTIPOLYGON(((0 0,1 0,1 1,0 0)),((5 5,6 5,6 6,5 5)))",
];

/// FNV-1a over a byte slice, folded into `state`.
///
/// Written out rather than imported so that the digest is a function of these six
/// lines and of nothing that could change under it. `wrapping_mul` because
/// FNV's multiply is defined modulo 2^64 — an overflow here is the algorithm, not
/// a bug, and it must not panic under the debug overflow checks the test profile
/// turns on.
const fn fold(mut state: u64, bytes: &[u8]) -> u64 {
    let mut index = 0;
    while index < bytes.len() {
        state ^= bytes[index] as u64;
        state = state.wrapping_mul(0x0000_0100_0000_01B3);
        index += 1;
    }
    state
}

/// The FNV-1a offset basis.
const BASIS: u64 = 0xCBF2_9CE4_8422_2325;

/// Run the corpus through every consumer-visible output path and fold the
/// resulting bytes into one number.
///
/// This value is a pure function of this crate's source. It contains no clock, no
/// address, no allocation order and no map iteration — and, crucially, no
/// floating-point arithmetic, which is what makes it equal on every target rather
/// than merely usually equal.
///
/// # Panics
///
/// Panics if any corpus member fails to parse or to re-serialize. The corpus is a
/// constant in this file, so that is a bug in this file rather than a runtime
/// condition, and a digest computed over a silently-skipped member would be a
/// number that agrees on two targets while proving nothing.
#[must_use]
pub fn digest() -> u64 {
    let crs = Crs::new(CORPUS_CRS).expect("the corpus CRS IRI is a non-empty constant");
    let mut state = BASIS;

    let parsed: Vec<GeometryLiteral> = CORPUS
        .iter()
        .map(|lexical| wkt::parse(lexical, &crs).expect("every corpus member is well-formed WKT"))
        .collect();

    // 1. The WKT writer: coordinate lexical forms, keyword spelling, separators.
    for literal in &parsed {
        state = fold(state, wkt::write(literal, CORPUS_SCALE).as_bytes());
    }

    // 2. The GeoJSON writer, for the members it can represent. A member carrying
    //    an M ordinate is refused rather than silently flattened, so the REFUSAL
    //    is folded in too — a build that started accepting it would move the
    //    digest, which is the point.
    for literal in &parsed {
        match geojson::write(literal, &crs, CORPUS_SCALE) {
            Ok(json) => state = fold(state, json.as_bytes()),
            Err(error) => state = fold(state, error.to_string().as_bytes()),
        }
    }

    // 3. The DE-9IM matrix of every ORDERED pair. Quadratic in the corpus, and
    //    that is deliberate: the matrix is where the orientation tests, the
    //    segment intersections and the scan line all show up at once, and the
    //    ordered pairs catch a transposition that the symmetric ones would hide.
    for a in &parsed {
        for b in &parsed {
            let matrix = relate(a.geometry(), b.geometry());
            state = fold(state, matrix.to_string().as_bytes());
        }
    }

    // 4. The measures, rendered as exact decimals. `length` and `distance` are
    //    the irrational ones — the paths a floating-point implementation would
    //    make target-dependent.
    for literal in &parsed {
        let geometry = literal.geometry();
        state = fold(
            state,
            measure::area(geometry)
                .to_decimal_string(CORPUS_SCALE)
                .as_bytes(),
        );
        state = fold(
            state,
            measure::length(geometry)
                .to_decimal_string(CORPUS_SCALE)
                .as_bytes(),
        );
        state = fold(
            state,
            measure::perimeter(geometry)
                .to_decimal_string(CORPUS_SCALE)
                .as_bytes(),
        );
    }
    for a in &parsed {
        for b in &parsed {
            if let Some(distance) = measure::distance(a.geometry(), b.geometry()) {
                state = fold(state, distance.to_decimal_string(CORPUS_SCALE).as_bytes());
            } else {
                state = fold(state, b"none");
            }
        }
    }

    // 5. The constructors, re-serialized. `centroid` is a ratio of exact
    //    rationals and `convex_hull` is decided entirely by orientation signs, so
    //    both are pure integer paths whose OUTPUT is a rendered coordinate.
    for literal in &parsed {
        let geometry = literal.geometry();
        state = fold(
            state,
            wkt::write_bare(&construct::envelope(geometry), CORPUS_SCALE).as_bytes(),
        );
        state = fold(
            state,
            wkt::write_bare(&construct::boundary(geometry), CORPUS_SCALE).as_bytes(),
        );
        state = fold(
            state,
            wkt::write_bare(&construct::convex_hull(geometry), CORPUS_SCALE).as_bytes(),
        );
        match construct::centroid(geometry) {
            Some(coord) => {
                state = fold(state, coord.x().to_decimal_string(CORPUS_SCALE).as_bytes());
                state = fold(state, coord.y().to_decimal_string(CORPUS_SCALE).as_bytes());
            }
            None => state = fold(state, b"none"),
        }
    }

    // 6. THE FLOAT BOUNDARY. Every numeric `geof:` result leaves this crate as an
    //    `xsd:double`, produced by `Rat::to_f64`, which computes the correctly
    //    rounded nearest double with integer arithmetic and assembles it with
    //    `f64::from_bits`. Folding the resulting BIT PATTERN is what makes this
    //    digest cover the one place a float appears at all. `to_bits` is a
    //    reinterpretation, not arithmetic, so it is exact on every target.
    for literal in &parsed {
        let geometry = literal.geometry();
        for value in [
            measure::area(geometry),
            measure::length(geometry),
            measure::perimeter(geometry),
        ] {
            state = fold(state, &value.to_f64().to_bits().to_be_bytes());
        }
    }

    state
}

/// The number of corpus members [`digest`] folds — the geometries, not the byte
/// sequences, of which there are far more (the DE-9IM and distance passes are
/// quadratic in this number).
///
/// Reported alongside the digest so that a harness can prove the digest is not
/// vacuous — a `digest()` that folded nothing would still be equal on two targets
/// and would still prove nothing at all — and so that two targets can be shown to
/// have folded the *same* corpus rather than merely agreeing on a number.
#[must_use]
pub fn corpus_len() -> usize {
    CORPUS.len()
}

#[cfg(test)]
mod tests {
    use super::{CORPUS, CORPUS_CRS, corpus_len, digest, fold};
    use crate::geom::Crs;
    use crate::wkt;

    /// The digest must be a pure function: the same call twice is the same value.
    #[test]
    fn the_digest_is_a_pure_function() {
        let first = digest();
        for run in 0..8 {
            assert_eq!(digest(), first, "run {run} diverged from the first");
        }
    }

    /// The digest must actually depend on the corpus — a hash that folded nothing
    /// would be equal on two targets and prove nothing.
    #[test]
    fn the_digest_is_not_vacuous() {
        assert!(corpus_len() >= 20, "the corpus must be worth hashing");
        assert_ne!(
            digest(),
            super::BASIS,
            "the digest must differ from the unfolded basis, or nothing was folded"
        );
        // And the fold itself must be sensitive to a single byte.
        assert_ne!(fold(super::BASIS, b"a"), fold(super::BASIS, b"b"));
        assert_ne!(fold(super::BASIS, b"ab"), fold(super::BASIS, b"ba"));
    }

    /// Every corpus member must parse, or the digest is computed over a silently
    /// smaller set than it claims.
    #[test]
    fn every_corpus_member_parses() {
        let crs = Crs::new(CORPUS_CRS).expect("a non-empty IRI");
        for lexical in CORPUS {
            assert!(
                wkt::parse(lexical, &crs).is_ok(),
                "corpus member {lexical:?} must parse"
            );
        }
    }

    /// The three spellings of `1.5` in the corpus are one geometry, which is the
    /// exactness property the digest is folding.
    #[test]
    fn the_corpus_carries_three_spellings_of_one_number() {
        let crs = Crs::new(CORPUS_CRS).expect("a non-empty IRI");
        let a = wkt::parse("POINT(1.5 0)", &crs).expect("parse");
        let b = wkt::parse("POINT(1.50 0)", &crs).expect("parse");
        let c = wkt::parse("POINT(15e-1 0)", &crs).expect("parse");
        assert_eq!(a, b, "1.5 and 1.50 denote one geometry");
        assert_eq!(a, c, "1.5 and 15e-1 denote one geometry");
        // The CONTROL: a nearby but different number must stay different, so the
        // equality above turns on the value rather than on everything agreeing.
        let d = wkt::parse("POINT(1.51 0)", &crs).expect("parse");
        assert_ne!(a, d);
    }
}
