// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! **Three ways to obtain a shapes graph must answer the same question the same
//! way, over every shapes graph the two SHACL corpora contain.**
//!
//! A prepared shapes product exists so that a shapes graph compiled once can be
//! executed later without being compiled again. That makes three lanes to the
//! same validator:
//!
//! * **PARSE** — the shapes graph is parsed from its source document, as it always
//!   was;
//! * **ADMIT** — the parse is written out as a product, and the product is
//!   restored by the common path, which trusts the memo the writer left;
//! * **REBUILD** — the same product is restored by the forward-compatibility
//!   path, which ignores the memo and re-derives the whole shapes graph from the
//!   dataset the product carries.
//!
//! The property this harness executes is that all three produce the IDENTICAL
//! validation report. Not "all three conform", not "all three found the same
//! number of violations" — the same report, byte for byte.
//!
//! # Why this is not a second conformance grading
//!
//! `w3c_conformance.rs` and `conformance.rs` already decide whether the engine's
//! answer is the RIGHT answer. Nothing here re-decides that, and nothing here
//! reads an expected report. A case whose engine answer is wrong is wrong in all
//! three lanes identically and passes here, which is correct: this harness's
//! subject is the codec, not the validator. What it adds is the one question
//! those two cannot ask, because each of them runs exactly one lane — whether the
//! codec changes the answer.
//!
//! # The comparison surface
//!
//! The report's N-Triples rendering, compared as bytes. It is the strongest
//! surface available and deliberately stronger than a tuple set:
//! `sh:sourceShape` and `sh:resultMessage` are both in it, so a restore that
//! attributed a violation to the wrong shape — or merged two blank-node-identified
//! shapes into one — fails here even though every focus node, path and component
//! still matched.
//!
//! Blank-node labelling does NOT force a weaker comparison, and the reason is
//! worth stating because it is the usual excuse for one. The labels a report
//! carries come from the data graph and the shapes graph; the labels it MINTS
//! (`_:report`, `_:r0`, …) are assigned in result order. All three lanes validate
//! the same data graph object, and a product that round-tripped its shapes
//! dataset faithfully reproduces the shapes graph's labels too — that is part of
//! what is under test here, not an obstacle to testing it. So the raw bytes are
//! stable across lanes when the codec is correct, and unstable exactly when it is
//! not. No canonicalization is applied and nothing is weakened.
//!
//! A case whose validation FAILS is compared too: the three lanes must fail
//! identically, with the same error text. A codec bug that turned a validation
//! error into a silent empty report is a disagreement, and is caught.
//!
//! # The partition
//!
//! Every discovered case lands in exactly one of four buckets, and every bucket
//! is counted and asserted:
//!
//! 1. **UNLOADABLE** — the case's own RDF does not load as a shapes graph plus a
//!    data graph. There is nothing to pack, so there is nothing for this harness
//!    to say; the two conformance harnesses grade these cases. Counted and named,
//!    never silently dropped, and admitted only when the case's own corpus
//!    DECLARES that the validator must refuse it (`mf:result sht:Failure`), so
//!    the bucket is the suite's refusal set rather than an excuse list.
//! 2. **REFUSED** — the shapes graph loads but `to_product` refuses it, naming a
//!    [`ProductDimension`]. This is a ledger with declared membership
//!    ([`REFUSAL_LEDGER`]) and it runs both ways: a case that starts refusing
//!    fails the suite, and a ledgered case that starts PACKING fails the suite
//!    too.
//! 3. **AGREED** — the shapes graph packs and all three lanes produce the same
//!    answer. The pass bucket.
//! 4. **DISAGREED** — the shapes graph packs and the lanes differ. Always a hard
//!    failure; this is the entire reason the harness exists. Anything that goes
//!    wrong AFTER a successful pack — a product that will not open, an `admit`
//!    that refuses, a `rebuild` that refuses — lands here rather than in the
//!    refusal ledger, because the writer already certified those bytes.
//!
//! # Anti-vacuity
//!
//! A lane that refused every shapes graph would leave bucket 2 holding the whole
//! corpus and every equivalence assertion vacuously true — a perfectly green
//! "equivalence" suite that proves the product surface is unusable. Over-refusal
//! is a first-class bug here, the mirror of a silent drop, so the floors below
//! are asserted as hard as the equality is:
//!
//! * the discovered case total is exact, so corpus drift is loud;
//! * the agreed bucket is exact, and is the whole corpus minus two small,
//!   individually justified sets;
//! * agreement on an ERROR is not evidence that reports agree, so the count of
//!   cases that agreed on a real report is asserted separately;
//! * a report with no results is agreed on trivially, so the count of cases that
//!   agreed on a report carrying at least one validation result is asserted
//!   separately too — that is the number that says the three lanes really did
//!   find the same violations in the same places.
//!
//! Run with `--nocapture` for the scoreboard:
//! `cargo test -p purrdf-shapes --test product_corpus_equivalence -- --nocapture`

