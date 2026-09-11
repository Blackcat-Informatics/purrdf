// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Where the token actually stops: a boundary-exhaustive sweep of the two scan
//! entry points this crate publishes.
//!
//! # Why a predicate test cannot reach this
//!
//! `purrdf_iri::terminals` proves its range tables against an independent
//! transcription on all 1,114,112 scalars, so *membership* is settled. A scanner
//! does not ask membership. It asks **where a token ends**, and it answers that
//! by composing a membership test with everything around it:
//!
//! * **maximal munch** — the name scan runs before the whitespace skip is ever
//!   consulted, so a class one scalar too wide moves the boundary rather than
//!   widening the accepted language;
//! * **the `?`/`$` disambiguation peek** — `?` is the variable sigil when a
//!   `VARNAME` first-scalar follows and the path `zero-or-one` operator when one
//!   does not, so the peek must ask the START class, not the continue class;
//! * **the separate `ANON` scan** — `'[' WS* ']'` is one terminal, scanned by a
//!   loop that is deliberately blind to comments and to every non-`WS` scalar;
//! * **the trailing-dot pushback** — a `PN_LOCAL` or `BLANK_NODE_LABEL` scan
//!   over-consumes a dot run and hands it back, because a trailing `.` is the
//!   statement terminator.
//!
//! Every one of those is correct-by-parts and wrong-by-composition in a way no
//! membership test can see. So this file feeds probes through the real scanners
//! and asserts the resulting token split.
//!
//! # Boundary-exhaustive, which is where completeness is possible
//!
//! A `format!`-plus-tokenize call for all 1,114,112 scalars, times several
//! productions, times each scanner, is minutes on every gate run. It is also
//! unnecessary: a range table can only be wrong **at a boundary**, so the gated
//! corpus is `lo - 1`, `lo`, `hi`, `hi + 1` for every range of every production
//! — derived from the predicates themselves, never hand-listed — plus
//!
//! * the whole Unicode `White_Space` property, swept out of
//!   [`char::is_whitespace`] rather than enumerated (a hand-picked list is how
//!   U+205F and U+2029 escape review),
//! * the invisible format characters, which review cannot see at all,
//! * and `[18t] IRIREF`'s own nine delimiters.
//!
//! The full lane is kept, `#[ignore]`d, for anyone who wants it.
//!
//! # What is asserted, and what is deliberately not
//!
//! Per probe the claim is three-valued and reads off the cited production:
//!
//! 1. the production admits the candidate at that position → the probe is ONE
//!    token and its payload holds the candidate;
//! 2. the candidate is `WS` and the position has a separator reading → the probe
//!    splits into exactly the tokens named;
//! 3. otherwise → the probe is **not** that one token, i.e. the candidate was not
//!    silently absorbed.
//!
//! Case 3 never pins an error arm or an operator's token count: `?a?b` is two
//! variables and `?a-?b` is three tokens, and both are correct. Pinning either
//! would pin an accident of the split rather than the property.
//!
//! # This does not replace the named vectors
//!
//! `tests/name_class_boundaries.rs` stays. A failure here prints
//! "U+2028 disagreed", which says nothing about *why* the boundary sits there;
//! the named vectors carry that reasoning, and the sweep carries the coverage.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use pretty_assertions::assert_eq;
use purrdf_iri::terminals;
use purrdf_sparql_algebra::lexer::{tokenize, tokenize_turtle};

// ── The scalar corpus, derived ────────────────────────────────────────────────

/// Every Unicode scalar value, in order.
fn all_scalars() -> impl Iterator<Item = char> {
    (0..=0x0010_FFFF_u32).filter_map(char::from_u32)
}

/// `WS ::= #x20 | #x9 | #xD | #xA` as a scalar test.
///
/// The shared production is byte-shaped because every member is ASCII and a byte
/// test is therefore exact over UTF-8; this file holds decoded scalars, so it
/// narrows first. `u8::try_from` fails above U+00FF and every Latin-1 scalar it
/// does yield is outside `WS`, so nothing non-ASCII can alias a member.
fn is_ws(c: char) -> bool {
    u8::try_from(c).is_ok_and(terminals::is_ws)
}

