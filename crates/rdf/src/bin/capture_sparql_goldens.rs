// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Capture the native SPARQL engine as committed goldens ( Task 2/8).
//!
//!  removed oxigraph; the native [`NativeSparqlEngine`] is now the SOLE
//! SPARQL authority (the cutover proved native ≡ oxigraph). This maintainer-only
//! binary captures the native engine's deterministic SPARQL outputs over OUR corpus
//! (`queries/**` + `generated/queries/**`) and over the GAP-A `$this`-substitution
//! shapes, writing them as byte-stable golden files under
//! `crates/sparql-conformance/tests/goldens/`. The Task-4 native gate then byte-diffs
//! the engine against these frozen goldens forever (native-vs-native = a regression
//! gate).
//!
//! `crates/sparql-conformance` stays oxigraph-free: it merely RECEIVES these data
//! files; this binary is oxigraph-free too (it loads purrdf.gts via the oxigraph-free
//! `flattened_dataset_from_bytes` and runs only the native engine).
//!
//! Determinism contract: every golden is byte-stable across runs. CONSTRUCT/DESCRIBE
//! goldens are RDFC-1.0 canonical N-Quads; SELECT goldens are the SORTED multiset of
//! `row_key` lines; ASK goldens are `true`/`false`. Re-running the binary must
//! produce NO git diff.

use std::path::{Path, PathBuf};

use purrdf_core::SparqlEngine;
use purrdf_rdf::capture_support::{
    collect_corpus_files, corpus_repo_root, is_deferred_construct, is_multi_query_file,
    is_nondeterministic, row_key,
};
use purrdf_rdf::{
    BlankScope, NativeRdfFormat, RdfDataset, SparqlRequest, SparqlResult, TermRef, TermValue,
    canonicalize, dataset_from_bytes,
};
use purrdf_sparql_eval::NativeSparqlEngine;

/// Where every golden tree roots. The conformance crate (oxigraph-free) reads these.
fn goldens_root() -> PathBuf {
    corpus_repo_root()
        .join("crates")
        .join("sparql-conformance")
        .join("tests")
        .join("goldens")
}

/// Tally of one capture pass over the corpus.
#[derive(Default)]
struct Tally {
    total: usize,
    nq: usize,
    rows: usize,
    ask: usize,
    nondeterministic: usize,
    multi: usize,
    deferred: usize,
    /// `(repo-relative label, error message)` — any of these HARD-FAILS the run.
    unexpected: Vec<(String, String)>,
}

fn main() {
    let goldens = goldens_root();

    // -----------------------------------------------------------------------
    // Deliverable 1 — corpus goldens from the real merged ontology.
    // -----------------------------------------------------------------------
    let corpus_tally = capture_corpus(&goldens);

    // -----------------------------------------------------------------------
    // Deliverable 2 — GAP-A substitution goldens (tiny inline dataset).
    // -----------------------------------------------------------------------
    let subst_written = capture_substitution_goldens(&goldens);

    // -----------------------------------------------------------------------
    // Tally + hard-fail on any UNEXPECTED capture failure.
    // -----------------------------------------------------------------------
    println!("---- corpus golden capture (EPIC #906 Task 2/8, native engine) ----");
    println!("corpus files (.rq):        {}", corpus_tally.total);
    println!("  goldens .nq  (CONSTRUCT): {}", corpus_tally.nq);
    println!("  goldens .rows (SELECT):   {}", corpus_tally.rows);
    println!("  goldens .ask  (ASK):      {}", corpus_tally.ask);
    println!(
        "  nondeterministic markers: {}",
        corpus_tally.nondeterministic
    );
    println!("  multi-query markers:      {}", corpus_tally.multi);
    println!("  deferred markers:         {}", corpus_tally.deferred);
    println!(
        "  UNEXPECTED failures:      {}",
        corpus_tally.unexpected.len()
    );
    println!("substitution goldens:       {subst_written}");

    if !corpus_tally.unexpected.is_empty() {
        eprintln!(
            "\nFATAL: {} corpus query(ies) failed UNEXPECTEDLY in the native engine \
             (not nondeterministic, not multi-query, not a known-deferred construct):",
            corpus_tally.unexpected.len()
        );
        for (label, msg) in &corpus_tally.unexpected {
            eprintln!("  {label}\n    -> {msg}");
        }
        std::process::exit(1);
    }
    println!("\nOK: all corpus goldens captured with no unexpected failures.");
}

