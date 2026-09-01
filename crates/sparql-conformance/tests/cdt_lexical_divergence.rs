// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The executed mitigation for PurRDF's two documented supersets of the SEP-0009
//! lexical space.
//!
//! # The divergence this measures
//!
//! SEP-0009's `Element` production admits neither an RDF 1.2 triple term
//! (`<<( s p o )>>`) nor a directional language-tagged literal (`"x"@en--ltr` /
//! `--rtl`). `purrdf-cdt` admits both, on the reasoning that refusing an RDF 1.2
//! term type is not an admissible outcome for this toolkit, and that a form
//! outside the spec's own lexical space is emitted only when such a term is
//! actually present — i.e. only for values the spec cannot express at all.
//!
//! That reasoning is an ARGUMENT. This file is the evidence, and it is the only
//! thing standing between "a deliberate, bounded extension" and "an
//! undocumented incompatibility": a conformant SEP-0009 reader handed one of
//! PurRDF's extended literals will call it ill-formed, so the claim that has to
//! hold is that no vector any upstream suite ships ever needs, contains, or
//! rejects one.
//!
//! # What the scan actually does
//!
//! It is not a text grep for `<<(`. It TOKENIZES every corpus file with the
//! production lexer — [`purrdf_sparql_algebra::lexer::tokenize`] for `.rq`,
//! [`tokenize_turtle`](purrdf_sparql_algebra::lexer::tokenize_turtle) for the RDF
//! syntaxes — resolves each file's own prefix bindings, recovers every literal
//! whose datatype is `cdt:List` or `cdt:Map`, and GRADES its lexical form with
//! [`purrdf_cdt::lexical_space`], the same function the crate uses to report its
//! own conformance position. A form that needs either superset grades
//! [`LexicalSpace::PurrdfSuperset`]; one the published grammar admits grades
//! [`LexicalSpace::Sep0009`].
//!
//! # Why the pinned counts are part of the assertion
//!
//! "Scanned every vector, found no divergence" is worthless if the recovery
//! silently found nothing to scan. [`corpus_cdt_literals_are_all_inside_sep0009`]
//! therefore pins the number of files visited, the number of CDT literals
//! recovered, and the number that PARSED — as exact equalities, never `>=`. A
//! recovery that breaks, a corpus that is re-synced, or a lexer change that stops
//! producing `^^` sequences turns this test RED instead of quietly turning it
//! vacuous.
//!
//! # Why the positive controls are part of the assertion
//!
//! A grader that answered `Sep0009` unconditionally would pass the scan above
//! perfectly. [`the_grader_fires_on_each_extended_production`] runs one literal
//! per widened form through the SAME recovery-and-grade path and requires
//! `PurrdfSuperset`, so the scan is known to be capable of failing.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use purrdf_cdt::{LexicalSpace, lexical_space, parse_cdt_by_iri};
use purrdf_sparql_algebra::lexer::{Token, tokenize, tokenize_turtle};

/// The two SEP-0009 datatype IRIs, spelled exactly as the spec fixes them.
const CDT_LIST: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List";
const CDT_MAP: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/Map";

/// One recovered composite literal: where it came from, and what it said.
#[derive(Debug)]
struct Recovered {
    /// The corpus file, relative to the workspace root.
    file: PathBuf,
    /// The literal's lexical form, unescaped by the lexer exactly as the parser
    /// would see it.
    lexical: String,
    /// `cdt:List` or `cdt:Map`.
    datatype: String,
}

/// Every directory the scan covers. These are the vendored/first-party corpora
/// the workspace ships; a divergence that no vector in ANY of them exercises is
/// a divergence no upstream suite can observe.
const CORPUS_ROOTS: &[&str] = &[
    "vectors",
    "crates/sparql-conformance/suite",
    "crates/sparql-conformance/corpus",
    "crates/sparql-conformance/entailment-suite",
];

/// The workspace root, from this crate's manifest directory.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("the workspace root resolves")
}

/// Every file under `root`, recursively, in a deterministic order.
fn walk(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            walk(&path, out);
        } else {
            out.push(path);
        }
    }
}

type Spanned<'a> = purrdf_sparql_algebra::lexer::Spanned<'a>;

/// A corpus lexer: source text in, one token stream out. Named so the
/// [`lexer_for`] signature stays readable — the bare `fn` pointer type spells
/// out two nested generics and a lifetime, which is exactly the shape
/// `clippy::type_complexity` asks to be given a name.
type Lexer = fn(&str) -> purrdf_sparql_algebra::Result<Vec<Spanned<'_>>>;

