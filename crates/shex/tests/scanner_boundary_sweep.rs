// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Where the token actually stops: a boundary-exhaustive sweep of this crate's
//! two scan entry points — the ShExC lexer and the ShapeMap scanner.
//!
//! # Why a predicate test cannot reach this
//!
//! `purrdf_iri::terminals` proves its range tables against an independent
//! transcription on all 1,114,112 scalars, so *membership* is settled. A scanner
//! does not ask membership. It asks **where a token ends**, and it answers that
//! by composing a membership test with everything around it: maximal munch, the
//! separate keyword-follow check, the trailing-dot pushback, and a whitespace
//! skip that runs only where the grammar puts one. Each of those is correct by
//! parts and can still be wrong by composition, and the composition is exactly
//! what no predicate test can see.
//!
//! The two scanners here are genuinely different machines. The ShExC lexer emits
//! a token stream; the ShapeMap scanner is a recursive-descent character cursor
//! with no token type at all, so its observable is the parsed map. Checking one
//! and arguing the other by prose would leave precisely the gap this file is for.
//!
//! # What this file is NOT an oracle for
//!
//! Read this before citing the sweep as coverage for a **table** change, because
//! the sweep will not tell you. Every probe's expected answer is read off the
//! very predicate the scanner consults, and the probe corpus is derived from that
//! predicate too. So for the six classes this crate does not transcribe — `WS`,
//! `PN_CHARS_BASE`, `PN_CHARS_U`, `PN_CHARS` and the two `VARNAME` positions, all
//! of them `purrdf_iri::terminals` functions — widening a range by one scalar
//! moves the corpus, the `admits` answer and the scanner in lockstep, and every
//! sweep here stays green. That was measured, not assumed: one scalar added to
//! `PN_CHARS_BASE`'s `[#x2C00-#x2FEF]` left both sweeps passing.
//!
//! That is a statement of scope rather than a defect to repair here. A scanner
//! test carrying its own copy of the tables would be a third transcription of a
//! production the workspace already owns, which is the very mistake
//! `purrdf_iri::terminals` exists to end. This file proves the **composition**;
//! the tables' contents are proved in two other places, and a tightening is
//! covered only if one of them moved:
//!
//! * `purrdf_iri::terminals` itself — an independent `matches!` transcription of
//!   each class, checked against the range table over all 1,114,112 scalars, plus
//!   the `terminal!` macro's `cardinality`, `ranges_all_ascii` and
//!   `ranges_sorted_disjoint` const assertions. This is the primary table oracle.
//! * `purrdf-sparql-algebra`'s sibling `tests/scanner_boundary_sweep.rs`, whose
//!   `the_derived_ranges_are_the_snapshotted_ones` renders those same six classes
//!   into a tracked snapshot, so a table edit reaches review as a range diff.
//!   That snapshot is deliberately **not** duplicated here — see
//!   [`UNSHARED_PRODUCTIONS`] for the one class it does not cover and
//!   [`the_locally_transcribed_ranges_are_the_snapshotted_ones`] for where that
//!   one is pinned instead.
//!
//! The two content classes are the exception, and the difference is worth
//! understanding rather than levelling away: `[18t] IRIREF` and
//! `[14t] STRING_LITERAL2` are each spelled **twice** — once by the scanner
//! (`terminals::is_iriref_forbidden`, and `shapemap.rs`'s own
//! `is_string_literal_content`) and once, independently, by [`iriref_content`]
//! and [`string_literal_content`] below. Where the two transcriptions have to
//! agree scalar for scalar, the sweep *is* a table oracle; where the expectation
//! delegates to the implementation, it cannot be.
//!
//! # Boundary-exhaustive, which is where completeness is possible
//!
//! A `format!`-plus-scan call for every scalar, times several productions, times
//! each scanner, is minutes on every gate run. It is also unnecessary: a range
//! table can only be wrong **at a boundary**, so the gated corpus is `lo - 1`,
//! `lo`, `hi`, `hi + 1` for every range of every production — derived from the
//! predicates themselves, never hand-listed — plus the whole Unicode
//! `White_Space` property swept out of [`char::is_whitespace`], the invisible
//! format characters, and `[18t] IRIREF`'s own delimiters. The full lane is kept,
//! `#[ignore]`d, for anyone who wants it.
//!
//! # What is asserted, and what is deliberately not
//!
//! Per probe the claim is three-valued and reads off the cited production:
//!
//! 1. the production admits the candidate at that position → the probe is ONE
//!    token (or one association) and its payload holds the candidate;
//! 2. the candidate is `WS` and the position has a separator reading → the probe
//!    splits into exactly the parts named;
//! 3. otherwise → the probe is **not** that reading, i.e. the candidate was not
//!    silently absorbed.
//!
//! Case 3 never pins an error arm. Which tokens result is the property; which
//! diagnostic fires is an accident of where the split happened to land.
//!
//! # Why the corpus derivation is spelled here as well as in the query front end
//!
//! `purrdf-shex` and `purrdf-sparql-algebra` are independent wasm-clean leaves —
//! neither depends on the other, and neither should start to for a test's sake.
//! The derivation is ~40 lines over `purrdf_iri::terminals`, which both already
//! carry, so each crate sweeps its own scanners from its own copy.
//!
//! # This does not replace the named vectors
//!
//! `tests/terminal_classes_unit.rs` stays. A failure here prints "U+2028
//! disagreed", which says nothing about *why* the boundary sits there; the named
//! vectors carry that reasoning, and the sweep carries the coverage.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use pretty_assertions::assert_eq;
use purrdf_core::TermValue;
use purrdf_iri::terminals;
use purrdf_shex::lexer::tokenize;
use purrdf_shex::{NodeSelector, ShapeMap, ShapeSelector, parse_shape_map};