/// Load the real merged ontology and freeze the native engine over every corpus
/// `.rq` file into a golden (or a classification marker).
fn capture_corpus(goldens: &Path) -> Tally {
    // Load the merged ontology exactly as the corpus conformance gate does: the
    // oxigraph-free flattened dataset (every named graph folded into the default
    // graph), so the goldens and the gate share one identical load view.
    let gts_path = corpus_repo_root()
        .join("generated")
        .join("dist")
        .join("purrdf.gts");
    let gts_bytes = std::fs::read(&gts_path)
        .unwrap_or_else(|e| panic!("read purrdf.gts at {}: {e}", gts_path.display()));
    let dataset = purrdf_rdf::gts::flattened_dataset_from_bytes(&gts_bytes)
        .expect("native flattened dataset from gts");
    let native = NativeSparqlEngine::new();

    let corpus = collect_corpus_files();
    let repo_root = corpus_repo_root();
    let corpus_root = goldens.join("corpus");

    let mut tally = Tally::default();

    for path in &corpus {
        tally.total += 1;
        let rel = path
            .strip_prefix(&repo_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let label = rel.clone();
        // Golden stem mirrors the repo-relative .rq path under goldens/corpus/.
        let stem = corpus_root.join(&rel);
        ensure_parent(&stem);

        let query_text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));

        // --- multi-query files: no single-invocation golden. ---
        if is_multi_query_file(&query_text) {
            tally.multi += 1;
            write_marker(
                &with_ext(&stem, "skip-multi"),
                "multi-query file: contains >1 top-level SPARQL query; skipped (run native for \
                 well-formedness only)\n",
            );
            continue;
        }

        // --- nondeterministic: no golden, run native for a well-formedness check. ---
        if is_nondeterministic(&query_text) {
            tally.nondeterministic += 1;
            // Best-effort native well-formedness sanity (NOW/RAND/UUID vary per call).
            let req = SparqlRequest {
                query: &query_text,
                base_iri: None,
                substitutions: &[],
            };
            let note = match native.query(&dataset, req) {
                Ok(_) => {
                    "nondeterministic (NOW/RAND/UUID/STRUUID): no golden; gate runs native \
                          for well-formedness only\n"
                }
                Err(_) => {
                    "nondeterministic (NOW/RAND/UUID/STRUUID): no golden; native eval \
                           errored (gate runs native for well-formedness only)\n"
                }
            };
            write_marker(&with_ext(&stem, "nondeterministic"), note);
            continue;
        }

        // --- run the native engine (the sole SPARQL authority). ---
        let req = SparqlRequest {
            query: &query_text,
            base_iri: None,
            substitutions: &[],
        };
        match native.query(&dataset, req) {
            Ok(result) => write_result_golden(&stem, &result, &mut tally),
            Err(e) => {
                let msg = e.to_string();
                if is_deferred_construct(&msg) {
                    tally.deferred += 1;
                    write_marker(
                        &with_ext(&stem, "deferred"),
                        &format!(
                            "deferred construct (property-path/service/lateral/describe/\
                                  rdf12-triple-term): {msg}\n"
                        ),
                    );
                } else {
                    tally.unexpected.push((label, msg));
                }
            }
        }
    }
    tally
}

