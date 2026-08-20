// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The whole-serializer round-trip guard `serialize.rs`'s own module doc
//! claims but no test before this one measured across a real corpus:
//! `crates/sparql-algebra/src/serialize.rs:17-19` says
//! `parse(pattern_to_select_query(p))` reproduces `p` "for every
//! `GraphPattern`/`Expression` variant the parser emits" — a claim a handful
//! of hand-authored `roundtrip_*` unit tests in `serialize.rs` itself can
//! gesture at but never PROVE, and Task 2's own investigation (the
//! `fmt_join_right_operand` re-association class, and the variable-endpoint
//! `SERVICE` double-wrap) found it FALSE. This sweep is the proof, run over
//! every real SPARQL query text this repository ships or vendors.
//!
//! # Corpus
//!
//! * The vendored W3C SPARQL 1.1 and 1.2 test suites
//!   (`crates/sparql-conformance/suite/w3c-sparql11`,
//!   `.../w3c-sparql12`) — every `.rq` file.
//! * The first-party `purrdf-extend` suite
//!   (`crates/sparql-conformance/suite/purrdf-extend`) — every `.rq` file.
//! * The user-guide's own worked examples — every fenced ` ```sparql ` block
//!   under `docs/**/*.md` (the same "documentation is a shipped surface"
//!   principle `shipped_sparql_examples.rs` established, narrowed here to the
//!   ONE extraction pass — whole fenced blocks — that yields genuinely
//!   complete, standalone queries worth round-tripping; that other test
//!   already proves every embedded fragment PARSES, which this sweep does
//!   not re-litigate).
//!
//! **`.ru` (UPDATE) files are explicitly OUT OF SCOPE.** This sweep exercises
//! [`pattern_to_select_query`], the QUERY-pattern serializer — there is no
//! UPDATE-request serializer in this crate to test, and inventing a sweep
//! surface for a function that does not exist would test nothing. `.ru`
//! fixtures under the swept suite roots are simply never opened here (`.rq`
//! is the only extension [`collect_rq`] follows).
//!
//! # Method
//!
//! [`pattern_to_select_query`]'s own established contract (see its
//! `serialize.rs` module tests' `where_body`/`assert_roundtrip` helpers) is
//! over a query's WHERE-body pattern, not the whole [`Query`]: it always
//! renders `SELECT * WHERE { … }` (or an aggregate chain's own complete
//! `SELECT`), so a query's OWN top-level form (`ASK`/`CONSTRUCT`/`DESCRIBE`
//! vs `SELECT`) and an EXPLICIT (non-`*`) projection list are, BY THAT
//! FUNCTION'S DESIGN, not what it reproduces — its driving use case is
//! `SERVICE` federation forwarding a WHERE-body fragment, which always wants
//! `*`. This sweep tests exactly what the function promises: parse the
//! corpus text, strip one outer `Project` if present (the `SELECT` scaffold;
//! `ASK`/`CONSTRUCT`/`DESCRIBE` carry none), serialize the body, re-parse
//! (always as a fresh `SELECT`), strip its outer `Project` the same way, and
//! compare.
//!
//! The one permitted modulo is [`normalize_join_assoc`]: `Join` is
//! associative (`serialize.rs`'s own class-fix left it deliberately
//! unbraced, stating the round-trip contract is semantics-preserved, not
//! tree-identical, exactly where semantics do not require tree identity), so
//! a `Join` spine may re-associate across a serialize/re-parse round trip
//! without that being a defect. `normalize_join_assoc` left-linearizes every
//! `Join` spine in both trees before comparing; nothing else is normalized —
//! any OTHER structural disagreement is a real one.
//!
//! # Corpus items that do not even parse
//!
//! A `.rq`/doc-example text that fails to PARSE at all (a W3C
//! `NegativeSyntaxTest`/`NegativeUpdateSyntaxTest` fixture, or a construct
//! this parser does not accept) is outside this sweep's contract — there is
//! no pattern to hand the serializer. Parser grammar coverage is
//! `sparql_conformance`'s job (its own pass/xfail/unmodeled ledger against
//! the official manifest test types); this sweep's `XFAIL` ledger is
//! reserved for the DIFFERENT, narrower claim named below. Such items are
//! counted (`unparseable`, printed) but never asserted on.
//!
//! # The `XFAIL` ledger
//!
//! An item that PARSES but whose round-trip disagrees is either (a) a real
//! serializer defect — fixed in `serialize.rs` in the same change that added
//! this sweep, never ledgered — or (b) a genuine "emit-only" gap: an algebra
//! shape the parser can produce but the surface grammar has no construct to
//! re-emit it through (the PurRDF predicate-wildcard path extension
//! (`<any>`), documented emit-only on
//! [`purrdf_sparql_algebra::PropertyPathExpression`]'s `Display`, is the
//! one shape in this crate with that property). `XFAIL` entries name the
//! corpus-relative path and the construct; `assert_eq!(XFAIL.len(), K)`
//! keeps the ledger's size a visible diff, and an entry that round-trips
//! cleanly (the gap got fixed and nobody removed the ledger row) fails the
//! sweep rather than sitting stale.

