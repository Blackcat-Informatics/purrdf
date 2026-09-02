// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The twenty-four topological relations of GeoSPARQL, as data.
//!
//! GeoSPARQL defines three relation families — Simple Features (`sf*`), Egenhofer
//! (`eh*`) and RCC8 (`rcc8*`) — and defines every member of every one of them as a
//! **pattern over the DE-9IM matrix** (OGC 22-047r1 Tables 2/3/4 for the
//! properties and Tables 6/7/8 for the functions). So this module is a table, not
//! an algorithm: [`crate::topology::relate`] computes one matrix and each relation
//! is a lookup against it.
//!
//! That is the whole reason the module exists as a table. Twenty-four
//! hand-written predicates would be twenty-four independent chances to answer
//! `false` for the wrong reason, and a `false` from a topological predicate is
//! indistinguishable from an honest one — there is no symptom, no error, and no
//! way for a caller to tell. One matrix and twenty-four patterns cannot drift.
//!
//! # Where this table knowingly departs from the published tables, and why
//!
//! Three departures. All are recorded here rather than in a commit message
//! because a reader comparing this file against the standard will otherwise read
//! them as bugs.
//!
//! **1. `sfIntersects` uses Table 2, not Table 6.** OGC 22-047r1 prints two
//! different patterns for `sfIntersects`: Table 2 (the property) gives the
//! four-row union `T********` / `*T*******` / `***T*****` / `****T****`, while
//! Table 6 (the function) gives `FT*******` / `F**T*****` / `F***T****` — which
//! is *character for character the pattern it also gives for `sfTouches`*. Table 6
//! is a published defect, and the standard refutes it from the inside: its own
//! Table 5 states the cross-family equivalence `intersects | ¬ disconnected |
//! ¬ disjoint`, and a relation equal to `sfTouches` is not the negation of
//! `sfDisjoint`. Implementing Table 6 would make `?a geof:sfIntersects ?b` answer
//! `false` for a point strictly inside a polygon. This table implements Table 2,
//! and [`intersects_is_the_negation_of_disjoint`](self) pins the equivalence
//! Table 5 asserts.
//!
//! **2. The type-dispatched relations are symmetric.** `sfCrosses` is
//! type-dispatched: the standard gives `T*T******` for the pairs
//! point/curve, point/area and curve/area, and `0********` for curve/curve. It
//! says nothing about the *reversed* pairs (area/curve, area/point, curve/point).
//! Reading that silence as "answer `false`" would mean `geof:sfCrosses(?line,
//! ?polygon)` is true while `geof:sfCrosses(?polygon, ?line)` is false for the
//! same crossing — a silent wrong answer produced by argument order alone. This
//! table answers the reversed pairs with the transposed pattern
//! (`T*****T**`, reading the `E/I` cell where the forward case reads `I/E`),
//! which is what the reference Simple Features implementations do and what the
//! geometry actually says.
//!
//! **3. `equals` is `T*F**FFF*`, not the printed `TFFFTFFFT`.** The standard
//! prints `TFFFTFFFT` for `sfEquals`, `ehEquals` and `rcc8eq` alike (Tables 2, 3,
//! 4, 6, 7, 8). Position 4 of that pattern is `boundary ∩ boundary`, and it
//! demands `T` — a **non-empty** boundary intersection. But a `Point` and a
//! `MultiPoint` have an *empty boundary* by definition, and so does a closed
//! curve; there is nothing there for the intersection to be non-empty in. Read
//! literally, `geof:sfEquals("POINT(1 1)", "POINT(1 1)")` is therefore `false`,
//! and so is every `rcc8eq` between two identical closed regions traced as
//! curves. A test in this crate caught exactly that
//! ([`equals_holds_between_two_identical_points`](self)).
//!
//! The pattern implemented instead is `T*F**FFF*`, which is precisely
//! `within AND contains` — the conjunction of the standard's own `WITHIN` and
//! `CONTAINS` patterns, character by character, and the definition equality
//! actually has: two sets are equal when each contains the other. It agrees with
//! `TFFFTFFFT` on every pair of geometries that *have* boundaries, which is why
//! the defect is invisible until a point is involved, and it is the pattern the
//! reference Simple Features implementations use.
//!
//! The exclusions the standard *does* state are honoured: `sfTouches` is `false`
//! for a point/point pair (two points cannot touch without their interiors
//! meeting), and `sfOverlaps` is `false` whenever the two geometries have
//! different topological dimensions.

use crate::de9im::{IntersectionMatrix, Pattern, Set};

/// Which of GeoSPARQL's three relation families a relation belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RelationFamily {
    /// The OGC Simple Features family (`geo:sf*` / `geof:sf*`).
    SimpleFeatures,
    /// The Egenhofer family (`geo:eh*` / `geof:eh*`).
    Egenhofer,
    /// The Region Connection Calculus 8 family (`geo:rcc8*` / `geof:rcc8*`).
    Rcc8,
}

impl RelationFamily {
    /// Every family, in a fixed order.
    pub const ALL: [Self; 3] = [Self::SimpleFeatures, Self::Egenhofer, Self::Rcc8];
}