/// Serialize a successful native result into the right golden kind.
fn write_result_golden(stem: &Path, result: &SparqlResult, tally: &mut Tally) {
    match result {
        SparqlResult::Graph(graph) => {
            let nquads = canonicalize(graph).nquads;
            std::fs::write(with_ext(stem, "nq"), nquads).expect("write .nq golden");
            tally.nq += 1;
        }
        SparqlResult::Solutions {
            variables, rows, ..
        } => {
            std::fs::write(with_ext(stem, "rows"), solutions_golden(variables, rows))
                .expect("write .rows golden");
            tally.rows += 1;
        }
        SparqlResult::Boolean(value) => {
            std::fs::write(with_ext(stem, "ask"), format!("{value}\n")).expect("write .ask golden");
            tally.ask += 1;
        }
    }
}

/// Render a SELECT result as a deterministic golden: first line is the tab-joined
/// variable list (preserving query projection order), then the SORTED `row_key`
/// lines (a deterministic multiset — solution row order is not contractual).
fn solutions_golden(variables: &[String], rows: &[Vec<Option<TermValue>>]) -> String {
    let mut out = String::new();
    out.push_str(&variables.join("\t"));
    out.push('\n');
    let mut keys: Vec<String> = rows.iter().map(|r| row_key(r)).collect();
    keys.sort();
    for k in keys {
        out.push_str(&k);
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// Deliverable 2 — GAP-A substitution goldens.
// ---------------------------------------------------------------------------

/// The fixed substitution dataset (mirrors `corpus_conformance.rs`'s substitution
/// sub-gate). Written to `goldens/substitution/dataset.nt` once so the Task-4 native
/// gate replays against the IDENTICAL data. The blank-node focus `_:bn` is captured as
/// a literal `.nt` line; the gate must parse it back with a stable label.
const SUBST_DATASET_NT: &str = concat!(
    "<http://ex/alice> <http://ex/knows> <http://ex/bob> .\n",
    "<http://ex/alice> <http://ex/age> \"30\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
    "<http://ex/bob> <http://ex/knows> <http://ex/carol> .\n",
    "<http://ex/bob> <http://ex/age> \"17\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
    "_:bn <http://ex/knows> <http://ex/carol> .\n",
    "<http://ex/carol> <http://ex/member> <http://ex/club> .\n",
);

/// One substitution shape to capture.
struct SubstShape {
    name: &'static str,
    query: &'static str,
    /// `(variable, term)` bindings.
    subst: Vec<(String, TermValue)>,
}

fn iri(s: &str) -> TermValue {
    TermValue::Iri(s.to_owned())
}

fn alice_focus() -> Vec<(String, TermValue)> {
    vec![("this".to_owned(), iri("http://ex/alice"))]
}

/// Capture the GAP-A `$this`-substitution shapes. Returns the count written.
fn capture_substitution_goldens(goldens: &Path) -> usize {
    let dir = goldens.join("substitution");
    std::fs::create_dir_all(&dir).expect("mkdir goldens/substitution");

    // Pin the dataset once (text form for the native gate to parse).
    std::fs::write(dir.join("dataset.nt"), SUBST_DATASET_NT).expect("write substitution dataset");

    // Build the native engine from the parsed dataset. The blank-node label is the
    // text-parsed label (whatever the native parser assigns `_:bn`); the captured
    // golden is the engine's contract over that dataset.
    let dataset: std::sync::Arc<RdfDataset> =
        dataset_from_bytes(SUBST_DATASET_NT.as_bytes(), NativeRdfFormat::NTriples)
            .expect("parse substitution dataset IR");
    let native = NativeSparqlEngine::new();

    // The shapes mirror the corpus substitution sub-gate: subject-position,
    // object-position, projected-only ?this, FILTER-referenced, into NOT EXISTS, and
    // an IRI vs a blank focus (the blank uses the parser's label).
    let blank_label = blank_label_for_bn(&dataset);
    let shapes: Vec<SubstShape> = vec![
        SubstShape {
            name: "subject_position",
            query: "SELECT ?this ?o WHERE { ?this <http://ex/knows> ?o }",
            subst: alice_focus(),
        },
        SubstShape {
            name: "object_position",
            query: "SELECT ?this ?s WHERE { ?s <http://ex/knows> ?this }",
            subst: vec![("this".to_owned(), iri("http://ex/carol"))],
        },
        SubstShape {
            name: "projected_only_focus",
            query: "SELECT ?this WHERE { ?this <http://ex/knows> ?o }",
            subst: alice_focus(),
        },
        SubstShape {
            name: "filter_referenced",
            query: "SELECT ?this ?o WHERE { ?this <http://ex/knows> ?o . \
                    ?this <http://ex/age> ?n FILTER(?n > 18) }",
            subst: alice_focus(),
        },
        SubstShape {
            name: "into_not_exists",
            query: "SELECT ?this ?o WHERE { ?this <http://ex/knows> ?o \
                    FILTER NOT EXISTS { ?this <http://ex/member> ?c } }",
            subst: alice_focus(),
        },
        SubstShape {
            name: "blank_focus",
            query: "SELECT ?this ?o WHERE { ?this <http://ex/knows> ?o }",
            subst: vec![(
                "this".to_owned(),
                TermValue::Blank {
                    label: blank_label,
                    scope: BlankScope::DEFAULT,
                },
            )],
        },
    ];

    let mut written = 0usize;
    for shape in &shapes {
        let req = SparqlRequest {
            query: shape.query,
            base_iri: None,
            substitutions: &shape.subst,
        };
        let nat_result = native
            .query(&dataset, req)
            .unwrap_or_else(|e| panic!("native substitution {} failed: {e:?}", shape.name));
        let (nat_vars, nat_rows) = expect_solutions(&nat_result, shape.name);
        let golden = solutions_golden(nat_vars, nat_rows);

        std::fs::write(dir.join(format!("{}.rows", shape.name)), &golden)
            .expect("write substitution .rows golden");
        std::fs::write(
            dir.join(format!("{}.query", shape.name)),
            format!("{}\n", shape.query),
        )
        .expect("write substitution .query");
        std::fs::write(
            dir.join(format!("{}.subst", shape.name)),
            subst_lines(&shape.subst),
        )
        .expect("write substitution .subst");
        written += 1;
    }
    written
}

/// Serialize the `(variable, term)` bindings as deterministic `var=term-debug` lines
/// (sorted) so the Task-4 gate can reconstruct the substitution natively.
fn subst_lines(subst: &[(String, TermValue)]) -> String {
    let mut lines: Vec<String> = subst.iter().map(|(v, t)| format!("{v}={t:?}")).collect();
    lines.sort();
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Find the label the native text parser assigned to `_:bn`. There is exactly one
/// blank node in [`SUBST_DATASET_NT`]; return its value-model label.
fn blank_label_for_bn(dataset: &RdfDataset) -> String {
    for q in dataset.quads() {
        for tid in [q.s, q.o] {
            if let TermRef::Blank { label, .. } = dataset.resolve(tid) {
                return label.to_owned();
            }
        }
    }
    panic!("substitution dataset must contain a blank node (_:bn)")
}

fn expect_solutions<'a>(
    result: &'a SparqlResult,
    name: &str,
) -> (&'a [String], &'a [Vec<Option<TermValue>>]) {
    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => (variables, rows),
        other => panic!("substitution {name}: expected SELECT solutions, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Small filesystem helpers.
// ---------------------------------------------------------------------------

/// Replace the path's extension (the corpus stem keeps the `.rq` name; we swap it for
/// `.nq` / `.rows` / `.ask` / a marker extension).
fn with_ext(stem: &Path, ext: &str) -> PathBuf {
    stem.with_extension(ext)
}

fn ensure_parent(path: &Path) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|e| panic!("mkdir {}: {e}", parent.display()));
    }
}

fn write_marker(path: &Path, note: &str) {
    std::fs::write(path, note).unwrap_or_else(|e| panic!("write marker {}: {e}", path.display()));
}