mod shacl_corpora;

use std::collections::BTreeMap;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use purrdf::RdfDataset;
use purrdf_shapes::engine::{PreparedShapes, validate_dataset_with_shapes_graph};
use purrdf_shapes::product::{
    HostBindings, ProductDimension, ShapesProduct, ShapesProductError, ShapesProfile,
};
use purrdf_shapes::shapes::Shapes;

use shacl_corpora::{
    Expected, FIRST_PARTY_TOTAL_CASES, W3C_TOTAL_CASES, file_iri, first_party_box_role_vocab,
    first_party_cases, w3c_cases,
};

/// Every case both corpora contribute. Asserted exactly, so a corpus that grew or
/// shrank without this harness noticing fails rather than quietly measuring less.
const TOTAL_CASES: usize = W3C_TOTAL_CASES + FIRST_PARTY_TOTAL_CASES;

// ── Bucket 1: cases whose own RDF does not load ────────────────────────────────

/// The exact number of cases whose shapes graph or data graph does not load.
///
/// Today every one of them is a case the vendored manifest itself declares
/// `mf:result sht:Failure` — an input the validator is REQUIRED to reject — and
/// the corpus contains exactly seven such cases, all under
/// `sparql/pre-binding/`, each carrying a `sh:sparql` body that SHACL's
/// pre-binding rules forbid. So this bucket is not an excuse list: it is the
/// suite's own refusal set, and `Case::refusal_is_declared` enforces that
/// correspondence case by case rather than trusting the number.
///
/// The count is asserted in addition to the rule, because the rule alone would
/// be satisfied by a parser that had started refusing NOTHING at all.
const UNLOADABLE_CASES: usize = 7;

// ── Bucket 2: the refusal ledger ──────────────────────────────────────────────

/// Shapes graphs that load but that `to_product` refuses, with the dimension the
/// refusal named and why the refusal is correct.
///
/// **It is empty, and that is the measurement.** Every shapes graph in both
/// corpora that parses at all — SHACL Core, SHACL-SPARQL with its constraint
/// bodies and custom components, and the SHACL-AF node expressions, targets and
/// functions — can be written as a prepared product and restored down both lanes.
/// The product format is not a subset of what the validator accepts.
///
/// Both directions are enforced, so the emptiness cannot rot. A case here that
/// starts PACKING fails the suite (the ledger entry is stale and the case belongs
/// in the measured population); a case not here that starts REFUSING fails the
/// suite (the product surface just lost a shapes graph it used to carry). The
/// second direction is the one that matters most: over-refusal is invisible in a
/// pass/fail count, because a refused case simply stops being compared and every
/// remaining comparison still succeeds.
const REFUSAL_LEDGER: &[(&str, ProductDimension, &str)] = &[];

// ── Anti-vacuity counts ───────────────────────────────────────────────────────

/// The exact number of cases that must reach bucket 3.
const AGREED_CASES: usize = TOTAL_CASES - UNLOADABLE_CASES - REFUSAL_LEDGER.len();

/// The exact number of agreed cases whose three lanes agreed on a REPORT rather
/// than on an identical validation error.
///
/// Three lanes that all fail identically DO agree, and that agreement is worth
/// asserting — but it is not evidence that two reports were ever compared. This
/// count is what makes the report comparison non-vacuous, and it is exact rather
/// than a floor for the reason every count in this repository's conformance
/// harnesses is exact: a floor absorbs a corpus that quietly shrank, and a lane
/// that started erroring on a case it used to report on would slide under one.
const AGREED_ON_REPORT_CASES: usize = 192;