/// One of the twenty-four topological relations.
///
/// The variant order is the order the standard's own tables list them in, and
/// [`SpatialRelation::ALL`] preserves it, so a registration walk is a pure
/// function of this enum rather than of any map's iteration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SpatialRelation {
    /// Simple Features: identical point sets.
    SfEquals,
    /// Simple Features: no point in common.
    SfDisjoint,
    /// Simple Features: at least one point in common.
    SfIntersects,
    /// Simple Features: boundaries meet, interiors do not.
    SfTouches,
    /// Simple Features: interiors meet in a set of lower dimension than both.
    SfCrosses,
    /// Simple Features: the first lies entirely inside the second.
    SfWithin,
    /// Simple Features: the second lies entirely inside the first.
    SfContains,
    /// Simple Features: same dimension, interiors meet, neither contains the other.
    SfOverlaps,
    /// Egenhofer: identical point sets.
    EhEquals,
    /// Egenhofer: no point in common.
    EhDisjoint,
    /// Egenhofer: boundaries touch, interiors do not meet.
    EhMeet,
    /// Egenhofer: interiors meet and each has points outside the other.
    EhOverlap,
    /// Egenhofer: the first covers the second (boundaries may coincide).
    EhCovers,
    /// Egenhofer: the converse of [`Self::EhCovers`].
    EhCoveredBy,
    /// Egenhofer: the first is strictly inside the second, no boundary contact.
    EhInside,
    /// Egenhofer: the converse of [`Self::EhInside`].
    EhContains,
    /// RCC8: identical regions.
    Rcc8Eq,
    /// RCC8: disconnected — nothing in common, not even boundary.
    Rcc8Dc,
    /// RCC8: externally connected — boundaries touch, interiors disjoint.
    Rcc8Ec,
    /// RCC8: partially overlapping.
    Rcc8Po,
    /// RCC8: tangential proper part inverse.
    Rcc8Tppi,
    /// RCC8: tangential proper part.
    Rcc8Tpp,
    /// RCC8: non-tangential proper part.
    Rcc8Ntpp,
    /// RCC8: non-tangential proper part inverse.
    Rcc8Ntppi,
}

// The pattern constants, spelled exactly as the standard's tables spell them so a
// reader can diff this file against Tables 2, 3 and 4 line by line. `Pattern::new`
// is `const`, so a mistyped pattern is a build failure here rather than a
// predicate that quietly answers the wrong question at a call site.