use std::path::{Path, PathBuf};

use purrdf_sparql_algebra::{GraphPattern, Query, SparqlParser, pattern_to_select_query};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root resolves")
}

/// Every `.rq` file under `dir`, recursively, in a stable (sorted) order.
/// `.ru` and every other extension are ignored — see this file's module doc
/// on why UPDATE requests are out of scope for a QUERY-pattern serializer
/// sweep.
fn collect_rq(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rq(&path, out);
        } else if path.extension().is_some_and(|x| x == "rq") {
            out.push(path);
        }
    }
    out.sort();
}

/// Every fenced ` ```sparql ` block under `dir`'s Markdown files — the book's
/// own worked, standalone examples (unlike `shipped_sparql_examples.rs`'s
/// other extraction passes, which exist to catch a fragment BROKEN mid-prose
/// and so deliberately look at partial/schematic text too, a fenced whole
/// block is written to be a complete, real query, which is what a
/// round-trip sweep needs).
fn collect_doc_examples(dir: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    let mut md_files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
                md_files.push(path);
            }
        }
    }
    md_files.sort();
    drop(entries);
    for file in md_files {
        let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("read {file:?}: {e}"));
        let mut in_fence = false;
        let mut body = String::new();
        let mut block_no = 0usize;
        for line in text.lines() {
            let trimmed = line.trim_start();
            if in_fence {
                if trimmed.starts_with("```") {
                    in_fence = false;
                    block_no += 1;
                    let label = format!(
                        "{}#sparql-block-{block_no}",
                        file.to_string_lossy().replace('\\', "/")
                    );
                    out.push((label, std::mem::take(&mut body)));
                } else {
                    body.push_str(line);
                    body.push('\n');
                }
                continue;
            }
            if trimmed.starts_with("```sparql") {
                in_fence = true;
                body.clear();
            }
        }
    }
    out
}

/// A query's WHERE-body pattern, whichever [`Query`] variant it is.
fn query_pattern(q: &Query) -> &GraphPattern {
    match q {
        Query::Select { pattern, .. }
        | Query::Construct { pattern, .. }
        | Query::Describe { pattern, .. }
        | Query::Ask { pattern, .. } => pattern,
    }
}

/// Strip exactly one outer `Project` (the `SELECT` scaffold) to recover the
/// WHERE body — the shape [`pattern_to_select_query`] consumes and always
/// re-produces on re-parse (see this file's module doc's "Method" section).
fn where_body(p: &GraphPattern) -> GraphPattern {
    match p {
        GraphPattern::Project { inner, .. } => (**inner).clone(),
        other => other.clone(),
    }
}