/// A production, named as the grammar names it, paired with the predicate that
/// decides membership in it.
type Production = (&'static str, fn(char) -> bool);

/// The productions whose ranges bound this sweep, each paired with the predicate
/// that decides membership. The corpus is read off these, so adding a production
/// here widens the sweep automatically.
const PRODUCTIONS: [Production; 7] = [
    ("WS", is_ws),
    ("PN_CHARS_BASE", terminals::is_pn_chars_base),
    ("PN_CHARS_U", terminals::is_pn_chars_u),
    ("PN_CHARS", terminals::is_pn_chars),
    ("VARNAME first scalar", terminals::is_varname_start),
    ("VARNAME later scalar", terminals::is_varname_continue),
    ("IRIREF content", iriref_content),
];

/// The subset of [`PRODUCTIONS`] that decides where a NAME stops. `WS` is the
/// class a scanner skips and `IRIREF content` is a delimited body, so neither is
/// a name class and neither belongs in the whitespace intersection below.
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
/// The surrogate gap is treated as a non-member run, so a production that spanned
/// it would show here as two ranges. None does, which is itself a fact the
/// snapshot records.
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

/// The scalars a range table can be wrong at: one below each range, both
/// endpoints, and one above.
fn range_boundaries() -> BTreeSet<char> {
    let mut out = BTreeSet::new();
    for (_, admits) in PRODUCTIONS {
        for (lo, hi) in derived_ranges(admits) {
            for cp in [lo.wrapping_sub(1), lo, hi, hi + 1] {
                out.extend(char::from_u32(cp));
            }
        }
    }
    out
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
    let mut out = range_boundaries();
    out.extend(all_scalars().filter(|c| c.is_whitespace()));
    out.extend(INVISIBLES);
    out.extend(IRIREF_DELIMITERS);
    out.into_iter().collect()
}

// ── What a scanner made of a probe ────────────────────────────────────────────

/// The reading a scan entry point gave a probe.
#[derive(Clone, PartialEq, Eq, Debug)]
enum Reading {
    /// The probe has no reading in this grammar.
    Refused,
    /// The tokens the probe split into, rendered.
    Tokens(Vec<String>),
}

/// Tokenize as SPARQL — [`tokenize`], the default [`LexerOptions`].
///
/// [`LexerOptions`]: purrdf_sparql_algebra::lexer::LexerOptions
fn sparql(probe: &str) -> Reading {
    tokenize(probe).map_or(Reading::Refused, |tokens| {
        Reading::Tokens(tokens.iter().map(|s| format!("{:?}", s.token)).collect())
    })
}

/// Tokenize as Turtle/TriG — [`tokenize_turtle`], which differs from
/// [`sparql`] only by `pn_local_allows_slash`.
fn turtle(probe: &str) -> Reading {
    tokenize_turtle(probe).map_or(Reading::Refused, |tokens| {
        Reading::Tokens(tokens.iter().map(|s| format!("{:?}", s.token)).collect())
    })
}

/// One rendered token, as the derived `Debug` spells it.
fn variable(name: &str) -> String {
    format!("Variable({name:?})")
}

/// A `Word` token (keyword, `a`, or a boolean), rendered.
fn word(text: &str) -> String {
    format!("Word({text:?})")
}

/// A `PrefixedName` token, rendered.
fn prefixed(prefix: &str, local: &str) -> String {
    format!("PrefixedName({prefix:?}, {local:?})")
}

/// A `BlankNodeLabel` token, rendered.
fn blank(label: &str) -> String {
    format!("BlankNodeLabel({label:?})")
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

/// A reading of exactly the tokens given.
fn split(tokens: &[String]) -> Reading {
    Reading::Tokens(tokens.to_vec())
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
    /// The one-token reading an admitted candidate must produce.
    joined: Box<dyn Fn(char) -> Reading>,
    /// The reading a `WS` candidate must produce, where the position has one.
    /// `None` where `WS` is what the production itself admits.
    separated: Option<Box<dyn Fn(char) -> Reading>>,
}

/// `[169s] PN_LOCAL`'s continuation class as SPARQL scans it: `PN_CHARS`, the
/// `':'` the production names, `'.'` where a further name character follows, and
/// the `'%'` that opens `[171s] PERCENT`.
fn pn_local_continue(c: char) -> bool {
    terminals::is_pn_chars(c) || c == ':' || c == '.' || c == '%'
}

/// The same class as Turtle scans it: a bare `/` is admitted there, because
/// Turtle has no `/` operator to be ambiguous with.
fn pn_local_continue_turtle(c: char) -> bool {
    pn_local_continue(c) || c == '/'
}

/// `[18t] IRIREF`'s content class: everything above `#x20` that is not one of the
/// nine delimiters. Non-ASCII is never forbidden raw, U+00A0 included.
fn iriref_content(c: char) -> bool {
    u32::from(c) > 0x20 && !IRIREF_DELIMITERS.contains(&c)
}

/// The sweeps both entry points share — every production Turtle and SPARQL spell
/// the same way. `pn_local` is the one class that differs, so it is a parameter.
fn shared_sweeps(pn_local: fn(char) -> bool) -> Vec<Sweep> {
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
            admits: Box::new(pn_local),
            joined: Box::new(|c| split(&[prefixed("ex", &format!("a{c}z"))])),
            separated: Some(Box::new(|_| split(&[prefixed("ex", "a"), word("z")]))),
        },
        Sweep {
            cited: "[169s] PN_LOCAL, then the trailing-dot pushback",
            probe: Box::new(|c| format!("ex:a{c}.")),
            // A dot run at the end is the statement terminator and is handed
            // back, so `'.'` itself is excluded here: `ex:a..` is not the local
            // name `a.` — it is `a` followed by two dots.
            admits: Box::new(move |c| pn_local(c) && c != '.'),
            joined: Box::new(|c| split(&[prefixed("ex", &format!("a{c}")), "Dot".to_owned()])),
            separated: Some(Box::new(|_| {
                split(&[prefixed("ex", "a"), "Dot".to_owned()])
            })),
        },
        Sweep {
            cited: "[142s] BLANK_NODE_LABEL continuation",
            probe: Box::new(|c| format!("_:a{c}z")),
            admits: Box::new(|c| terminals::is_pn_chars(c) || c == '.'),
            joined: Box::new(|c| split(&[blank(&format!("a{c}z"))])),
            separated: Some(Box::new(|_| split(&[blank("a"), word("z")]))),
        },
        Sweep {
            cited: "[142s] BLANK_NODE_LABEL, then the trailing-dot pushback",
            probe: Box::new(|c| format!("_:a{c}.")),
            admits: Box::new(terminals::is_pn_chars),
            joined: Box::new(|c| split(&[blank(&format!("a{c}")), "Dot".to_owned()])),
            separated: Some(Box::new(|_| split(&[blank("a"), "Dot".to_owned()]))),
        },
        Sweep {
            cited: "[163s] ANON ::= '[' WS* ']', the separate scan",
            probe: Box::new(|c| format!("[{c}]")),
            // `WS` is what this production admits, so there is no separator
            // reading: a scalar that is not `WS` must fail to CLOSE the `ANON`.
            admits: Box::new(is_ws),
            joined: Box::new(|_| split(&["Anon".to_owned()])),
            separated: None,
        },
        Sweep {
            // `LANG_DIR ::= '@' [a-zA-Z]+ ('-' [a-zA-Z0-9]+)* ('--' [a-zA-Z]+)?`.
            // The lexer's job here is the MUNCH — it takes the maximal
            // `[a-zA-Z0-9-]` run and hands it to the parser, which holds the
            // position-dependence (digits only in a subtag after a `'-'`). That
            // split is deliberate: `@e0n` re-split at the token level would
            // silently become `@e` followed by the name `0n`, where deferring
            // the check names the malformed tag instead. The completion of the
            // claim is asserted at the parser in
            // `the_lexer_munches_a_language_tag_and_the_parser_validates_it`.
            cited: "[145] LANG_DIR munch run, which is not a PN_CHARS run",
            probe: Box::new(|c| format!("\"x\"@e{c}n")),
            admits: Box::new(|c| c.is_ascii_alphanumeric() || c == '-'),
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

/// The two `VARNAME` sweeps, SPARQL-only: Turtle has no variables.
///
/// Head and tail are swept **separately** because the production is
/// position-dependent, and both directions of getting that wrong are silent. A
/// head-only class refuses the lawful `?a\u{300}`; a tail-only class accepts the
/// unlawful `?\u{300}` and turns the path operator into a variable sigil.
fn varname_sweeps() -> Vec<Sweep> {
    vec![
        Sweep {
            cited: "[166s] VARNAME first scalar, through the `?` disambiguation peek",
            probe: Box::new(|c| format!("?{c}z")),
            admits: Box::new(terminals::is_varname_start),
            joined: Box::new(|c| split(&[variable(&format!("{c}z"))])),
            separated: Some(Box::new(|_| split(&["Question".to_owned(), word("z")]))),
        },
        Sweep {
            cited: "[166s] VARNAME later scalar",
            probe: Box::new(|c| format!("?a{c}z")),
            admits: Box::new(terminals::is_varname_continue),
            joined: Box::new(|c| split(&[variable(&format!("a{c}z"))])),
            separated: Some(Box::new(|_| split(&[variable("a"), word("z")]))),
        },
    ]
}

/// Every SPARQL sweep.
fn sparql_sweeps() -> Vec<Sweep> {
    let mut sweeps = varname_sweeps();
    sweeps.extend(shared_sweeps(pn_local_continue));
    sweeps
}

/// Every Turtle sweep. `VARNAME` is absent because Turtle has no `Var`.
fn turtle_sweeps() -> Vec<Sweep> {
    shared_sweeps(pn_local_continue_turtle)
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
                    "U+{:04X} is admitted by {cited}, so {probe:?} is ONE token",
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

// ── The three derivations the rest of this rests on ───────────────────────────

/// The derived ranges, snapshotted, so a predicate edit shows up as a reviewable
/// range diff instead of a green run.
#[test]
fn the_derived_ranges_are_the_snapshotted_ones() {
    let mut rendered = String::new();
    for (name, admits) in PRODUCTIONS {
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
/// `char::is_whitespace` admits" silently splits a lawful name in two, and why
/// "exclude the Unicode spaces from the name classes" would manufacture an
/// over-refusal. Intersected over all 1,114,112 scalars rather than asserted of
/// U+1680 alone — if the intersection ever grows a second member, that is a
/// finding, not an assertion to adjust.
#[test]
fn the_ogham_space_mark_is_the_only_whitespace_name_character() {
    let intersect = |admits: fn(char) -> bool| -> Vec<char> {
        all_scalars()
            .filter(|c| c.is_whitespace() && admits(*c))
            .collect()
    };
    for (name, admits) in NAME_PRODUCTIONS {
        assert_eq!(
            intersect(admits),
            vec!['\u{1680}'],
            "{name} meets Unicode `White_Space` in exactly one scalar"
        );
    }
    // And the converse, which is what makes the four-member `WS` skip safe: no
    // member of `WS` is a name character in any position.
    for c in all_scalars().filter(|c| is_ws(*c)) {
        assert!(
            !terminals::is_pn_chars(c) && !terminals::is_varname_continue(c),
            "U+{:04X} is `WS` and must not also be a name character",
            u32::from(c)
        );
    }
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
    // The behaviour that follows from it: it joins a name rather than ending one,
    // in both scan entry points.
    let joined = split(&[prefixed("ex", "a\u{feff}b")]);
    assert_eq!(sparql("ex:a\u{feff}b"), joined);
    assert_eq!(turtle("ex:a\u{feff}b"), joined);
    assert_eq!(sparql("?a\u{feff}b"), split(&[variable("a\u{feff}b")]));
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
            sparql(&format!("ex:a{c}b")),
            split(&[prefixed("ex", &format!("a{c}b"))]),
            "U+{:04X} is `PN_CHARS_BASE` and joins the name",
            u32::from(c)
        );
    }
    for c in ['\u{200b}', '\u{200e}'] {
        assert_ne!(
            sparql(&format!("ex:a{c}b")),
            split(&[prefixed("ex", &format!("a{c}b"))]),
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
    // The four-per-range neighbourhoods overlap heavily once six name classes
    // that extend one another are unioned, so the DISTINCT corpus is far smaller
    // than four times the range count. The floor guards against it collapsing,
    // not against it shrinking by a scalar.
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

/// `tokenize` — the SPARQL entry point — splits exactly where the productions say.
#[test]
fn the_sparql_scanner_splits_where_the_productions_say() {
    let scalars = probe_scalars();
    let sweeps = sparql_sweeps();
    let cases = run(sparql, &sweeps, &scalars);
    println!(
        "SPARQL: {} scalars x {} sweeps = {cases} probes",
        scalars.len(),
        sweeps.len()
    );
}

/// `tokenize_turtle` — a genuinely different scanner, differing by
/// `pn_local_allows_slash` — splits exactly where the productions say.
#[test]
fn the_turtle_scanner_splits_where_the_productions_say() {
    let scalars = probe_scalars();
    let sweeps = turtle_sweeps();
    let cases = run(turtle, &sweeps, &scalars);
    println!(
        "Turtle: {} scalars x {} sweeps = {cases} probes",
        scalars.len(),
        sweeps.len()
    );
}

/// The two entry points differ in the `/` and in nothing else.
///
/// Asserted over the whole corpus rather than by inspection of the one flag: the
/// `LexerOptions` seam is the place a second leniency would be added, and a
/// second leniency that changed Turtle's reading of anything else would show here.
#[test]
fn the_two_scan_entry_points_differ_only_in_the_slash() {
    let sweeps = shared_sweeps(pn_local_continue);
    for &c in &probe_scalars() {
        if c == '/' {
            continue;
        }
        for sweep in &sweeps {
            let probe = (sweep.probe)(c);
            assert_eq!(
                sparql(&probe),
                turtle(&probe),
                "U+{:04X}: the two entry points must agree on {probe:?}",
                u32::from(c)
            );
        }
    }

    let disagree: BTreeSet<&str> = sweeps
        .iter()
        .filter(|sweep| {
            let probe = (sweep.probe)('/');
            sparql(&probe) != turtle(&probe)
        })
        .map(|sweep| sweep.cited)
        .collect();
    assert_eq!(
        disagree,
        BTreeSet::from([
            "[169s] PN_LOCAL continuation",
            "[169s] PN_LOCAL, then the trailing-dot pushback",
        ]),
        "a `/` may move the boundary in PN_LOCAL position and nowhere else"
    );

    assert_eq!(
        sparql("ex:a/z"),
        split(&[prefixed("ex", "a"), "Slash".to_owned(), word("z")]),
        "in SPARQL `/` is the property-path sequence operator"
    );
    assert_eq!(
        turtle("ex:a/z"),
        split(&[prefixed("ex", "a/z")]),
        "in Turtle term position `/` is a PN_LOCAL character"
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
    assert_eq!(sparql("?a\u{b7}"), split(&[variable("a\u{b7}")]));
    assert_eq!(sparql("?a\u{300}"), split(&[variable("a\u{300}")]));
    assert_eq!(sparql("?a\u{203f}b"), split(&[variable("a\u{203f}b")]));
    assert_eq!(sparql("ex:a\u{b7}b"), split(&[prefixed("ex", "a\u{b7}b")]));

    // The same scalars in head position, which a tail-only predicate would
    // wrongly accept. `?` is then the path `zero-or-one` operator and the scalar
    // after it begins no terminal at all.
    assert_eq!(sparql("?\u{b7}"), Reading::Refused);
    assert_eq!(sparql("?\u{300}"), Reading::Refused);
    assert_eq!(sparql("?\u{203f}"), Reading::Refused);

    // And the lawful head, so none of the above is blanket strictness.
    assert_eq!(sparql("?a"), split(&[variable("a")]));
    assert_eq!(sparql("?_"), split(&[variable("_")]));
    assert_eq!(sparql("?0"), split(&[variable("0")]));
}

/// The dot pushback keeps an *internal* dot and hands back only a trailing run.
///
/// The neighbour to every trailing-dot refusal in the sweeps: narrowing the
/// pushback to "no dots in names" would refuse `ex:a.b` and `_:a.b`, which the
/// production admits.
#[test]
fn an_internal_dot_stays_in_the_name_and_a_trailing_run_does_not() {
    assert_eq!(
        sparql("ex:a.b."),
        split(&[prefixed("ex", "a.b"), "Dot".to_owned()])
    );
    assert_eq!(sparql("_:a.b."), split(&[blank("a.b"), "Dot".to_owned()]));
    assert_eq!(
        sparql("ex:a..."),
        split(&[
            prefixed("ex", "a"),
            "Dot".to_owned(),
            "Dot".to_owned(),
            "Dot".to_owned(),
        ])
    );
}

/// The language-tag claim the token-level sweep can only half make.
///
/// The lexer takes the maximal `[a-zA-Z0-9-]` run; `LANG_DIR`'s digits are lawful
/// only in a subtag after a `'-'`, and that position-dependence is the parser's.
/// Asserting only the munch would leave "`@0` is accepted" on the record with
/// nothing to say where it is refused; asserting only the parser would let the
/// munch narrow silently and re-split `@en-GB` into three tokens.
#[test]
fn the_lexer_munches_a_language_tag_and_the_parser_validates_it() {
    use purrdf_sparql_algebra::SparqlParser;

    // The munch: one token, digits and all.
    assert_eq!(
        sparql("\"x\"@e0n"),
        split(&[string_lit("x"), lang_tag("e0n")])
    );

    let parser = SparqlParser::new();
    let query = |tag: &str| format!("SELECT ?s WHERE {{ ?s <urn:ex:p> \"x\"@{tag} }}");
    for tag in [
        "en",
        "en-GB",
        "zh-Hans",
        "de-CH-1901",
        "en--ltr",
        "en-GB--rtl",
    ] {
        assert!(
            parser.parse_query(&query(tag)).is_ok(),
            "@{tag} is a well-formed LANG_DIR"
        );
    }
    for tag in ["e0n", "0en", "en-", "-en", "en--up"] {
        assert!(
            parser.parse_query(&query(tag)).is_err(),
            "@{tag} is not a well-formed LANG_DIR"
        );
    }
}

// ── The full lane, for anyone who wants it ────────────────────────────────────

/// Every sweep over every Unicode scalar, SPARQL.
///
/// Kept runnable and kept passing: the boundary corpus is an argument that a
/// range table can only be wrong at an edge, and this lane is the thing that
/// argument stands in for. Off the gate because it is three orders of magnitude
/// more probes for a claim the gated corpus already makes.
#[test]
#[ignore = "the whole 1,114,112-scalar space; the boundary corpus is the gated lane"]
fn the_full_sparql_scanner_sweep() {
    let scalars: Vec<char> = all_scalars().collect();
    run(sparql, &sparql_sweeps(), &scalars);
}

/// Every sweep over every Unicode scalar, Turtle.
#[test]
#[ignore = "the whole 1,114,112-scalar space; the boundary corpus is the gated lane"]
fn the_full_turtle_scanner_sweep() {
    let scalars: Vec<char> = all_scalars().collect();
    run(turtle, &turtle_sweeps(), &scalars);
}