/// The exact number of agreed cases whose shared report carries at least one
/// validation result.
///
/// An empty report is agreed on by any two lanes that both found nothing,
/// including two lanes that lost the entire shapes graph in the same way. This is
/// the count that says the three lanes found the SAME violations at the same
/// focus nodes; the remaining agreed cases are the ones the corpus expects to
/// conform.
const AGREED_WITH_RESULTS_CASES: usize = 179;

// ── One case ──────────────────────────────────────────────────────────────────

/// A case reduced to what all three lanes need: a parsed shapes graph, and the
/// data graph to run it over.
struct Loaded {
    shapes: Arc<Shapes>,
    data: Arc<RdfDataset>,
}

/// One discovered case, corpus-qualified, with its inputs already loaded (or the
/// reason they could not be).
struct Case {
    /// Corpus-qualified identifier, so the two corpora cannot collide.
    id: String,
    /// Whether this case's corpus DECLARES that the validator must refuse it.
    ///
    /// Only the vendored suite has such a declaration (`mf:result sht:Failure`),
    /// and it is the only thing that makes a case's inputs legitimately
    /// unloadable here. The first-party corpus has no such notion: every one of
    /// its cases is a report the engine is expected to produce, so a first-party
    /// case that stops loading is a regression, never a bucket-1 entry.
    refusal_is_declared: bool,
    loaded: Result<Loaded, String>,
}

/// What one lane produced: the report's N-Triples plus its result count, or the
/// error the lane failed with.
#[derive(PartialEq, Eq)]
struct LaneOutcome {
    /// `Ok` carries `(report N-Triples, result count)`; `Err` carries the failure.
    answer: Result<(String, usize), String>,
}

impl LaneOutcome {
    /// A short rendering for a disagreement message: the full N-Triples of two
    /// differing reports is unreadable, so name the shape and the first
    /// difference is reported separately.
    fn summary(&self) -> String {
        match &self.answer {
            Ok((nt, results)) => format!("report with {results} result(s), {} bytes", nt.len()),
            Err(e) => format!("failed: {e}"),
        }
    }
}

/// Where a case landed.
enum Bucket {
    /// Bucket 1: the case's own RDF does not load.
    Unloadable(String),
    /// Bucket 2: the shapes graph loads, `to_product` refuses it.
    Refused(ShapesProductError),
    /// Bucket 3: all three lanes agreed. Carries the shared outcome, for the
    /// anti-vacuity tallies.
    Agreed(LaneOutcome),
    /// Bucket 4: the lanes disagreed, or something after the pack refused.
    Disagreed(String),
}

// ── Loading ───────────────────────────────────────────────────────────────────

/// Load a vendored W3C case: parse the shapes document against its own `file://`
/// base, recover the document prefix map from the source text (the native codec
/// drops prefixes when it folds to the IR, and SHACL-SPARQL bodies need them),
/// and record the manifest's `sht:shapesGraph` IRI into the parsed shapes.
///
/// The shapes-graph IRI is recorded INTO the `Shapes` rather than passed to the
/// validator, so that a product that failed to carry it shows up as a
/// disagreement instead of being papered over by a harness that re-supplied it to
/// every lane.
fn load_w3c(case: &shacl_corpora::W3cCase) -> Result<Loaded, String> {
    let shapes_text = fs::read_to_string(&case.shapes_path)
        .map_err(|e| format!("cannot read shapes {}: {e}", case.shapes_path.display()))?;
    let shapes_dataset = purrdf::parse_dataset(
        shapes_text.as_bytes(),
        "text/turtle",
        Some(&file_iri(&case.shapes_path)),
    )
    .map_err(|e| format!("shapes graph parse error: {e}"))?;
    let doc_prefixes = purrdf_shapes::text_ingest::extract_prefixes(&shapes_text);
    let shapes = purrdf_shapes::shapes::from_dataset_with_config_and_graph(
        &shapes_dataset,
        &doc_prefixes,
        None,
        case.shapes_graph_iri.clone(),
    )
    .map_err(|e| format!("shapes parse error: {e}"))?;

    let data = if case.data_path == case.shapes_path {
        shapes_dataset
    } else {
        shacl_corpora::parse_turtle_file(&case.data_path)
            .map_err(|e| format!("data graph parse error: {e}"))?
    };

    Ok(Loaded {
        shapes: Arc::new(shapes),
        data,
    })
}