/// `T*F**FFF*` — identical point sets: the interiors meet, and neither geometry
/// has any interior or boundary outside the other. Shared by `sfEquals`,
/// `ehEquals` and `rcc8eq`, which the standard defines with one pattern.
///
/// **This is not the nine characters the standard prints.** See the module docs'
/// third departure for why `TFFFTFFFT` cannot be implemented literally.
const EQUALS: Pattern = Pattern::new("T*F**FFF*");
/// `FF*FF****` — no point in common. Shared by `sfDisjoint` and `ehDisjoint`.
const DISJOINT: Pattern = Pattern::new("FF*FF****");
/// The four-row union of Table 2: any of the four interior/boundary cells is
/// non-empty. See the module docs for why Table 6's rendering is not used.
const INTERSECTS: [Pattern; 4] = [
    Pattern::new("T********"),
    Pattern::new("*T*******"),
    Pattern::new("***T*****"),
    Pattern::new("****T****"),
];
/// The three-row union shared by `sfTouches` and `ehMeet`: the interiors are
/// disjoint but some pair of boundary/interior cells meets.
const TOUCHES: [Pattern; 3] = [
    Pattern::new("FT*******"),
    Pattern::new("F**T*****"),
    Pattern::new("F***T****"),
];
/// `T*F**F***` — the first lies entirely inside the second.
const WITHIN: Pattern = Pattern::new("T*F**F***");
/// `T*****FF*` — the transpose of `WITHIN`.
const CONTAINS: Pattern = Pattern::new("T*****FF*");
/// `T*T***T**` — `sfOverlaps` for point/point and area/area, and `ehOverlap` for
/// every pair.
///
/// NOT `sfCrosses`: this pattern additionally requires `E/I ≠ ∅`, which
/// `sfCrosses` does not. Reusing it for the forward mixed-dimension crossing
/// pairs made `crosses` disagree with its own reversed arm — see
/// [`CROSSES_FORWARD`].
const OVERLAP: Pattern = Pattern::new("T*T***T**");
/// `T*T******` — `sfCrosses` for the forward mixed-dimension pairs (point/curve,
/// point/area, curve/area), exactly as OGC Simple Features prints it.
///
/// The interiors must meet and the first's interior must reach the second's
/// exterior. There is deliberately no `E/I` cell: requiring one (as [`OVERLAP`]
/// does) is the `sfOverlaps` reading, and it is strictly stronger, so using it
/// here made `sfCrosses(a, b)` and `sfCrosses(b, a)` disagree for the same
/// crossing — a silent wrong answer produced by argument order alone, which is
/// the exact defect the reversed arm exists to prevent.
const CROSSES_FORWARD: Pattern = Pattern::new("T*T******");
/// `1*T***T**` — `sfOverlaps` for curve/curve: the shared interior must itself be
/// one-dimensional, or the two curves merely cross at points and that is
/// `sfCrosses`, not `sfOverlaps`.
const OVERLAP_CURVES: Pattern = Pattern::new("1*T***T**");
/// `0********` — `sfCrosses` for curve/curve: the interiors meet in points only.
const CROSSES_CURVES: Pattern = Pattern::new("0********");
/// `T*****T**` — the transpose of [`CROSSES_FORWARD`], used for the reversed
/// mixed-dimension pairs. See the module docs.
///
/// Transposing `T*T******` maps cell `I/E` (index 2) to `E/I` (index 6) and
/// leaves `I/I` (index 0) where it is, which is precisely this pattern. That
/// identity is what makes `sfCrosses` symmetric, and it is asserted by
/// `the_two_crosses_arms_are_transposes`.
const CROSSES_REVERSED: Pattern = Pattern::new("T*****T**");
/// `T*TFT*FF*` — Egenhofer covers.
const EH_COVERS: Pattern = Pattern::new("T*TFT*FF*");
/// `TFF*TFT**` — Egenhofer coveredBy.
const EH_COVERED_BY: Pattern = Pattern::new("TFF*TFT**");
/// `TFF*FFT**` — Egenhofer inside.
const EH_INSIDE: Pattern = Pattern::new("TFF*FFT**");
/// `T*TFF*FF*` — Egenhofer contains.
const EH_CONTAINS: Pattern = Pattern::new("T*TFF*FF*");
/// `FFTFFTTTT` — RCC8 disconnected.
const RCC8_DC: Pattern = Pattern::new("FFTFFTTTT");
/// `FFTFTTTTT` — RCC8 externally connected.
const RCC8_EC: Pattern = Pattern::new("FFTFTTTTT");
/// `TTTTTTTTT` — RCC8 partially overlapping.
const RCC8_PO: Pattern = Pattern::new("TTTTTTTTT");
/// `TTTFTTFFT` — RCC8 tangential proper part inverse.
const RCC8_TPPI: Pattern = Pattern::new("TTTFTTFFT");
/// `TFFTTFTTT` — RCC8 tangential proper part.
const RCC8_TPP: Pattern = Pattern::new("TFFTTFTTT");
/// `TFFTFFTTT` — RCC8 non-tangential proper part.
const RCC8_NTPP: Pattern = Pattern::new("TFFTFFTTT");
/// `TTTFFTFFT` — RCC8 non-tangential proper part inverse.
const RCC8_NTPPI: Pattern = Pattern::new("TTTFFTFFT");

impl SpatialRelation {
    /// All twenty-four relations, in the order the standard's tables list them.
    pub const ALL: [Self; 24] = [
        Self::SfEquals,
        Self::SfDisjoint,
        Self::SfIntersects,
        Self::SfTouches,
        Self::SfCrosses,
        Self::SfWithin,
        Self::SfContains,
        Self::SfOverlaps,
        Self::EhEquals,
        Self::EhDisjoint,
        Self::EhMeet,
        Self::EhOverlap,
        Self::EhCovers,
        Self::EhCoveredBy,
        Self::EhInside,
        Self::EhContains,
        Self::Rcc8Eq,
        Self::Rcc8Dc,
        Self::Rcc8Ec,
        Self::Rcc8Po,
        Self::Rcc8Tppi,
        Self::Rcc8Tpp,
        Self::Rcc8Ntpp,
        Self::Rcc8Ntppi,
    ];