/// Left-linearize every `Join` spine in `p` — the ONE permitted modulo this
/// sweep's equality check allows (see this file's module doc). Every other
/// [`GraphPattern`] variant is reconstructed with its children normalized
/// the same way and every non-pattern field carried through unchanged; the
/// match is exhaustive (no wildcard arm), so a future algebra variant is a
/// compile error here until this function is taught its shape, not a
/// silently-unnormalized blind spot.
fn normalize_join_assoc(p: &GraphPattern) -> GraphPattern {
    match p {
        GraphPattern::Join { .. } => {
            let mut leaves = Vec::new();
            flatten_join(p, &mut leaves);
            let mut normalized = leaves.into_iter().map(normalize_join_assoc);
            let first = normalized
                .next()
                .expect("a Join node flattens to at least two leaves");
            normalized.fold(first, |acc, next| GraphPattern::Join {
                left: Box::new(acc),
                right: Box::new(next),
            })
        }
        GraphPattern::Bgp { patterns } => GraphPattern::Bgp {
            patterns: patterns.clone(),
        },
        GraphPattern::Path {
            subject,
            path,
            object,
        } => GraphPattern::Path {
            subject: subject.clone(),
            path: path.clone(),
            object: object.clone(),
        },
        GraphPattern::PropertyFunction(call) => GraphPattern::PropertyFunction(call.clone()),
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => GraphPattern::LeftJoin {
            left: Box::new(normalize_join_assoc(left)),
            right: Box::new(normalize_join_assoc(right)),
            expression: expression.clone(),
        },
        GraphPattern::Lateral { left, right } => GraphPattern::Lateral {
            left: Box::new(normalize_join_assoc(left)),
            right: Box::new(normalize_join_assoc(right)),
        },
        GraphPattern::Filter { expr, inner } => GraphPattern::Filter {
            expr: expr.clone(),
            inner: Box::new(normalize_join_assoc(inner)),
        },
        GraphPattern::Union { left, right } => GraphPattern::Union {
            left: Box::new(normalize_join_assoc(left)),
            right: Box::new(normalize_join_assoc(right)),
        },
        GraphPattern::Graph { name, inner } => GraphPattern::Graph {
            name: name.clone(),
            inner: Box::new(normalize_join_assoc(inner)),
        },
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => GraphPattern::Extend {
            inner: Box::new(normalize_join_assoc(inner)),
            variable: variable.clone(),
            expression: expression.clone(),
        },
        GraphPattern::Minus { left, right } => GraphPattern::Minus {
            left: Box::new(normalize_join_assoc(left)),
            right: Box::new(normalize_join_assoc(right)),
        },
        GraphPattern::Service {
            name,
            inner,
            silent,
        } => GraphPattern::Service {
            name: name.clone(),
            inner: Box::new(normalize_join_assoc(inner)),
            silent: *silent,
        },
        GraphPattern::Values {
            variables,
            bindings,
        } => GraphPattern::Values {
            variables: variables.clone(),
            bindings: bindings.clone(),
        },
        GraphPattern::OrderBy { inner, expression } => GraphPattern::OrderBy {
            inner: Box::new(normalize_join_assoc(inner)),
            expression: expression.clone(),
        },
        GraphPattern::Project { inner, variables } => GraphPattern::Project {
            inner: Box::new(normalize_join_assoc(inner)),
            variables: variables.clone(),
        },
        GraphPattern::Distinct { inner } => GraphPattern::Distinct {
            inner: Box::new(normalize_join_assoc(inner)),
        },
        GraphPattern::Reduced { inner } => GraphPattern::Reduced {
            inner: Box::new(normalize_join_assoc(inner)),
        },
        GraphPattern::Slice {
            inner,
            start,
            length,
        } => GraphPattern::Slice {
            inner: Box::new(normalize_join_assoc(inner)),
            start: *start,
            length: *length,
        },
        GraphPattern::Group {
            inner,
            variables,
            aggregates,
        } => GraphPattern::Group {
            inner: Box::new(normalize_join_assoc(inner)),
            variables: variables.clone(),
            aggregates: aggregates.clone(),
        },
    }
}

fn flatten_join<'a>(p: &'a GraphPattern, out: &mut Vec<&'a GraphPattern>) {
    match p {
        GraphPattern::Join { left, right } => {
            flatten_join(left, out);
            flatten_join(right, out);
        }
        other => out.push(other),
    }
}

/// Parse `text`, strip its outer `Project`, serialize, re-parse, strip again,
/// and compare modulo [`normalize_join_assoc`]. `Ok(())` on success;
/// `Err` describes the mismatch. Panics only on an internal contract
/// violation (`pattern_to_select_query`'s own re-parse yielding something
/// other than `Query::Select`, which its own doc guarantees never happens).
fn roundtrip(original: &Query) -> Result<(), String> {
    let body = where_body(query_pattern(original));
    let text = pattern_to_select_query(&body);
    let reparsed = SparqlParser::new()
        .parse_query(&text)
        .map_err(|e| format!("re-parse of the serialized text failed: {e}\n  text: {text}"))?;
    let Query::Select {
        pattern: reparsed_pattern,
        ..
    } = &reparsed
    else {
        panic!("pattern_to_select_query's own contract is a re-parseable SELECT; got {reparsed:?}");
    };
    let reparsed_body = where_body(reparsed_pattern);
    let original_norm = normalize_join_assoc(&body);
    let reparsed_norm = normalize_join_assoc(&reparsed_body);
    if original_norm != reparsed_norm {
        return Err(format!(
            "round-trip mismatch (modulo Join re-association)\n  text: {text}\n  \
             original:  {original_norm:?}\n  reparsed:  {reparsed_norm:?}"
        ));
    }
    Ok(())
}