/// Load a first-party corpus case, with the corpus's caller-supplied box-role
/// vocabulary configured exactly as `conformance.rs` configures it. PurRDF mints
/// no vocabulary IRIs, so a harness that left it out would be running a different
/// shapes graph than the corpus describes — and would then prove the product
/// round-trips a feature the corpus was written to exercise, without ever
/// switching it on.
fn load_first_party(case: &shacl_corpora::FirstPartyCase) -> Result<Loaded, String> {
    let shapes_ttl = fs::read_to_string(&case.shapes_path)
        .map_err(|e| format!("cannot read shapes.ttl: {e}"))?;
    let data_nt =
        fs::read_to_string(&case.data_path).map_err(|e| format!("cannot read data.nt: {e}"))?;

    let shapes = purrdf_shapes::engine::parse_shapes_with_config(
        &shapes_ttl,
        None,
        Some(first_party_box_role_vocab()),
    )
    .map_err(|e| format!("shapes parse error: {e}"))?;
    let data = purrdf_shapes::text_ingest::parse_ntriples_to_dataset(&data_nt)
        .map_err(|errors| format!("data graph parse error: {}", errors.join("; ")))?;

    Ok(Loaded {
        shapes: Arc::new(shapes),
        data,
    })
}

// ── The three lanes ───────────────────────────────────────────────────────────

/// Validate `data` with `shapes`, letting the shapes graph speak for its own
/// `sh:shapesGraph` IRI (`None` here means "do not override").
fn run_lane(shapes: &Shapes, data: &RdfDataset) -> LaneOutcome {
    let answer = catch_unwind(AssertUnwindSafe(|| {
        validate_dataset_with_shapes_graph(data, shapes, None)
    }))
    .unwrap_or_else(|payload| {
        let message = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("<non-string panic payload>");
        Err(format!("panicked: {message}"))
    });

    LaneOutcome {
        answer: answer.map(|report| (report.to_ntriples(), report.results.len())),
    }
}

/// Run one loaded case through all three lanes and decide its bucket.
fn classify(loaded: &Loaded) -> Bucket {
    let parsed = PreparedShapes::new(Arc::clone(&loaded.shapes));

    let bytes = match parsed.to_product(&ShapesProfile::CORE) {
        Ok(bytes) => bytes,
        Err(e) => return Bucket::Refused(e),
    };

    // Everything below this point is post-pack: the writer certified these bytes,
    // so a refusal here is a codec defect, not a ledgerable capability gap.
    //
    // The bytes are opened twice because each consuming entry point takes the view
    // by value. That is the right shape anyway: the two lanes are two independent
    // restores of one artifact, which is exactly how a consumer meets them.
    let host = HostBindings::empty();
    let open = |lane: &str| match ShapesProduct::open(&bytes) {
        Ok(view) => Ok(view),
        Err(e) => Err(Bucket::Disagreed(format!(
            "the product this build just wrote does not open for the {lane} lane: {e}"
        ))),
    };
    let admitted = match open("ADMIT").map(|view| view.admit(&ShapesProfile::CORE, &host)) {
        Ok(Ok(prepared)) => prepared,
        Ok(Err(e)) => {
            return Bucket::Disagreed(format!(
                "the product this build just wrote is not admitted by this build: {e}"
            ));
        }
        Err(bucket) => return bucket,
    };
    let rebuilt = match open("REBUILD").map(|view| view.rebuild(&ShapesProfile::CORE, &host)) {
        Ok(Ok(prepared)) => prepared,
        Ok(Err(e)) => {
            return Bucket::Disagreed(format!(
                "the product this build just wrote cannot be rebuilt from its own carried \
                 dataset: {e}"
            ));
        }
        Err(bucket) => return bucket,
    };

    let parse_lane = run_lane(&loaded.shapes, &loaded.data);
    let admit_lane = run_lane(admitted.shapes(), &loaded.data);
    let rebuild_lane = run_lane(rebuilt.shapes(), &loaded.data);

    if let Some(diff) = disagreement("PARSE", &parse_lane, "ADMIT", &admit_lane) {
        return Bucket::Disagreed(diff);
    }
    if let Some(diff) = disagreement("PARSE", &parse_lane, "REBUILD", &rebuild_lane) {
        return Bucket::Disagreed(diff);
    }
    Bucket::Agreed(parse_lane)
}