    /// The local name this relation carries in both the `geo:` property
    /// vocabulary and the `geof:` function vocabulary — the standard uses the
    /// same local name for the property and its function (Tables 9, 10 and 11).
    #[must_use]
    pub const fn local_name(self) -> &'static str {
        match self {
            Self::SfEquals => "sfEquals",
            Self::SfDisjoint => "sfDisjoint",
            Self::SfIntersects => "sfIntersects",
            Self::SfTouches => "sfTouches",
            Self::SfCrosses => "sfCrosses",
            Self::SfWithin => "sfWithin",
            Self::SfContains => "sfContains",
            Self::SfOverlaps => "sfOverlaps",
            Self::EhEquals => "ehEquals",
            Self::EhDisjoint => "ehDisjoint",
            Self::EhMeet => "ehMeet",
            Self::EhOverlap => "ehOverlap",
            Self::EhCovers => "ehCovers",
            Self::EhCoveredBy => "ehCoveredBy",
            Self::EhInside => "ehInside",
            Self::EhContains => "ehContains",
            Self::Rcc8Eq => "rcc8eq",
            Self::Rcc8Dc => "rcc8dc",
            Self::Rcc8Ec => "rcc8ec",
            Self::Rcc8Po => "rcc8po",
            Self::Rcc8Tppi => "rcc8tppi",
            Self::Rcc8Tpp => "rcc8tpp",
            Self::Rcc8Ntpp => "rcc8ntpp",
            Self::Rcc8Ntppi => "rcc8ntppi",
        }
    }

    /// The relation with this local name, or `None`.
    #[must_use]
    pub fn from_local_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|relation| relation.local_name() == name)
    }

    /// Which family this relation belongs to.
    #[must_use]
    pub const fn family(self) -> RelationFamily {
        match self {
            Self::SfEquals
            | Self::SfDisjoint
            | Self::SfIntersects
            | Self::SfTouches
            | Self::SfCrosses
            | Self::SfWithin
            | Self::SfContains
            | Self::SfOverlaps => RelationFamily::SimpleFeatures,
            Self::EhEquals
            | Self::EhDisjoint
            | Self::EhMeet
            | Self::EhOverlap
            | Self::EhCovers
            | Self::EhCoveredBy
            | Self::EhInside
            | Self::EhContains => RelationFamily::Egenhofer,
            _ => RelationFamily::Rcc8,
        }
    }

    /// Whether `matrix` — the DE-9IM matrix of the ordered pair `(a, b)` — puts
    /// `a` and `b` in this relation, given their topological dimensions.
    ///
    /// `dim_a` and `dim_b` are the largest topological dimension present in each
    /// geometry (`-1` empty, `0` points, `1` curves, `2` areas). They are read by
    /// exactly the three relations the standard type-dispatches — `sfTouches`,
    /// `sfCrosses` and `sfOverlaps` — and ignored by the other twenty-one. See
    /// the module docs for the two places this table knowingly departs from the
    /// published tables.
    #[must_use]
    pub fn holds(self, matrix: &IntersectionMatrix, dim_a: i32, dim_b: i32) -> bool {
        match self {
            Self::SfEquals | Self::EhEquals | Self::Rcc8Eq => matrix.matches(&EQUALS),
            Self::SfDisjoint | Self::EhDisjoint => matrix.matches(&DISJOINT),
            Self::SfIntersects => matrix.matches_any(&INTERSECTS),
            Self::SfTouches | Self::EhMeet => {
                // The standard's one stated exclusion: two point geometries
                // cannot touch, because a point has no boundary and so any
                // meeting at all is an interior/interior meeting.
                if dim_a == 0 && dim_b == 0 {
                    return false;
                }
                matrix.matches_any(&TOUCHES)
            }
            Self::SfCrosses => crosses(matrix, dim_a, dim_b),
            Self::SfWithin => matrix.matches(&WITHIN),
            Self::SfContains => matrix.matches(&CONTAINS),
            Self::SfOverlaps => overlaps(matrix, dim_a, dim_b),
            Self::EhOverlap => matrix.matches(&OVERLAP),
            Self::EhCovers => matrix.matches(&EH_COVERS),
            Self::EhCoveredBy => matrix.matches(&EH_COVERED_BY),
            Self::EhInside => matrix.matches(&EH_INSIDE),
            Self::EhContains => matrix.matches(&EH_CONTAINS),
            Self::Rcc8Dc => matrix.matches(&RCC8_DC),
            Self::Rcc8Ec => matrix.matches(&RCC8_EC),
            Self::Rcc8Po => matrix.matches(&RCC8_PO),
            Self::Rcc8Tppi => matrix.matches(&RCC8_TPPI),
            Self::Rcc8Tpp => matrix.matches(&RCC8_TPP),
            Self::Rcc8Ntpp => matrix.matches(&RCC8_NTPP),
            Self::Rcc8Ntppi => matrix.matches(&RCC8_NTPPI),
        }
    }
}

/// `sfCrosses`, which the standard type-dispatches.
///
/// Forward mixed-dimension pairs (point/curve, point/area, curve/area) read
/// `T*T******`; curve/curve reads `0********`; the reversed mixed pairs read the
/// transposed pattern rather than answering `false`, for the reason the module
/// docs give. Equal non-curve dimensions cannot cross — that is `sfOverlaps`.
fn crosses(matrix: &IntersectionMatrix, dim_a: i32, dim_b: i32) -> bool {
    match (dim_a, dim_b) {
        (1, 1) => matrix.matches(&CROSSES_CURVES),
        (0, 1 | 2) | (1, 2) => matrix.matches(&CROSSES_FORWARD),
        (1 | 2, 0) | (2, 1) => matrix.matches(&CROSSES_REVERSED),
        _ => false,
    }
}

/// `sfOverlaps`, which the standard type-dispatches.
///
/// Defined only for equal dimensions — a curve cannot overlap an area in the
/// Simple Features sense, it crosses it — with the curve/curve case demanding a
/// one-dimensional shared interior so that two curves merely crossing at a point
/// are not reported as overlapping.
fn overlaps(matrix: &IntersectionMatrix, dim_a: i32, dim_b: i32) -> bool {
    if dim_a != dim_b {
        return false;
    }
    match dim_a {
        0 | 2 => matrix.matches(&OVERLAP),
        1 => matrix.matches(&OVERLAP_CURVES),
        _ => false,
    }
}