/// (corpus-relative path or `file#sparql-block-N` label, emit-only construct
/// name). See this file's module doc's "The `XFAIL` ledger" section: an
/// entry here is a construct the PARSER can produce but the surface grammar
/// has no construct to re-emit — never a route around a real serializer bug.
const XFAIL: [(&str, &str); 0] = [];

#[test]
fn corpus_round_trips_through_the_serializer() {
    let root = workspace_root();

    let mut rq_files = Vec::new();
    for suite in [
        "crates/sparql-conformance/suite/w3c-sparql11",
        "crates/sparql-conformance/suite/w3c-sparql12",
        "crates/sparql-conformance/suite/purrdf-extend",
    ] {
        let dir = root.join(suite);
        assert!(
            dir.is_dir(),
            "expected corpus root {suite} to exist under the workspace"
        );
        collect_rq(&dir, &mut rq_files);
    }
    let doc_examples = collect_doc_examples(&root.join("docs"));

    let seen = rq_files.len() + doc_examples.len();
    println!(
        "corpus round-trip sweep: {} vendored/first-party .rq files + {} doc examples = {seen} \
         total",
        rq_files.len(),
        doc_examples.len()
    );
    // Measured at authoring time: 352 w3c-sparql11 + 242 w3c-sparql12 + 41
    // purrdf-extend .rq files, plus 9 fenced ```sparql blocks in the book's
    // temporal-arithmetic section (the only file under docs/ that has any) —
    // 644 total. A corpus that stops loading (a moved suite root, a broken
    // walk) silently sweeps far less than this and must fail loudly instead.
    const N: usize = 644;
    assert!(
        seen >= N,
        "the corpus round-trip sweep saw only {seen} items, expected at least {N} — a corpus \
         that stops loading is a silent regression, not a smaller sweep"
    );

    assert_eq!(
        XFAIL.len(),
        0,
        "keep this in sync with the XFAIL array literal's length"
    );

    // Several W3C fixtures reference a relative IRI (`<ng-01.ttl>`, `<g>`, …)
    // that only resolves against the manifest's own per-test base — with NO
    // base at all, those are a bare (and here, legitimate) "invalid IRI"
    // parse error, which would otherwise misclassify a perfectly
    // ROUND-TRIPPABLE query as `unparseable` and quietly shrink this sweep's
    // real coverage. The exact base value is immaterial to what this sweep
    // checks (an absolute IRI baked into the FIRST parse round-trips
    // byte-for-byte regardless of which absolute IRI it resolved to; the
    // serialized text carries it already-resolved, so the RE-parse inside
    // `roundtrip` needs no base at all) — `example.org`, per AGENTS.md's
    // fixture convention.
    let parser = SparqlParser::new().with_base_iri("https://example.org/corpus/");
    let mut unparseable = 0usize;
    let mut xfail_matched: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut failures = Vec::new();

    let mut items: Vec<(String, String)> = Vec::with_capacity(seen);
    for path in &rq_files {
        let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        let label = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        items.push((label, text));
    }
    items.extend(doc_examples);

    for (label, text) in &items {
        let Ok(query) = parser.parse_query(text) else {
            // Outside this sweep's contract — see the module doc's
            // "Corpus items that do not even parse" section.
            unparseable += 1;
            continue;
        };
        let xfail_entry = XFAIL.iter().find(|(path, _)| path == label);
        match (roundtrip(&query), xfail_entry) {
            (Ok(()), None) => {}
            (Ok(()), Some((path, construct))) => failures.push(format!(
                "{path}: XFAIL entry for {construct:?} round-trips cleanly now — remove the \
                 ledger entry"
            )),
            (Err(msg), None) => failures.push(format!("{label}: {msg}")),
            (Err(_), Some((path, construct))) => {
                xfail_matched.insert(path);
                let _ = construct;
            }
        }
    }

    assert_eq!(
        xfail_matched.len(),
        XFAIL.len(),
        "an XFAIL entry never matched any swept, parseable item — a dead ledger row \
         (matched: {xfail_matched:?})"
    );

    println!(
        "unparseable (skipped, outside the serializer's contract — see module doc): \
         {unparseable}"
    );

    assert!(
        failures.is_empty(),
        "{} corpus item(s) failed the round-trip sweep:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