/// Which lexer a corpus file needs, or `None` when it carries no RDF/SPARQL
/// syntax at all.
fn lexer_for(path: &Path) -> Option<Lexer> {
    match path.extension().and_then(|e| e.to_str())? {
        "rq" | "ru" => Some(tokenize),
        "ttl" | "trig" | "nt" | "nq" | "n3" => Some(tokenize_turtle),
        _ => None,
    }
}

/// Recover every `cdt:List` / `cdt:Map`-typed literal in one already-tokenized
/// file.
///
/// The prefix map is built from the file's own `PREFIX p: <iri>` / `@prefix p:
/// <iri>` declarations as the token stream presents them, so a prefixed datatype
/// is resolved the way the parser resolves it rather than by matching the text
/// `cdt:`. A file that binds the SEP-0009 namespace to some other prefix is
/// handled; one that binds `cdt:` to something else is NOT mistaken for a
/// composite.
fn recover_composites(file: &Path, tokens: &[Spanned<'_>]) -> Vec<Recovered> {
    let mut prefixes: BTreeMap<&str, String> = BTreeMap::new();
    for window in tokens.windows(3) {
        let declares = matches!(&window[0].token, Token::Word(w) if w.eq_ignore_ascii_case("PREFIX"))
            || matches!(&window[0].token, Token::LangTag(t) if t.eq_ignore_ascii_case("prefix"));
        if !declares {
            continue;
        }
        if let (Token::PrefixedName(prefix, local), Token::Iri(iri)) =
            (&window[1].token, &window[2].token)
            && local.is_empty()
        {
            prefixes.insert(prefix, iri.to_string());
        }
    }

    let mut found = Vec::new();
    for window in tokens.windows(3) {
        let (Token::StringLit(lexical) | Token::LongStringLit(lexical)) = &window[0].token else {
            continue;
        };
        if window[1].token != Token::HatHat {
            continue;
        }
        let datatype = match &window[2].token {
            Token::Iri(iri) => iri.to_string(),
            Token::PrefixedName(prefix, local) => match prefixes.get(prefix) {
                Some(namespace) => format!("{namespace}{local}"),
                None => continue,
            },
            _ => continue,
        };
        if datatype != CDT_LIST && datatype != CDT_MAP {
            continue;
        }
        found.push(Recovered {
            file: file.to_path_buf(),
            lexical: lexical.to_string(),
            datatype,
        });
    }
    found
}

/// Recover every composite literal across every corpus root, plus the count of
/// files actually tokenized.
fn scan_corpora() -> (Vec<Recovered>, usize) {
    let root = workspace_root();
    let mut files = Vec::new();
    for corpus in CORPUS_ROOTS {
        walk(&root.join(corpus), &mut files);
    }

    let mut recovered = Vec::new();
    let mut tokenized = 0usize;
    for file in files {
        let Some(lexer) = lexer_for(&file) else {
            continue;
        };
        let Ok(source) = std::fs::read_to_string(&file) else {
            continue;
        };
        tokenized += 1;
        // A corpus deliberately carries syntactically INVALID files (every
        // negative-syntax vector is one). A file the lexer refuses carries no
        // recoverable literal, and its refusal is the other suites' business,
        // not this scan's.
        let Ok(tokens) = lexer(&source) else {
            continue;
        };
        let relative = file.strip_prefix(&root).unwrap_or(&file).to_path_buf();
        recovered.extend(recover_composites(&relative, &tokens));
    }
    (recovered, tokenized)
}

/// **The mitigation.** No composite literal anywhere in any corpus this
/// workspace ships needs either of PurRDF's two supersets of the SEP-0009
/// lexical space.
///
/// Every recovered literal that parses is graded, and every grade must be
/// [`LexicalSpace::Sep0009`]. A single `PurrdfSuperset` would mean an upstream
/// vector exercises a form the published grammar cannot read — which is exactly
/// the condition under which the extension stops being unobservable and starts
/// being an incompatibility.
///
/// The counts below are pinned as EQUALITIES so the scan cannot pass by finding
/// nothing.
#[test]
fn corpus_cdt_literals_are_all_inside_sep0009() {
    let (recovered, tokenized) = scan_corpora();

    assert_eq!(
        tokenized, EXPECTED_TOKENIZED_FILES,
        "the scan tokenized {tokenized} corpus files, not the pinned \
         {EXPECTED_TOKENIZED_FILES} — the corpus moved, or the file-kind filter did"
    );
    assert_eq!(
        recovered.len(),
        EXPECTED_COMPOSITE_LITERALS,
        "the scan recovered {} composite literals, not the pinned \
         {EXPECTED_COMPOSITE_LITERALS} — a scan that finds nothing proves nothing",
        recovered.len()
    );

    let mut parsed = 0usize;
    let mut ill_formed = 0usize;
    for item in &recovered {
        match parse_cdt_by_iri(&item.lexical, &item.datatype) {
            Ok(Some(value)) => {
                parsed += 1;
                assert_eq!(
                    lexical_space(&value),
                    LexicalSpace::Sep0009,
                    "{}: the composite literal {:?} needs a PurRDF superset of the \
                     SEP-0009 lexical space, so a conformant reader would call it \
                     ill-formed — the extension is no longer unobservable",
                    item.file.display(),
                    item.lexical
                );
            }
            // A corpus vector that is DELIBERATELY ill-formed (`"1"^^cdt:List`,
            // `"[1,"^^cdt:Map`) denotes no value to grade. It still must not
            // reach for either widened production, so the raw form is checked
            // for both markers directly — the one place a text check is the
            // honest instrument, because there is no value to ask.
            Ok(None) | Err(_) => {
                ill_formed += 1;
                assert!(
                    !item.lexical.contains("<<("),
                    "{}: the ill-formed composite literal {:?} contains the triple-term \
                     production PurRDF adds, so an upstream vector DOES exercise the \
                     extension",
                    item.file.display(),
                    item.lexical
                );
                assert!(
                    !item.lexical.contains("--ltr") && !item.lexical.contains("--rtl"),
                    "{}: the ill-formed composite literal {:?} contains the directional \
                     language-tag production PurRDF adds",
                    item.file.display(),
                    item.lexical
                );
            }
        }
    }

    assert_eq!(
        parsed, EXPECTED_WELL_FORMED,
        "{parsed} of the recovered literals parsed, not the pinned \
         {EXPECTED_WELL_FORMED}"
    );
    assert_eq!(
        ill_formed,
        recovered.len() - EXPECTED_WELL_FORMED,
        "the well-formed and ill-formed counts must account for every recovered literal"
    );
}

/// The scan above covers `.rq` and the RDF syntaxes. It would be blind to a
/// composite literal that appeared only in an EXPECTED-RESULTS file, so this
/// pins that no such literal exists: every SPARQL Results file in every corpus
/// is free of the SEP-0009 namespace entirely.
///
/// If a future corpus adds one, this test fails and the scan above must grow a
/// results reader rather than silently keeping a hole.
#[test]
fn no_expected_results_file_carries_a_composite_literal() {
    let root = workspace_root();
    let mut files = Vec::new();
    for corpus in CORPUS_ROOTS {
        walk(&root.join(corpus), &mut files);
    }
    let mut results = 0usize;
    for file in files {
        if !matches!(
            file.extension().and_then(|e| e.to_str()),
            Some("srx" | "srj" | "tsv" | "csv")
        ) {
            continue;
        }
        results += 1;
        let Ok(source) = std::fs::read_to_string(&file) else {
            continue;
        };
        assert!(
            !source.contains("SPARQL-CDTs"),
            "{}: an expected-results file carries the SEP-0009 namespace, so the \
             lexical-space scan's coverage claim no longer holds",
            file.display()
        );
    }
    assert_eq!(
        results, EXPECTED_RESULT_FILES,
        "the coverage check visited {results} results files, not the pinned \
         {EXPECTED_RESULT_FILES}"
    );
}

/// The POSITIVE CONTROL for the scan: the grader really does report
/// [`LexicalSpace::PurrdfSuperset`] for each widened production, reached through
/// the same recovery-and-grade path the scan uses.
///
/// Without this, a grader stuck at `Sep0009` would make
/// [`corpus_cdt_literals_are_all_inside_sep0009`] a pass over something never
/// evaluated.
#[test]
fn the_grader_fires_on_each_extended_production() {
    // Superset 1: an RDF 1.2 triple term as a composite element.
    let triple = "[<<(<http://example.org/s> <http://example.org/p> \"o\"^^<http://www.w3.org/2001/XMLSchema#string>)>>]";
    let value = parse_cdt_by_iri(triple, CDT_LIST)
        .expect("the triple-term form is inside PurRDF's lexical space")
        .expect("it is a cdt:List");
    assert_eq!(lexical_space(&value), LexicalSpace::PurrdfSuperset);

    // Superset 2: a directional language-tagged literal, in both directions and
    // in both a list element and a map key.
    for form in ["[\"hello\"@en--ltr]", "[\"hello\"@he--rtl]"] {
        let value = parse_cdt_by_iri(form, CDT_LIST)
            .expect("the directional form is inside PurRDF's lexical space")
            .expect("it is a cdt:List");
        assert_eq!(
            lexical_space(&value),
            LexicalSpace::PurrdfSuperset,
            "{form} must grade as an extension"
        );
    }
    let keyed = parse_cdt_by_iri("{\"hello\"@en--rtl: 1}", CDT_MAP)
        .expect("a directional map KEY is inside PurRDF's lexical space")
        .expect("it is a cdt:Map");
    assert_eq!(lexical_space(&keyed), LexicalSpace::PurrdfSuperset);

    // The NEIGHBOURING valid case for each: the same shapes without the
    // widening still grade as strict SEP-0009, so the grader is not simply
    // answering `PurrdfSuperset` for anything unusual.
    for form in [
        "[\"hello\"@en]",
        "[[1], {1: 2}, _:b, <http://example.org/i>, null]",
    ] {
        let value = parse_cdt_by_iri(form, CDT_LIST)
            .expect("a strict SEP-0009 form parses")
            .expect("it is a cdt:List");
        assert_eq!(
            lexical_space(&value),
            LexicalSpace::Sep0009,
            "{form} is strict SEP-0009 and must grade as such"
        );
    }
}

/// The recovery itself must work, or the scan's counts mean nothing. This runs
/// the exact `recover_composites` the scan uses over a hand-written source that
/// exercises every shape it has to handle: the full-IRI datatype, a prefixed
/// one, a NON-`cdt` prefix bound to the SEP-0009 namespace, a `cdt` prefix bound
/// to something else (which must NOT be recovered), a long-quoted literal, and a
/// non-composite datatype.
// `{3:4}` and `{5:6}` are SEP-0009 cdt:Map LEXICAL FORMS, not format strings:
// `{key:value}` is the spec's own map syntax and `3`/`5` are map keys, not
// positional argument indices. Nothing here reaches a formatting macro — the
// forms are compared as data — so the lint is reading map syntax as `{}`
// syntax. Scoped to this one fixture rather than the module.
#[allow(
    clippy::literal_string_with_formatting_args,
    reason = "cdt:Map lexical forms, never formatted"
)]
#[test]
fn the_recovery_resolves_datatypes_the_way_the_parser_does() {
    let source = concat!(
        "PREFIX cdt: <http://w3id.org/awslabs/neptune/SPARQL-CDTs/>\n",
        "PREFIX comp: <http://w3id.org/awslabs/neptune/SPARQL-CDTs/>\n",
        "PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>\n",
        "ASK { FILTER( \"[1]\"^^cdt:List = \"[2]\"^^comp:List ) \n",
        "      FILTER( \"{3:4}\"^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/Map> = \
         \"\"\"{5:6}\"\"\"^^cdt:Map ) \n",
        "      FILTER( \"7\"^^xsd:integer = 7 ) }\n",
    );
    let tokens = tokenize(source).expect("the fixture tokenizes");
    let found = recover_composites(Path::new("fixture.rq"), &tokens);
    let forms: Vec<&str> = found.iter().map(|r| r.lexical.as_str()).collect();
    assert_eq!(
        forms,
        vec!["[1]", "[2]", "{3:4}", "{5:6}"],
        "every composite literal, and only those, must be recovered"
    );
    assert_eq!(
        found.iter().filter(|r| r.datatype == CDT_LIST).count(),
        2,
        "both list literals must resolve to the cdt:List IRI"
    );

    // A `cdt:` prefix bound elsewhere must NOT be mistaken for a composite.
    let shadowed = concat!(
        "PREFIX cdt: <http://example.org/not-sep-0009/>\n",
        "ASK { FILTER( \"[1]\"^^cdt:List = \"[1]\"^^cdt:List ) }\n",
    );
    let tokens = tokenize(shadowed).expect("the fixture tokenizes");
    assert!(
        recover_composites(Path::new("shadowed.rq"), &tokens).is_empty(),
        "resolution must go through the file's own prefix bindings, not the text `cdt:`"
    );
}

/// Files the scan tokenizes — every `.rq`/`.ru`/`.ttl`/`.trig`/`.nt`/`.nq`/`.n3`
/// under [`CORPUS_ROOTS`].
const EXPECTED_TOKENIZED_FILES: usize = 2885;
/// Composite literals recovered across all of them.
const EXPECTED_COMPOSITE_LITERALS: usize = 959;
/// How many of those parse to a value (the rest are the corpus's deliberately
/// ill-formed vectors).
const EXPECTED_WELL_FORMED: usize = 949;
/// Expected-results files visited by the coverage check.
const EXPECTED_RESULT_FILES: usize = 430;