/// Describe how two lanes differ, or `None` when they agree exactly.
fn disagreement(
    left_name: &str,
    left: &LaneOutcome,
    right_name: &str,
    right: &LaneOutcome,
) -> Option<String> {
    if left == right {
        return None;
    }
    let mut message = format!(
        "the {left_name} lane and the {right_name} lane do not agree:\n    \
         {left_name}: {}\n    {right_name}: {}",
        left.summary(),
        right.summary()
    );
    if let (Ok((left_nt, _)), Ok((right_nt, _))) = (&left.answer, &right.answer) {
        message.push('\n');
        message.push_str(&line_diff(left_nt, right_nt));
    }
    Some(message)
}

/// The first few report lines each side has and the other does not.
///
/// Two multi-kilobyte N-Triples documents printed in full are unreadable, and the
/// part a reader needs is which RESULTS moved.
fn line_diff(left: &str, right: &str) -> String {
    let mut counts: BTreeMap<&str, i32> = BTreeMap::new();
    for line in left.lines() {
        *counts.entry(line).or_insert(0) += 1;
    }
    for line in right.lines() {
        *counts.entry(line).or_insert(0) -= 1;
    }
    let mut lines: Vec<String> = counts
        .into_iter()
        .filter(|(_, n)| *n != 0)
        .map(|(line, n)| {
            let side = if n > 0 { "only-left " } else { "only-right" };
            format!("    {side} x{}: {line}", n.abs())
        })
        .collect();
    let elided = lines.len().saturating_sub(12);
    lines.truncate(12);
    if elided > 0 {
        lines.push(format!("    … and {elided} more differing line(s)"));
    }
    lines.join("\n")
}

// ── The harness ───────────────────────────────────────────────────────────────