// ── The scalar corpus, derived ────────────────────────────────────────────────

/// Every Unicode scalar value, in order.
fn all_scalars() -> impl Iterator<Item = char> {
    (0..=0x0010_FFFF_u32).filter_map(char::from_u32)
}

/// `WS ::= #x20 | #x9 | #xD | #xA` as a scalar test — the four scalars both
/// ShExC's `@pass` and ShapeMap's `PASSED TOKENS` name.
///
/// The shared production is byte-shaped because every member is ASCII and a byte
/// test is therefore exact over UTF-8; this file holds decoded scalars, so it
/// narrows first. `u8::try_from` fails above U+00FF and every Latin-1 scalar it
/// does yield is outside `WS`, so nothing non-ASCII can alias a member.
fn is_ws(c: char) -> bool {
    u8::try_from(c).is_ok_and(terminals::is_ws)
}

/// `[18t] IRIREF`'s content class: everything above `#x20` that is not one of the
/// nine delimiters the production excludes by name. Non-ASCII is never forbidden
/// raw, U+00A0 and U+007F included.
fn iriref_content(c: char) -> bool {
    u32::from(c) > 0x20 && !IRIREF_DELIMITERS.contains(&c)
}

/// `[14t] STRING_LITERAL2`'s content class:
/// `'"' ([^#x22#x5C#xA#xD] | ECHAR | UCHAR)* '"'`. Only four scalars leave it —
/// the closing quote, the escape lead, and the two line terminators. A raw TAB is
/// lawful, and so is every C1 control.
fn string_literal_content(c: char) -> bool {
    !matches!(c, '"' | '\\' | '\n' | '\r')
}