/// The transpose of `matrix`: the DE-9IM matrix of the reversed pair.
///
/// Used by the tests below to state each converse law as an equality rather than
/// by recomputing a relate, and available to callers for the same reason.
#[must_use]
pub fn transpose(matrix: &IntersectionMatrix) -> IntersectionMatrix {
    let mut out = IntersectionMatrix::new();
    for row in Set::ALL {
        for column in Set::ALL {
            out.set(column, row, matrix.get(row, column));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{CROSSES_FORWARD, CROSSES_REVERSED, RelationFamily, SpatialRelation, transpose};
    use crate::de9im::{Dim, IntersectionMatrix, Set};

    fn matrix(text: &str) -> IntersectionMatrix {
        IntersectionMatrix::parse(text).expect("a nine-entry matrix")
    }

    /// `sfCrosses` must not depend on argument order.
    ///
    /// `crosses` dispatches on the ORDER of the two dimensions, so the property
    /// that matters is the behavioural one: swapping the operands (which
    /// transposes the matrix and swaps the dimensions) must not change the
    /// answer. This sweeps every matrix built from the cells a real `relate`
    /// produces, over every mixed-dimension pair, and it is the test that catches
    /// the arms drifting apart no matter which pattern is edited.
    #[test]
    fn crosses_does_not_depend_on_argument_order() {
        // The auditor's minimal witness, kept explicit so a regression names it:
        // MULTIPOINT(1 1, 3 3) against LINESTRING(1 1, 1 1).
        let forward = matrix("0F0FFFFF2");
        let reversed = matrix("0FFFFF0F2");
        assert_eq!(
            transpose(&forward),
            reversed,
            "the two matrices really are transposes, so the fixture is sound"
        );
        assert_eq!(
            SpatialRelation::SfCrosses.holds(&forward, 0, 1),
            SpatialRelation::SfCrosses.holds(&reversed, 1, 0),
            "sfCrosses answered differently for the same crossing read in the two \
             argument orders"
        );

        // And the sweep, over the cell alphabet a matrix can actually carry.
        let mut disagreements = 0usize;
        let mut trues = 0usize;
        for cells in 0..3usize.pow(5) {
            // Vary the five cells the crosses patterns constrain; the rest are
            // fixed, since `*` matches anything anyway.
            let mut digits = ['F'; 9];
            let mut rest = cells;
            for slot in [0usize, 2, 4, 6, 8] {
                digits[slot] = ['F', '0', '2'][rest % 3];
                rest /= 3;
            }
            let text: String = digits.iter().collect();
            let m = matrix(&text);
            let t = transpose(&m);
            for (da, db) in [(0, 1), (0, 2), (1, 2)] {
                let a = SpatialRelation::SfCrosses.holds(&m, da, db);
                let b = SpatialRelation::SfCrosses.holds(&t, db, da);
                if a != b {
                    disagreements += 1;
                }
                if a {
                    trues += 1;
                }
            }
        }
        assert_eq!(
            disagreements, 0,
            "sfCrosses must be order-independent for every matrix"
        );
        // Non-vacuity: a `crosses` that always answered false would sweep clean.
        assert!(
            trues > 0,
            "the sweep must contain crossings, or it proves nothing"
        );
    }

    /// The two `sfCrosses` arms must be transposes of one another, cell by cell.
    ///
    /// `crosses` dispatches on the ORDER of the two dimensions, so if the forward
    /// and reversed patterns are not transposes then `sfCrosses(a, b)` and
    /// `sfCrosses(b, a)` disagree for one crossing — a wrong answer produced by
    /// argument order alone, and a silent one, because both answers are just
    /// `false`. That is what happened when the forward arm reused the
    /// `sfOverlaps` pattern `T*T***T**`, whose extra `E/I` cell has no transpose
    /// in `T*****T**`.
    ///
    /// The two patterns have no public accessor, so this compares their rendered
    /// forms — which is also the form the standard prints them in.
    #[test]
    fn the_two_crosses_arms_are_transposes() {
        // Cell i = 3*row + col; transposing sends i to 3*col + row.
        let forward = format!("{CROSSES_FORWARD}");
        let reversed = format!("{CROSSES_REVERSED}");
        assert_eq!(forward.len(), 9, "a DE-9IM pattern has nine cells");
        let forward: Vec<char> = forward.chars().collect();
        let reversed: Vec<char> = reversed.chars().collect();
        for row in 0..3 {
            for col in 0..3 {
                assert_eq!(
                    forward[3 * row + col],
                    reversed[3 * col + row],
                    "cell ({row},{col}) of the forward arm must be cell ({col},{row}) of the \
                     reversed arm: {forward:?} vs {reversed:?}"
                );
            }
        }
    }

    /// Every relation has a distinct local name, and the round trip through it is
    /// total — the registration walk depends on both.
    #[test]
    fn local_names_are_distinct_and_round_trip() {
        let mut names: Vec<&str> = SpatialRelation::ALL
            .iter()
            .map(|r| r.local_name())
            .collect();
        assert_eq!(
            names.len(),
            24,
            "the standard defines twenty-four relations"
        );
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "no two relations share a local name");

        for relation in SpatialRelation::ALL {
            assert_eq!(
                SpatialRelation::from_local_name(relation.local_name()),
                Some(relation),
                "{} must round trip",
                relation.local_name()
            );
        }
        assert_eq!(
            SpatialRelation::from_local_name("sfNotAThing"),
            None,
            "an unknown local name resolves to nothing rather than to a default"
        );
    }

    /// The RCC8 names are `rcc8dc` and `rcc8ec`. The standard's Annex B.1
    /// summary table prints `rcc8dcc` and `rcc8ecc`; the ontology, the rule
    /// vocabulary and Tables 4, 8 and 11 all say otherwise, and this pins the
    /// spelling the rest of the standard agrees on.
    #[test]
    fn the_rcc8_local_names_are_the_two_letter_ones() {
        assert_eq!(SpatialRelation::Rcc8Dc.local_name(), "rcc8dc");
        assert_eq!(SpatialRelation::Rcc8Ec.local_name(), "rcc8ec");
        assert_eq!(SpatialRelation::from_local_name("rcc8dcc"), None);
        assert_eq!(SpatialRelation::from_local_name("rcc8ecc"), None);
    }

    #[test]
    fn each_family_holds_exactly_eight_relations() {
        for family in RelationFamily::ALL {
            let count = SpatialRelation::ALL
                .iter()
                .filter(|r| r.family() == family)
                .count();
            assert_eq!(count, 8, "{family:?} has eight members");
        }
    }

    // ---- the departures from the published tables ------------------------

    /// Table 5 of the standard asserts `intersects | ¬ disconnected |
    /// ¬ disjoint`. Table 6's `sfIntersects` pattern would break it; Table 2's
    /// does not. This is the test that pins which table this crate implements.
    #[test]
    fn intersects_is_the_negation_of_disjoint() {
        let cases = [
            "0FFFFF212", // a point strictly inside a polygon
            "F0FFFF212", // a point on a polygon's boundary
            "FF0FFF212", // a point outside a polygon
            "212101212", // two properly overlapping polygons
            "FF2FF1212", // two disjoint polygons
            "FF2F01212", // two polygons sharing one boundary point
            "2FFF1FFF2", // two identical polygons
        ];
        for text in cases {
            let m = matrix(text);
            let intersects = SpatialRelation::SfIntersects.holds(&m, 2, 2);
            let disjoint = SpatialRelation::SfDisjoint.holds(&m, 2, 2);
            assert_ne!(
                intersects, disjoint,
                "{text}: intersects must be exactly the negation of disjoint"
            );
        }
        // The specific case Table 6 would have got wrong.
        let point_in_polygon = matrix("0FFFFF212");
        assert!(
            SpatialRelation::SfIntersects.holds(&point_in_polygon, 0, 2),
            "a point strictly inside a polygon intersects it; Table 6's pattern says otherwise"
        );
        assert!(
            !SpatialRelation::SfTouches.holds(&point_in_polygon, 0, 2),
            "and it does NOT touch it — which is what Table 6 conflated the two into"
        );
    }

    /// The third departure, as a regression test: the printed `TFFFTFFFT`
    /// demands a non-empty boundary/boundary cell, and a point has no boundary,
    /// so the literal pattern answers `false` for two identical points.
    #[test]
    fn equals_holds_between_two_identical_points() {
        let two_identical_points = matrix("0FFFFFFF2");
        for relation in [
            SpatialRelation::SfEquals,
            SpatialRelation::EhEquals,
            SpatialRelation::Rcc8Eq,
        ] {
            assert!(
                relation.holds(&two_identical_points, 0, 0),
                "{relation:?} must hold between two identical points; the printed \
                 TFFFTFFFT pattern answers false because a point has no boundary"
            );
        }
        // The CONTROL: two DISTINCT points must still not be equal, so the
        // relaxation above has not turned equality into a tautology.
        let two_distinct_points = matrix("FF0FFF0F2");
        for relation in [
            SpatialRelation::SfEquals,
            SpatialRelation::EhEquals,
            SpatialRelation::Rcc8Eq,
        ] {
            assert!(
                !relation.holds(&two_distinct_points, 0, 0),
                "{relation:?} must NOT hold between two distinct points"
            );
        }
        // And it still agrees with the printed pattern wherever boundaries exist.
        let two_identical_polygons = matrix("2FFF1FFF2");
        assert!(SpatialRelation::SfEquals.holds(&two_identical_polygons, 2, 2));
        for text in [
            "212101212",
            "FF2FF1212",
            "2FF1FF212",
            "212FF1FF2",
            "FF2F11212",
        ] {
            assert!(
                !SpatialRelation::SfEquals.holds(&matrix(text), 2, 2),
                "{text} is not an equality"
            );
        }
    }

    /// Equality is exactly mutual containment — the property that justifies the
    /// pattern this crate implements, checked against the standard's own two
    /// patterns rather than against the constant derived from them.
    #[test]
    fn equals_is_exactly_within_and_contains() {
        for text in [
            "0FFFFFFF2",
            "FF0FFF0F2",
            "2FFF1FFF2",
            "212101212",
            "FF2FF1212",
            "2FF1FF212",
            "212FF1FF2",
            "FF2F11212",
            "0FFFFF212",
            "1FFF0FFF2",
        ] {
            let m = matrix(text);
            assert_eq!(
                SpatialRelation::SfEquals.holds(&m, 2, 2),
                SpatialRelation::SfWithin.holds(&m, 2, 2)
                    && SpatialRelation::SfContains.holds(&m, 2, 2),
                "{text}: equality is mutual containment"
            );
        }
    }

    /// `sfCrosses` must not depend on which argument was written first.
    #[test]
    fn crosses_answers_the_reversed_pair_the_same_way() {
        // A curve crossing an area: interiors meet, and the curve has interior
        // outside the area.
        let curve_crosses_area = matrix("102FF1212");
        assert!(
            SpatialRelation::SfCrosses.holds(&curve_crosses_area, 1, 2),
            "the forward pair crosses"
        );
        let reversed = transpose(&curve_crosses_area);
        assert!(
            SpatialRelation::SfCrosses.holds(&reversed, 2, 1),
            "and so does the reversed pair; answering false here would make the relation \
             depend on argument order alone"
        );
    }

    /// The exclusions the standard DOES state are honoured — and the
    /// neighbouring case that is not excluded still answers.
    #[test]
    fn the_stated_exclusions_hold_and_their_neighbours_still_answer() {
        // Two points cannot touch.
        let two_points_meeting = matrix("0FFFFFFF2");
        assert!(
            !SpatialRelation::SfTouches.holds(&two_points_meeting, 0, 0),
            "point/point is excluded from sfTouches"
        );
        // The neighbouring VALID case: a point touching a curve's endpoint.
        let point_on_curve_end = matrix("FF0F0F102");
        assert!(
            SpatialRelation::SfTouches.holds(&point_on_curve_end, 0, 1),
            "point/curve is NOT excluded, and must still answer true"
        );

        // sfOverlaps needs equal dimensions.
        let mixed = matrix("102FF1212");
        assert!(
            !SpatialRelation::SfOverlaps.holds(&mixed, 1, 2),
            "a curve and an area have different dimensions and cannot overlap"
        );
        // The neighbouring VALID case: two areas with the same matrix shape.
        let two_areas = matrix("212101212");
        assert!(
            SpatialRelation::SfOverlaps.holds(&two_areas, 2, 2),
            "two properly overlapping areas DO overlap"
        );
        // And two curves need a one-dimensional shared interior.
        let curves_crossing_at_a_point = matrix("0F1FF0102");
        assert!(
            !SpatialRelation::SfOverlaps.holds(&curves_crossing_at_a_point, 1, 1),
            "curves meeting at a point cross, they do not overlap"
        );
        assert!(
            SpatialRelation::SfCrosses.holds(&curves_crossing_at_a_point, 1, 1),
            "and the relation they DO satisfy is sfCrosses"
        );
    }

    // ---- the converse laws ------------------------------------------------

    /// `within` and `contains` are converses, as are the Egenhofer and RCC8
    /// inside/contains and proper-part pairs. Asserting them against the
    /// transposed matrix is an independent check on the pattern constants: a
    /// mistyped pattern breaks the law even though the pattern still parses.
    #[test]
    fn the_converse_pairs_are_transposes_of_one_another() {
        let cases = [
            "0FFFFF212",
            "212FF1FF2",
            "212101212",
            "FF2FF1212",
            "2FFF1FFF2",
            "FF2F11212",
            "212F11FF2",
            "1FF0FF212",
        ];
        let converses = [
            (SpatialRelation::SfWithin, SpatialRelation::SfContains),
            (SpatialRelation::EhInside, SpatialRelation::EhContains),
            (SpatialRelation::EhCoveredBy, SpatialRelation::EhCovers),
            (SpatialRelation::Rcc8Tpp, SpatialRelation::Rcc8Tppi),
            (SpatialRelation::Rcc8Ntpp, SpatialRelation::Rcc8Ntppi),
        ];
        for text in cases {
            let forward = matrix(text);
            let reversed = transpose(&forward);
            for (left, right) in converses {
                assert_eq!(
                    left.holds(&forward, 2, 2),
                    right.holds(&reversed, 2, 2),
                    "{text}: {left:?} on the pair must equal {right:?} on the reversed pair"
                );
            }
        }
    }

    /// The symmetric relations must answer identically on the transposed matrix.
    #[test]
    fn the_symmetric_relations_are_symmetric() {
        let cases = [
            "0FFFFF212",
            "212101212",
            "FF2FF1212",
            "2FFF1FFF2",
            "FF2F01212",
            "FF2F11212",
        ];
        let symmetric = [
            SpatialRelation::SfEquals,
            SpatialRelation::SfDisjoint,
            SpatialRelation::SfIntersects,
            SpatialRelation::SfTouches,
            SpatialRelation::SfOverlaps,
            SpatialRelation::EhEquals,
            SpatialRelation::EhDisjoint,
            SpatialRelation::EhMeet,
            SpatialRelation::EhOverlap,
            SpatialRelation::Rcc8Eq,
            SpatialRelation::Rcc8Dc,
            SpatialRelation::Rcc8Ec,
            SpatialRelation::Rcc8Po,
        ];
        for text in cases {
            let forward = matrix(text);
            let reversed = transpose(&forward);
            for relation in symmetric {
                assert_eq!(
                    relation.holds(&forward, 2, 2),
                    relation.holds(&reversed, 2, 2),
                    "{text}: {relation:?} must be symmetric"
                );
            }
        }
    }

    /// The three families agree where Table 5 says they agree. This is a
    /// cross-family oracle: it checks twelve patterns against each other rather
    /// than against the code that produced them.
    ///
    /// One refinement to the table as printed. Table 5 gives
    /// `within | ntpp + tpp | inside + coveredBy`, but both right-hand sides are
    /// **proper**-part relations: `rcc8tpp`'s pattern requires the exterior of
    /// the first to meet the interior of the second, and `ehCoveredBy`'s requires
    /// the same, so both are `false` when the two regions are equal — while
    /// `sfWithin` is `true`, because a region is within itself. The equality case
    /// therefore has to be added explicitly, and this test states the law as
    /// `within == eq OR tpp OR ntpp`. That is not a disagreement with the
    /// standard; it is the standard's row read together with its own first row.
    #[test]
    fn the_families_agree_where_table_five_says_they_do() {
        // Table 5 is scoped to closed, non-empty regions, so every case here is
        // an area/area matrix.
        let regions = [
            "2FFF1FFF2", // equal
            "FF2FF1212", // disjoint / disconnected
            "FF2F11212", // touching along an edge / externally connected
            "212101212", // properly overlapping
            "2FF1FF212", // strictly inside
            "212FF1FF2", // strictly containing
        ];
        for text in regions {
            let m = matrix(text);
            assert_eq!(
                SpatialRelation::SfEquals.holds(&m, 2, 2),
                SpatialRelation::Rcc8Eq.holds(&m, 2, 2),
                "{text}: equals"
            );
            assert_eq!(
                SpatialRelation::SfDisjoint.holds(&m, 2, 2),
                SpatialRelation::Rcc8Dc.holds(&m, 2, 2),
                "{text}: disjoint is disconnected"
            );
            assert_eq!(
                SpatialRelation::SfTouches.holds(&m, 2, 2),
                SpatialRelation::Rcc8Ec.holds(&m, 2, 2),
                "{text}: touches is externally connected"
            );
            assert_eq!(
                SpatialRelation::SfOverlaps.holds(&m, 2, 2),
                SpatialRelation::Rcc8Po.holds(&m, 2, 2),
                "{text}: overlaps is partially overlapping"
            );
            let equal = SpatialRelation::Rcc8Eq.holds(&m, 2, 2);
            assert_eq!(
                SpatialRelation::SfWithin.holds(&m, 2, 2),
                equal
                    || SpatialRelation::Rcc8Ntpp.holds(&m, 2, 2)
                    || SpatialRelation::Rcc8Tpp.holds(&m, 2, 2),
                "{text}: within is eq OR ntpp OR tpp (the RCC8 parts are PROPER)"
            );
            assert_eq!(
                SpatialRelation::SfContains.holds(&m, 2, 2),
                equal
                    || SpatialRelation::Rcc8Ntppi.holds(&m, 2, 2)
                    || SpatialRelation::Rcc8Tppi.holds(&m, 2, 2),
                "{text}: contains is eq OR ntppi OR tppi"
            );
            assert_eq!(
                SpatialRelation::SfWithin.holds(&m, 2, 2),
                SpatialRelation::EhEquals.holds(&m, 2, 2)
                    || SpatialRelation::EhInside.holds(&m, 2, 2)
                    || SpatialRelation::EhCoveredBy.holds(&m, 2, 2),
                "{text}: within is equals OR inside OR coveredBy"
            );
            // The control: the equality disjunct is not doing all the work.
            assert!(
                !equal || text == "2FFF1FFF2",
                "{text}: only the equal fixture may be equal"
            );
        }
    }

    #[test]
    fn transpose_is_an_involution_and_swaps_the_off_diagonal() {
        // Deliberately asymmetric off-diagonals, so the swap is observable.
        let m = matrix("012201212");
        assert_eq!(m.get(Set::Interior, Set::Boundary), Dim::One);
        assert_eq!(m.get(Set::Boundary, Set::Interior), Dim::Two);
        let t = transpose(&m);
        assert_eq!(
            t.get(Set::Interior, Set::Boundary),
            Dim::Two,
            "the transpose reads the original's (boundary, interior)"
        );
        assert_eq!(t.get(Set::Boundary, Set::Interior), Dim::One);
        assert_eq!(
            t.get(Set::Interior, Set::Interior),
            m.get(Set::Interior, Set::Interior),
            "the diagonal is fixed"
        );
        assert_eq!(transpose(&t), m, "transposing twice is the identity");
    }
}