#[test]
fn product_corpus_equivalence() {
    // Discovery first, and from the shared corpus reader, so this harness and the
    // two conformance harnesses cannot be looking at different corpora.
    let mut cases: Vec<Case> = Vec::new();
    for case in w3c_cases() {
        cases.push(Case {
            id: format!("w3c/{}", case.id),
            refusal_is_declared: matches!(case.expected, Expected::Failure),
            loaded: load_w3c(&case),
        });
    }
    for case in first_party_cases() {
        cases.push(Case {
            id: format!("corpus/{}", case.name),
            refusal_is_declared: false,
            loaded: load_first_party(&case),
        });
    }
    assert_eq!(
        cases.len(),
        TOTAL_CASES,
        "discovered case count drifted — both corpora are frozen, so this means \
         discovery changed; update the counts in the shared corpus reader only on a \
         deliberate corpus change"
    );

    let refusal_ledger: BTreeMap<&str, (ProductDimension, &str)> = REFUSAL_LEDGER
        .iter()
        .map(|(id, dimension, reason)| (*id, (*dimension, *reason)))
        .collect();
    assert_eq!(
        refusal_ledger.len(),
        REFUSAL_LEDGER.len(),
        "duplicate entries in REFUSAL_LEDGER"
    );

    let mut errors: Vec<String> = Vec::new();
    // (case id, reason) for every bucket-1 case, printed with the scoreboard so
    // the bucket is visible rather than merely counted.
    let mut unloadable_cases: Vec<(&str, String)> = Vec::new();
    let mut unloadable = 0usize;
    let mut refused = 0usize;
    let mut agreed = 0usize;
    let mut agreed_on_report = 0usize;
    let mut agreed_with_results = 0usize;
    let mut disagreed = 0usize;
    // Which dimension each refusal named, for the scoreboard.
    let mut dimensions: BTreeMap<&'static str, usize> = BTreeMap::new();

    // Engine panics are caught per lane and reported as disagreements, so the
    // default hook's backtraces would only drown the scoreboard.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    for case in &cases {
        let id = case.id.as_str();
        let bucket = match &case.loaded {
            Err(reason) => Bucket::Unloadable(reason.clone()),
            Ok(loaded) => classify(loaded),
        };

        match bucket {
            Bucket::Unloadable(reason) => {
                unloadable += 1;
                unloadable_cases.push((id, reason.clone()));
                if !case.refusal_is_declared {
                    errors.push(format!(
                        "UNLOADABLE [{id}]: this case's RDF does not load, so the product lanes \
                         cannot be compared over it — and its corpus does not DECLARE that the \
                         validator must refuse it. Only a vendored case carrying \
                         `mf:result sht:Failure` may land here; everything else that stops \
                         loading is a regression that must not be absorbed into a bucket \
                         count. Reason: {reason}"
                    ));
                }
            }
            Bucket::Refused(error) => {
                refused += 1;
                *dimensions.entry(error.dimension().label()).or_insert(0) += 1;
                match refusal_ledger.get(id) {
                    None => errors.push(format!(
                        "OVER-REFUSAL [{id}]: this shapes graph loads but the product writer \
                         refused it on `{}`, and it is not in REFUSAL_LEDGER. A shapes graph \
                         that can no longer be packed is a shapes graph the product surface \
                         silently stopped covering — every remaining comparison still passes, \
                         which is exactly why this is an error and not a number. Refusal: {error}",
                        error.dimension()
                    )),
                    Some((expected, _)) if *expected != error.dimension() => {
                        errors.push(format!(
                            "LEDGER DIMENSION MOVED [{id}]: REFUSAL_LEDGER records `{expected}` \
                             but the writer refused on `{}`. The dimension is the part a caller \
                             branches on, so a change here is a change to the contract. \
                             Refusal: {error}",
                            error.dimension()
                        ));
                    }
                    Some(_) => {}
                }
            }
            Bucket::Agreed(outcome) => {
                agreed += 1;
                if let Ok((_, results)) = &outcome.answer {
                    agreed_on_report += 1;
                    if *results > 0 {
                        agreed_with_results += 1;
                    }
                }
                if let Some((dimension, reason)) = refusal_ledger.get(id) {
                    errors.push(format!(
                        "XPACK [{id}]: REFUSAL_LEDGER says the writer refuses this shapes graph \
                         on `{dimension}`, but it packs now and all three lanes agree. Remove the \
                         entry — a stale ledger hides the next real refusal. (reason was: {reason})"
                    ));
                }
            }
            Bucket::Disagreed(detail) => {
                disagreed += 1;
                errors.push(format!("DISAGREE [{id}]: {detail}"));
            }
        }
    }

    std::panic::set_hook(default_hook);

    // The single scrapable scoreboard line, plus the refusal ledger's shape.
    println!(
        "PRODUCT-EQUIVALENCE: passed {agreed} ledgered {refused} unparsable {unloadable} \
         disagreed {disagreed} total {}",
        cases.len()
    );
    println!(
        "  evidence: {agreed_on_report} agreed on a report, of which {agreed_with_results} \
         carried at least one validation result"
    );
    if dimensions.is_empty() {
        println!("  refusal dimensions: none — every loadable shapes graph packs");
    } else {
        for (dimension, count) in &dimensions {
            println!("  refusal dimension {dimension:<28} {count:>3}");
        }
    }
    for (id, reason) in &unloadable_cases {
        println!("  unparsable {id}: {reason}");
    }

    assert!(
        errors.is_empty(),
        "product_corpus_equivalence: {} error(s):\n{}",
        errors.len(),
        errors.join("\n\n")
    );

    // Exact-count gates. Every discovered case is in exactly one bucket, and the
    // partition must add up — a case that fell out of the loop entirely would
    // otherwise be invisible.
    assert_eq!(
        disagreed, 0,
        "a disagreement is always a hard failure and must have been reported above"
    );
    assert_eq!(
        unloadable, UNLOADABLE_CASES,
        "the unloadable bucket drifted; the cases in it are printed above"
    );
    assert_eq!(
        refused,
        REFUSAL_LEDGER.len(),
        "the refusal bucket must match its ledger exactly"
    );
    assert_eq!(
        agreed, AGREED_CASES,
        "every case that is neither unloadable nor ledgered as refused must have been \
         compared across all three lanes"
    );
    assert_eq!(
        unloadable + refused + agreed + disagreed,
        TOTAL_CASES,
        "the four buckets must partition the discovered cases"
    );

    // Anti-vacuity: a product lane that refused, or emptied, everything would
    // satisfy every equality above.
    assert_eq!(
        agreed_on_report, AGREED_ON_REPORT_CASES,
        "the number of cases that agreed on an actual report moved; three lanes agreeing on \
         an identical failure is agreement, but it is not evidence that any two reports were \
         compared"
    );
    assert_eq!(
        agreed_with_results, AGREED_WITH_RESULTS_CASES,
        "the number of agreed reports carrying at least one validation result moved; an \
         empty report is agreed on by any two lanes that both lost the shapes graph"
    );
}