/// A production, named as the grammar names it, paired with the predicate that
/// decides membership in it.
type Production = (&'static str, fn(char) -> bool);

/// The productions whose ranges bound this sweep, each paired with the predicate
/// that decides membership. The corpus is read off these, so adding a production
/// here widens the sweep automatically.
const PRODUCTIONS: [Production; 8] = [
    ("WS", is_ws),
    ("PN_CHARS_BASE", terminals::is_pn_chars_base),
    ("PN_CHARS_U", terminals::is_pn_chars_u),
    ("PN_CHARS", terminals::is_pn_chars),
    ("VARNAME first scalar", terminals::is_varname_start),
    ("VARNAME later scalar", terminals::is_varname_continue),
    ("IRIREF content", iriref_content),
    ("STRING_LITERAL2 content", string_literal_content),
];

/// The member of [`PRODUCTIONS`] the query front end's sibling sweep does not
/// carry, and whose ranges are therefore pinned nowhere else in the workspace.
///
/// `[14t] STRING_LITERAL2`'s content class is a shape-map terminal: `shapemap.rs`
/// scans it with its own `matches!`, and [`string_literal_content`] transcribes
/// that independently for this file. Nothing outside `purrdf-shex` renders it.
///
/// The other seven members of [`PRODUCTIONS`] are excluded on purpose. Each is
/// the same class the sibling sweep already snapshots — the same six
/// `purrdf_iri::terminals` functions and the same nine-delimiter `IRIREF`
/// content rule, run through the same derivation — so re-rendering them would
/// copy a snapshot rather than add an oracle, and would leave two files to keep
/// in step for no claim either one does not already make.
const UNSHARED_PRODUCTIONS: [Production; 1] = [("STRING_LITERAL2 content", string_literal_content)];

/// The subset of [`PRODUCTIONS`] that decides where a NAME stops.
const NAME_PRODUCTIONS: [Production; 5] = [
    ("PN_CHARS_BASE", terminals::is_pn_chars_base),
    ("PN_CHARS_U", terminals::is_pn_chars_u),
    ("PN_CHARS", terminals::is_pn_chars),
    ("VARNAME first scalar", terminals::is_varname_start),
    ("VARNAME later scalar", terminals::is_varname_continue),
];

/// The contiguous scalar ranges a predicate admits, read off the **predicate**
/// rather than off any table it happens to be implemented with.
///
/// The surrogate gap is treated as a non-member run, so a class that spans it
/// shows here as two ranges.
fn derived_ranges(admits: fn(char) -> bool) -> Vec<(u32, u32)> {
    let mut ranges: Vec<(u32, u32)> = Vec::new();
    for cp in 0..=0x0010_FFFF_u32 {
        if !char::from_u32(cp).is_some_and(admits) {
            continue;
        }
        match ranges.pop() {
            Some((lo, hi)) if hi + 1 == cp => ranges.push((lo, cp)),
            Some(previous) => {
                ranges.push(previous);
                ranges.push((cp, cp));
            }
            None => ranges.push((cp, cp)),
        }
    }
    ranges
}

/// The invisible format characters. None is `White_Space`, so a whitespace audit
/// never looks at them, and three of the six sit inside a name class while three
/// sit in the holes beside it — which is the whole point.
const INVISIBLES: [char; 6] = [
    '\u{00ad}', // SOFT HYPHEN
    '\u{200b}', // ZERO WIDTH SPACE
    '\u{200e}', // LEFT-TO-RIGHT MARK
    '\u{200f}', // RIGHT-TO-LEFT MARK
    '\u{2060}', // WORD JOINER
    '\u{feff}', // ZERO WIDTH NO-BREAK SPACE (byte-order mark)
];

/// The nine scalars `[18t] IRIREF` excludes by name, beyond its `#x00-#x20` range.
const IRIREF_DELIMITERS: [char; 9] = ['<', '>', '"', '{', '}', '|', '^', '`', '\\'];

/// The gated corpus: boundaries, all of Unicode `White_Space`, the invisibles and
/// the `IRIREF` delimiters.
fn probe_scalars() -> Vec<char> {
    let mut out = BTreeSet::new();
    for (_, admits) in PRODUCTIONS {
        for (lo, hi) in derived_ranges(admits) {
            for cp in [lo.wrapping_sub(1), lo, hi, hi + 1] {
                out.extend(char::from_u32(cp));
            }
        }
    }
    out.extend(all_scalars().filter(|c| c.is_whitespace()));
    out.extend(INVISIBLES);
    out.extend(IRIREF_DELIMITERS);
    out.into_iter().collect()
}

// ── What a scanner made of a probe ────────────────────────────────────────────

/// The reading a scan entry point gave a probe.
///
/// "Parts" because the two entry points observe different things: the lexer's
/// parts are its tokens, the shape map's are its associations. The three-valued
/// claim is the same either way.
#[derive(Clone, PartialEq, Eq, Debug)]
enum Reading {
    /// The probe has no reading in this grammar.
    Refused,
    /// The parts the probe resolved into, rendered.
    Parts(Vec<String>),
}

/// Tokenize as ShExC — [`tokenize`], the schema lexer.
fn shexc(probe: &str) -> Reading {
    tokenize(probe).map_or(Reading::Refused, |tokens| {
        Reading::Parts(tokens.iter().map(|s| format!("{:?}", s.token)).collect())
    })
}

/// Parse as a query shape map — [`parse_shape_map`], a character cursor with no
/// token type, whose observable is therefore the map itself.
fn shape_map(probe: &str) -> Reading {
    parse_shape_map(probe, None).map_or(Reading::Refused, |map| Reading::Parts(rendered(&map)))
}

/// A parsed map's associations, rendered one per part.
fn rendered(map: &ShapeMap) -> Vec<String> {
    map.0
        .iter()
        .map(|a| format!("{:?} @ {:?}", a.node, a.shape))
        .collect()
}

/// The reading a shape map of these associations would render as.
fn associations(pairs: Vec<(NodeSelector, ShapeSelector)>) -> Reading {
    Reading::Parts(
        pairs
            .into_iter()
            .map(|(node, shape)| format!("{node:?} @ {shape:?}"))
            .collect(),
    )
}

/// The reading of a one-association map selecting a concrete node against `START`.
fn sole_node(term: TermValue) -> Reading {
    associations(vec![(NodeSelector::Node(term), ShapeSelector::Start)])
}

/// A `Word` token, as the derived `Debug` spells it.
fn word(text: &str) -> String {
    format!("Word({text:?})")
}

/// A `PName` token, rendered.
fn pname(prefix: &str, local: &str) -> String {
    format!("PName({prefix:?}, {local:?})")
}

/// A `BNode` token, rendered.
fn bnode(label: &str) -> String {
    format!("BNode({label:?})")
}

/// An `Iri` token, rendered.
fn iri(body: &str) -> String {
    format!("Iri({body:?})")
}

/// A `StringLit` token, rendered.
fn string_lit(body: &str) -> String {
    format!("StringLit({body:?})")
}

/// A `LangTag` token, rendered.
fn lang_tag(body: &str) -> String {
    format!("LangTag({body:?})")
}

/// A reading of exactly the parts given.
fn split(parts: &[String]) -> Reading {
    Reading::Parts(parts.to_vec())
}

// ── The sweep table ───────────────────────────────────────────────────────────

/// One production, swept at one position.
struct Sweep {
    /// The production and position, cited by grammar number.
    cited: &'static str,
    /// The probe text a candidate scalar is dropped into.
    probe: Box<dyn Fn(char) -> String>,
    /// Whether the production admits the candidate **at that position**.
    admits: Box<dyn Fn(char) -> bool>,
    /// The joined reading an admitted candidate must produce.
    joined: Box<dyn Fn(char) -> Reading>,
    /// The reading a `WS` candidate must produce, where the position has one.
    /// `None` where `WS` is what the production itself admits.
    separated: Option<Box<dyn Fn(char) -> Reading>>,
}

/// Drive every sweep over every candidate, returning the number of probes run.
fn run(scan: fn(&str) -> Reading, sweeps: &[Sweep], scalars: &[char]) -> usize {
    let mut cases = 0;
    for sweep in sweeps {
        let cited = sweep.cited;
        for &c in scalars {
            let probe = (sweep.probe)(c);
            let observed = scan(&probe);
            let joined = (sweep.joined)(c);
            if (sweep.admits)(c) {
                assert_eq!(
                    observed,
                    joined,
                    "U+{:04X} is admitted by {cited}, so {probe:?} reads as one",
                    u32::from(c)
                );
            } else if is_ws(c)
                && let Some(separated) = sweep.separated.as_ref()
            {
                assert_eq!(
                    observed,
                    separated(c),
                    "U+{:04X} is `WS`, so it separates in {probe:?}",
                    u32::from(c)
                );
            } else {
                assert_ne!(
                    observed,
                    joined,
                    "U+{:04X} is not admitted by {cited}, so {probe:?} must not absorb it",
                    u32::from(c)
                );
            }
            cases += 1;
        }
    }
    cases
}

// ── The ShExC lexer ───────────────────────────────────────────────────────────

/// Every ShExC lexer sweep.
///
/// `BLANK_NODE_LABEL` is swept at BOTH positions, because the production is
/// position-dependent and both directions of getting that wrong are silent: a
/// head-only class refuses the lawful `_:cafe\u{301}`, a tail-only class accepts
/// the unlawful `_:\u{301}x`.
fn shexc_sweeps() -> Vec<Sweep> {
    vec![
        Sweep {
            cited: "[168s] PN_PREFIX continuation, under maximal munch",
            probe: Box::new(|c| format!("a{c}z")),
            admits: Box::new(|c| terminals::is_pn_chars(c) || c == '.'),
            joined: Box::new(|c| split(&[word(&format!("a{c}z"))])),
            separated: Some(Box::new(|_| split(&[word("a"), word("z")]))),
        },
        Sweep {
            cited: "[169s] PN_LOCAL continuation",
            probe: Box::new(|c| format!("ex:a{c}z")),
            admits: Box::new(|c| terminals::is_pn_chars(c) || c == ':' || c == '.'),
            joined: Box::new(|c| split(&[pname("ex", &format!("a{c}z"))])),
            separated: Some(Box::new(|_| split(&[pname("ex", "a"), word("z")]))),
        },
        Sweep {
            cited: "[169s] PN_LOCAL, then the trailing-dot pushback",
            // A dot run at the end may not close the production, so it is handed
            // back: `ex:a..` is not the local name `a.`.
            probe: Box::new(|c| format!("ex:a{c}.")),
            admits: Box::new(|c| terminals::is_pn_chars(c) || c == ':'),
            joined: Box::new(|c| split(&[pname("ex", &format!("a{c}")), "Dot".to_owned()])),
            separated: Some(Box::new(|_| split(&[pname("ex", "a"), "Dot".to_owned()]))),
        },
        Sweep {
            cited: "[142s] BLANK_NODE_LABEL first scalar",
            probe: Box::new(|c| format!("_:{c}z")),
            admits: Box::new(|c| terminals::is_pn_chars_u(c) || c.is_ascii_digit()),
            joined: Box::new(|c| split(&[bnode(&format!("{c}z"))])),
            // `WS` does not separate here: the production REQUIRES a first
            // scalar, so `_: z` names no blank node at all.
            separated: Some(Box::new(|_| Reading::Refused)),
        },
        Sweep {
            cited: "[142s] BLANK_NODE_LABEL continuation",
            probe: Box::new(|c| format!("_:a{c}z")),
            admits: Box::new(|c| terminals::is_pn_chars(c) || c == '.'),
            joined: Box::new(|c| split(&[bnode(&format!("a{c}z"))])),
            separated: Some(Box::new(|_| split(&[bnode("a"), word("z")]))),
        },
        Sweep {
            cited: "[142s] BLANK_NODE_LABEL, then the trailing-dot pushback",
            probe: Box::new(|c| format!("_:a{c}.")),
            admits: Box::new(terminals::is_pn_chars),
            joined: Box::new(|c| split(&[bnode(&format!("a{c}")), "Dot".to_owned()])),
            separated: Some(Box::new(|_| split(&[bnode("a"), "Dot".to_owned()]))),
        },
        Sweep {
            // `LANGTAG ::= '@' [a-zA-Z]+ ('-' [a-zA-Z0-9]+)*`. The digits are
            // NOT position-free: they are lawful only in a subtag after a `'-'`,
            // so applying one uniform class to the whole tag — the obvious
            // shortcut, and the one a name-class reflex reaches for — admits
            // `@e0n`, which is not a language tag.
            cited: "[145s] LANGTAG primary subtag, which is not a PN_CHARS run",
            probe: Box::new(|c| format!("\"x\"@e{c}n")),
            admits: Box::new(|c| c.is_ascii_alphabetic() || c == '-'),
            joined: Box::new(|c| split(&[string_lit("x"), lang_tag(&format!("e{c}n"))])),
            separated: Some(Box::new(|_| {
                split(&[string_lit("x"), lang_tag("e"), word("n")])
            })),
        },
        Sweep {
            cited: "[18t] IRIREF content, which excludes WS and admits U+00A0",
            probe: Box::new(|c| format!("<urn:ex:a{c}b>")),
            admits: Box::new(iriref_content),
            joined: Box::new(|c| split(&[iri(&format!("urn:ex:a{c}b"))])),
            separated: None,
        },
    ]
}

// ── The ShapeMap scanner ──────────────────────────────────────────────────────

/// A one-association map naming `<http://a.example/s>` against `START`.
fn sole_subject() -> Reading {
    sole_node(TermValue::iri("http://a.example/s"))
}

/// Every ShapeMap sweep.
///
/// The separator positions are swept three times over — after a term, after the
/// `'@'`, and after the `START` keyword — because `skip_ws` is called at each and
/// a scalar that leaked through any one of them would give a document a reading
/// the grammar does not give it.
///
/// `[18t] IRIREF`'s content class is **not** swept here, and the omission is a
/// layering fact rather than a gap: this entry point resolves every `IRIREF` as it
/// reads it, and RFC-3987 is narrower than the token production — `[` and `]` are
/// lawful `IRIREF` *content* and unlawful in an IRI outside an IP-literal, so a
/// positive result would be measuring the resolver, not the boundary. What the
/// scanner does decide is pinned by name in
/// [`the_shape_map_iriref_body_stops_at_the_scalars_the_production_excludes`].
fn shape_map_sweeps() -> Vec<Sweep> {
    vec![
        Sweep {
            cited: "PASSED TOKENS between a term and the '@'",
            probe: Box::new(|c| format!("<http://a.example/s>{c}@START")),
            admits: Box::new(is_ws),
            joined: Box::new(|_| sole_subject()),
            separated: None,
        },
        Sweep {
            cited: "PASSED TOKENS between the '@' and the shape label",
            probe: Box::new(|c| format!("<http://a.example/s>@{c}START")),
            admits: Box::new(is_ws),
            joined: Box::new(|_| sole_subject()),
            separated: None,
        },
        Sweep {
            cited: "PASSED TOKENS after the START keyword, at its follow boundary",
            probe: Box::new(|c| {
                format!("<http://a.example/s>@START{c},<http://a.example/t>@START")
            }),
            admits: Box::new(is_ws),
            joined: Box::new(|_| {
                associations(vec![
                    (
                        NodeSelector::Node(TermValue::iri("http://a.example/s")),
                        ShapeSelector::Start,
                    ),
                    (
                        NodeSelector::Node(TermValue::iri("http://a.example/t")),
                        ShapeSelector::Start,
                    ),
                ])
            }),
            separated: None,
        },
        Sweep {
            cited: "[142s] BLANK_NODE_LABEL first scalar",
            probe: Box::new(|c| format!("_:{c}z@START")),
            admits: Box::new(|c| terminals::is_pn_chars_u(c) || c.is_ascii_digit()),
            joined: Box::new(|c| sole_node(TermValue::blank(format!("{c}z")))),
            // The production REQUIRES a first scalar, so `_: z` names nothing.
            separated: Some(Box::new(|_| Reading::Refused)),
        },
        Sweep {
            cited: "[142s] BLANK_NODE_LABEL continuation",
            probe: Box::new(|c| format!("_:a{c}z@START")),
            admits: Box::new(|c| terminals::is_pn_chars(c) || c == '.'),
            joined: Box::new(|c| sole_node(TermValue::blank(format!("a{c}z")))),
            // A label ends at `WS`, and what follows it is then an unreadable
            // `z` where the grammar wants the '@'.
            separated: Some(Box::new(|_| Reading::Refused)),
        },
        Sweep {
            cited: "[14t] STRING_LITERAL2 content, which admits every control but CR and LF",
            probe: Box::new(|c| format!("\"a{c}b\"^^<http://example.org/d>@START")),
            admits: Box::new(string_literal_content),
            joined: Box::new(|c| {
                sole_node(TermValue::typed_literal(
                    format!("a{c}b"),
                    "http://example.org/d",
                ))
            }),
            separated: None,
        },
    ]
}

// ── The three derivations the rest of this rests on ───────────────────────────

/// The ranges of [`UNSHARED_PRODUCTIONS`], snapshotted, so an edit to this
/// crate's own transcription shows up as a reviewable range diff instead of a
/// green run.
///
/// This is the narrow half of the table oracle described at the top of the file,
/// and it is narrow deliberately. The sweeps cannot see a table change for the
/// classes whose expectation delegates to the implementation; for
/// `[14t] STRING_LITERAL2` they can, because the scanner and this file hold two
/// separate transcriptions — but that argument protects the *pair*, not either
/// copy alone. Editing both to agree on a wrong class would keep the sweeps
/// green, and this is what makes that edit visible.
///
/// Rendered from [`UNSHARED_PRODUCTIONS`] rather than from [`PRODUCTIONS`]: see
/// that constant for why the other seven are the sibling crate's to pin.
#[test]
fn the_locally_transcribed_ranges_are_the_snapshotted_ones() {
    let mut rendered = String::new();
    for (name, admits) in UNSHARED_PRODUCTIONS {
        writeln!(rendered, "{name}").expect("writing to a String cannot fail");
        for (lo, hi) in derived_ranges(admits) {
            if lo == hi {
                writeln!(rendered, "  U+{lo:04X}")
            } else {
                writeln!(rendered, "  U+{lo:04X}..U+{hi:04X}")
            }
            .expect("writing to a String cannot fail");
        }
    }
    insta::assert_snapshot!(rendered);
}

/// **Derivation one.** U+1680 OGHAM SPACE MARK is the *only* scalar that is both
/// Unicode `White_Space` and a name character.
///
/// The entire over-refusal guard rests on this: it is why "skip everything
/// `char::is_whitespace` admits" silently splits a lawful ShExC label in two, and
/// why "exclude the Unicode spaces from the name classes" would manufacture an
/// over-refusal. Intersected over all 1,114,112 scalars rather than asserted of
/// U+1680 alone — if the intersection ever grows a second member, that is a
/// finding, not an assertion to adjust.
#[test]
fn the_ogham_space_mark_is_the_only_whitespace_name_character() {
    for (name, admits) in NAME_PRODUCTIONS {
        let both: Vec<char> = all_scalars()
            .filter(|c| c.is_whitespace() && admits(*c))
            .collect();
        assert_eq!(
            both,
            vec!['\u{1680}'],
            "{name} meets Unicode `White_Space` in exactly one scalar"
        );
    }
    // And the converse, which is what makes the four-member `WS` skip safe: no
    // member of `WS` is a name character in any position.
    for c in all_scalars().filter(|c| is_ws(*c)) {
        assert!(
            !terminals::is_pn_chars(c),
            "U+{:04X} is `WS` and must not also be a name character",
            u32::from(c)
        );
    }
    // Both scanners act on it: it JOINS a name rather than separating two.
    assert_eq!(
        shexc("ex:a\u{1680}b"),
        split(&[pname("ex", "a\u{1680}b")]),
        "U+1680 is `PN_CHARS_BASE`, so this is ONE token"
    );
    assert_eq!(
        shape_map("_:a\u{1680}b@START"),
        sole_node(TermValue::blank("a\u{1680}b")),
        "U+1680 is `PN_CHARS_BASE`, so this is ONE label"
    );
}

/// **Derivation two.** U+FEFF is inside `PN_CHARS_BASE`, so a byte-order mark is
/// a lawful *name* character — not trivia, and not an offender.
///
/// Derived: the range the sweep reads off the predicate is located and its
/// endpoints are checked against the production `[#xFDF0-#xFFFD]`, rather than
/// the membership being asserted directly.
#[test]
fn the_byte_order_mark_is_a_lawful_name_character() {
    let ranges = derived_ranges(terminals::is_pn_chars_base);
    let containing = |c: char| {
        ranges
            .iter()
            .copied()
            .find(|(lo, hi)| (*lo..=*hi).contains(&u32::from(c)))
    };
    assert_eq!(
        containing('\u{feff}'),
        Some((0xFDF0, 0xFFFD)),
        "the BOM sits inside `PN_CHARS_BASE`'s [#xFDF0-#xFFFD]"
    );
    assert_eq!(shexc("ex:a\u{feff}b"), split(&[pname("ex", "a\u{feff}b")]));
    assert_eq!(
        shape_map("_:a\u{feff}b@START"),
        sole_node(TermValue::blank("a\u{feff}b"))
    );
}

/// **Derivation three.** U+200C ZERO WIDTH NON-JOINER and U+200D ZERO WIDTH
/// JOINER are inside `PN_CHARS_BASE`, so they are lawful and must never be
/// treated as offenders — while U+200B and U+200E, one scalar to either side,
/// are not.
///
/// The pair is what makes "invisible, therefore suspicious" the wrong rule: being
/// an offender is a property of the **position**, not of the character.
#[test]
fn the_zero_width_joiners_are_lawful_and_their_neighbours_are_not() {
    let ranges = derived_ranges(terminals::is_pn_chars_base);
    let containing = |c: char| {
        ranges
            .iter()
            .copied()
            .find(|(lo, hi)| (*lo..=*hi).contains(&u32::from(c)))
    };
    assert_eq!(containing('\u{200c}'), Some((0x200C, 0x200D)));
    assert_eq!(containing('\u{200d}'), Some((0x200C, 0x200D)));
    assert_eq!(containing('\u{200b}'), None, "ZERO WIDTH SPACE is a hole");
    assert_eq!(containing('\u{200e}'), None, "LEFT-TO-RIGHT MARK is a hole");

    for c in ['\u{200c}', '\u{200d}'] {
        assert_eq!(
            shexc(&format!("ex:a{c}b")),
            split(&[pname("ex", &format!("a{c}b"))]),
            "U+{:04X} is `PN_CHARS_BASE` and joins the name",
            u32::from(c)
        );
    }
    for c in ['\u{200b}', '\u{200e}'] {
        assert_ne!(
            shexc(&format!("ex:a{c}b")),
            split(&[pname("ex", &format!("a{c}b"))]),
            "U+{:04X} is in the hole and must not join the name",
            u32::from(c)
        );
    }
}

// ── The sweeps ────────────────────────────────────────────────────────────────

/// The gated corpus is not empty and not accidentally narrow.
#[test]
fn the_corpus_covers_every_boundary_and_all_of_unicode_whitespace() {
    let scalars = probe_scalars();
    // The four-per-range neighbourhoods overlap heavily once name classes that
    // extend one another are unioned, so the DISTINCT corpus is far smaller than
    // four times the range count. The floor guards against collapse.
    assert!(
        scalars.len() >= 100,
        "the boundary corpus collapsed to {} scalars",
        scalars.len()
    );
    for c in all_scalars().filter(|c| c.is_whitespace()) {
        assert!(scalars.contains(&c), "U+{:04X} missing", u32::from(c));
    }
    for (_, admits) in PRODUCTIONS {
        for (lo, hi) in derived_ranges(admits) {
            for cp in [lo.wrapping_sub(1), lo, hi, hi + 1] {
                if let Some(c) = char::from_u32(cp) {
                    assert!(scalars.contains(&c), "U+{cp:04X} missing");
                }
            }
        }
    }
}

/// The ShExC lexer splits exactly where the productions say.
#[test]
fn the_shexc_scanner_splits_where_the_productions_say() {
    let scalars = probe_scalars();
    let sweeps = shexc_sweeps();
    let cases = run(shexc, &sweeps, &scalars);
    println!(
        "ShExC: {} scalars x {} sweeps = {cases} probes",
        scalars.len(),
        sweeps.len()
    );
}

/// The ShapeMap scanner — a different machine, with no token type — reads exactly
/// where the productions say.
#[test]
fn the_shape_map_scanner_reads_where_the_productions_say() {
    let scalars = probe_scalars();
    let sweeps = shape_map_sweeps();
    let cases = run(shape_map, &sweeps, &scalars);
    println!(
        "ShapeMap: {} scalars x {} sweeps = {cases} probes",
        scalars.len(),
        sweeps.len()
    );
}

/// Being an offender is a property of the **position**, not of the character.
///
/// Named rather than swept because the point is the pairing: each scalar appears
/// once where the grammar admits it and once where it does not, so neither half
/// can be read as blanket strictness or blanket leniency.
#[test]
fn offence_is_a_property_of_the_position() {
    // Tail positives a head-only predicate would wrongly refuse.
    assert_eq!(shexc("_:a\u{b7}"), split(&[bnode("a\u{b7}")]));
    assert_eq!(shexc("_:a\u{300}"), split(&[bnode("a\u{300}")]));
    assert_eq!(shexc("_:a\u{203f}b"), split(&[bnode("a\u{203f}b")]));
    assert_eq!(shexc("ex:a\u{b7}b"), split(&[pname("ex", "a\u{b7}b")]));
    assert_eq!(
        shape_map("_:cafe\u{301}@START"),
        sole_node(TermValue::blank("cafe\u{301}"))
    );

    // The same scalars in head position, where `BLANK_NODE_LABEL` names only
    // `PN_CHARS_U | [0-9]` and a tail-only predicate would wrongly accept them.
    for head in ['\u{b7}', '\u{300}', '\u{203f}', '-'] {
        assert_eq!(
            shexc(&format!("_:{head}z")),
            Reading::Refused,
            "U+{:04X} may continue a label and may not begin one",
            u32::from(head)
        );
        assert_eq!(
            shape_map(&format!("_:{head}z@START")),
            Reading::Refused,
            "U+{:04X} may continue a label and may not begin one",
            u32::from(head)
        );
    }

    // And the lawful heads, so none of the above is blanket strictness.
    assert_eq!(shexc("_:az"), split(&[bnode("az")]));
    assert_eq!(shexc("_:_z"), split(&[bnode("_z")]));
    assert_eq!(shexc("_:0z"), split(&[bnode("0z")]));
    assert_eq!(
        shape_map("_:0z@START"),
        sole_node(TermValue::blank("0z")),
        "a label may begin with a digit"
    );
}

/// The `IRIREF` body in a shape map stops exactly at the scalars the production
/// excludes — and at no others.
///
/// Named rather than swept because the positive direction runs into the resolver
/// (see [`shape_map_sweeps`]), so the two halves need different corpora. Both
/// halves are here, in one vector, so the refusal is never asserted alone:
///
/// * `#x00-#x20` and the nine delimiters end the body. `#x20` SPACE is the one a
///   Unicode control property gets backwards — it is not a control, so it used to
///   be absorbed, and `<urn:ex:a b>` named an IRI with a space in it.
/// * Everything above `#x20` that is not a delimiter is content, U+00A0 and the
///   rest of the Unicode whitespace included. Narrowing the class to "no
///   whitespace" would refuse IRIs this workspace's own writers emit.
#[test]
fn the_shape_map_iriref_body_stops_at_the_scalars_the_production_excludes() {
    for c in ['\u{a0}', '\u{1680}', '\u{2000}', '\u{200b}', '\u{3000}'] {
        assert_eq!(
            shape_map(&format!("<urn:ex:a{c}b>@START")),
            sole_node(TermValue::iri(format!("urn:ex:a{c}b"))),
            "U+{:04X} is above #x20 and is not a delimiter, so it is IRI content",
            u32::from(c)
        );
    }
    for c in [
        ' ', '\t', '\n', '\r', '\u{0b}', '\u{0c}', '\u{1f}', // the #x00-#x20 range
        '<', '"', '{', '}', '|', '^', '`', '\\', // eight of the nine delimiters
    ] {
        assert_eq!(
            shape_map(&format!("<urn:ex:a{c}b>@START")),
            Reading::Refused,
            "U+{:04X} is excluded from IRIREF content and must end the body",
            u32::from(c)
        );
    }
    // The ninth delimiter is `'>'`, which does not "end the body" so much as CLOSE
    // it — so it gets its own reading rather than a refusal.
    assert_eq!(
        shape_map("<urn:ex:a>@START"),
        sole_node(TermValue::iri("urn:ex:a"))
    );
}

/// A raw control is lawful *content* inside a string literal, and a line
/// terminator is not.
///
/// The mirror half of the `IRIREF` vector above, and the other direction of the
/// same wrong question: `char::is_control` refused a TAB the production admits.
///
/// Written with an explicit datatype rather than `@START`, because a bare `'@'`
/// after a literal is a `LANGTAG` and would bind the shape label into the term.
#[test]
fn a_shape_map_literal_admits_a_raw_tab_and_refuses_a_raw_newline() {
    let typed = |lexical: &str| format!("\"{lexical}\"^^<http://example.org/d>@START");
    assert_eq!(
        shape_map(&typed("a\tb")),
        sole_node(TermValue::typed_literal("a\tb", "http://example.org/d")),
        "`[14t] STRING_LITERAL2` excludes only #x22, #x5C, #xA and #xD"
    );
    for c in ['\n', '\r'] {
        assert_eq!(
            shape_map(&typed(&format!("a{c}b"))),
            Reading::Refused,
            "U+{:04X} is a line terminator and leaves the production",
            u32::from(c)
        );
    }
}

/// The dot pushback keeps an *internal* dot and hands back only a trailing run.
///
/// The neighbour to every trailing-dot refusal in the sweeps: narrowing the
/// pushback to "no dots in names" would refuse `ex:a.b` and `_:a.b`, which the
/// production admits. The shape map has no production that admits a `.` after a
/// label at all, so there the trailing dot is a refusal rather than a pushback —
/// and the internal one still parses.
#[test]
fn an_internal_dot_stays_in_the_name_and_a_trailing_run_does_not() {
    assert_eq!(
        shexc("ex:a.b."),
        split(&[pname("ex", "a.b"), "Dot".to_owned()])
    );
    assert_eq!(shexc("_:a.b."), split(&[bnode("a.b"), "Dot".to_owned()]));
    assert_eq!(
        shape_map("_:a.b@START"),
        sole_node(TermValue::blank("a.b")),
        "an internal dot is `BLANK_NODE_LABEL` content"
    );
    assert_eq!(
        shape_map("_:a.@START"),
        Reading::Refused,
        "a label may not end on a dot, and nothing else may consume it"
    );
}

// ── The full lane, for anyone who wants it ────────────────────────────────────

/// Every sweep over every Unicode scalar, ShExC.
///
/// Kept runnable and kept passing: the boundary corpus is an argument that a
/// range table can only be wrong at an edge, and this lane is the thing that
/// argument stands in for. Off the gate because it is three orders of magnitude
/// more probes for a claim the gated corpus already makes.
#[test]
#[ignore = "the whole 1,114,112-scalar space; the boundary corpus is the gated lane"]
fn the_full_shexc_scanner_sweep() {
    let scalars: Vec<char> = all_scalars().collect();
    run(shexc, &shexc_sweeps(), &scalars);
}

/// Every sweep over every Unicode scalar, ShapeMap.
#[test]
#[ignore = "the whole 1,114,112-scalar space; the boundary corpus is the gated lane"]
fn the_full_shape_map_scanner_sweep() {
    let scalars: Vec<char> = all_scalars().collect();
    run(shape_map, &shape_map_sweeps(), &scalars);
}
