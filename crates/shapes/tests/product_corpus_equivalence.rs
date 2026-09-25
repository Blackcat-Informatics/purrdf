// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **Three ways to obtain a shapes graph must answer the same question the same
//! way, over every shapes graph the three SHACL corpora contain** — the vendored
//! W3C data-shapes suite, the `sht:Validate` entries of the vendored W3C SHACL 1.2
//! suite, and the first-party corpus.
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

use shacl_corpora::shacl12::{Body, shacl12_cases};
use shacl_corpora::{
    Expected, FIRST_PARTY_TOTAL_CASES, W3C_TOTAL_CASES, file_iri, first_party_box_role_vocab,
    first_party_cases, w3c_cases,
};

/// The `sht:Validate` entries of the vendored W3C SHACL 1.2 suite — the third
/// corpus whose shapes graphs the product must carry.
const W3C12_VALIDATE_CASES: usize = 174;

/// Every case the three corpora contribute. Asserted exactly, so a corpus that grew
/// or shrank without this harness noticing fails rather than quietly measuring less.
const TOTAL_CASES: usize = W3C_TOTAL_CASES + FIRST_PARTY_TOTAL_CASES + W3C12_VALIDATE_CASES;

// ── Bucket 1: cases whose own RDF does not load ────────────────────────────────

/// The exact number of cases whose shapes graph or data graph does not load.
///
/// Two kinds of case land here, and both are declared rather than discovered:
///
/// * a case the vendored manifest itself declares `mf:result sht:Failure` — an
///   input the validator is REQUIRED to reject. The SHACL 1.0 suite has exactly
///   seven, all under `sparql/pre-binding/`, each carrying a `sh:sparql` body that
///   SHACL's pre-binding rules forbid; the SHACL 1.2 suite has
///   [`W3C12_DECLARED_FAILURES`];
/// * a SHACL 1.2 case named in [`W3C12_REFUSED_AT_LOAD`], with its reason.
///
/// So this bucket is not an excuse list, and `Case::refusal_is_declared` enforces
/// that correspondence case by case rather than trusting the number.
///
/// The count is asserted in addition to the rule, because the rule alone would
/// be satisfied by a parser that had started refusing NOTHING at all.
const UNLOADABLE_CASES: usize = 7 + W3C12_DECLARED_FAILURES + W3C12_REFUSED_AT_LOAD.len();

/// The W3C SHACL 1.2 `sht:Validate` entries whose manifest declares
/// `mf:result sht:Failure` — inputs a validator must reject, admitted to the
/// unloadable bucket by the same rule as the seven SHACL 1.0 ones.
const W3C12_DECLARED_FAILURES: usize = 5;

/// W3C SHACL 1.2 `sht:Validate` entries whose shapes graph this engine REFUSES at
/// load, each with the reason — SHACL 1.2 features `w3c12_conformance.rs` ledgers
/// as unevaluated, where the engine's answer is a load error rather than a report. There is no shapes graph to pack, so the codec has nothing to say
/// about them. The ledger runs both ways: an entry whose shapes graph starts
/// loading fails the suite, and so does an unledgered 1.2 case that stops loading.
const W3C12_REFUSED_AT_LOAD: &[(&str, &str)] = &[
    (
        "w3c12/sparql/functions/instanceCount-example",
        R_INSTANCES_OF_EXPR,
    ),
    (
        "w3c12/inference-rules/rules-entailment-validation",
        R_ENTAILMENT,
    ),
];

const R_ENTAILMENT: &str = "declares sh:entailment, and SHACL requires a processor to signal a \
     failure for an entailment regime it does not support";

const R_INSTANCES_OF_EXPR: &str = "a custom function body passes a node-expression argument to \
     shnex:instancesOf, which the parser refuses because it requires an IRI";

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
/// Moved from 192 when the first-party corpus gained its relation-reaching
/// `sh:SPARQLFunction` case; see [`AGREED_WITH_RESULTS_CASES`] for why that case's
/// arrival is visible in two counts rather than one.
///
/// Moved from 193 to 338 when two things arrived together: the first-party case
/// whose shapes graph carries the W3C vocabulary's own declarations of three
/// built-ins (+1), and the 144 `sht:Validate` entries of the W3C SHACL 1.2 suite
/// whose shapes graphs load (+144 — its 174 entries minus the 5 declared failures
/// and the 25 [`W3C12_REFUSED_AT_LOAD`] entries). Every one of them agreed on a
/// report, including the two whose custom list function is called only from
/// SPARQL text and which the product writer used to refuse, because the model does
/// not carry an uncalled declaration and the restore did not re-derive it.
///
/// Moved from 338 to 335 when the shapes-graph census reached the loader: the ten
/// SHACL 1.2 list-component cases now load and agree (+10), and thirteen cases now
/// stop at load on a term the engine does not evaluate (−13), so
/// [`W3C12_REFUSED_AT_LOAD`] went from 25 entries to 28.
///
/// Moved from 335 to 338 when `sh:singleLine`, `sh:rootClass` and `sh:someValue`
/// became evaluated components: `singleLine-001`, `rootClass-001` and
/// `someValue-001` now load and agree on a report (+3), and
/// [`W3C12_REFUSED_AT_LOAD`] went from 28 entries to 25.
///
/// Moved from 338 to 344 when the property-pair components took any SHACL
/// property path and `sh:subsetOf` became evaluated: `equals-002`,
/// `disjoint-002`, `lessThan-003`, `lessThanOrEquals-002`, `subsetOf-001` and
/// `subsetOf-002` now load and agree on a report (+6), and
/// [`W3C12_REFUSED_AT_LOAD`] went from 25 entries to 19.
///
/// Moved from 344 to 349 when `sh:uniqueValuesFor` became evaluated:
/// `uniqueValuesFor-001` to `-005` now load and agree on a report (+5), and
/// [`W3C12_REFUSED_AT_LOAD`] went from 19 entries to 14.
///
/// Moved from 349 to 351 when `sh:closed sh:ByTypes` became evaluated:
/// `closed-003` and `closed-004` now load and agree on a report (+2), and
/// [`W3C12_REFUSED_AT_LOAD`] went from 14 entries to 12.
///
/// Moved from 351 to 356 when per-constraint reifier annotations and the
/// `sh:Debug` / `sh:Trace` severities became evaluated: `deactivated-003`,
/// `severity-003`, `severity-004`, `severity-005` and `message-002` now load and
/// agree on a report (+5) — the annotation list and the two new severities travel
/// in the product — and [`W3C12_REFUSED_AT_LOAD`] went from 12 entries to 7.
///
/// Moved from 356 to 359 when implicit class targets, `sh:ShapeClass`,
/// `sh:targetWhere` and node-expression `sh:targetNode` values became evaluated:
/// `targetClassImplicit-002`, `targetWhere-001` and `targetNode-select-001` now
/// load and agree on a report (+3) — the two new target kinds travel in the
/// product — and [`W3C12_REFUSED_AT_LOAD`] went from 7 entries to 4.
///
/// Moved from 359 to 361 when `sh:values` and `sh:defaultValue` became evaluated:
/// `property-select-001` and `property-sparqlExpr-001` now load and agree on a
/// report (+2) — a property shape's two node expressions travel in the product —
/// and [`W3C12_REFUSED_AT_LOAD`] went from 4 entries to 2.
const AGREED_ON_REPORT_CASES: usize = 361;

/// The exact number of agreed cases whose shared report carries at least one
/// validation result.
///
/// An empty report is agreed on by any two lanes that both found nothing,
/// including two lanes that lost the entire shapes graph in the same way. This is
/// the count that says the three lanes found the SAME violations at the same
/// focus nodes; the remaining agreed cases are the ones the corpus expects to
/// conform.
///
/// # Why it moved from 179
///
/// The first-party corpus gained a case whose `sh:SPARQLFunction` body reaches the
/// host-registered corpus relation, and this harness now installs that relation for
/// the whole run.
///
/// The move IS the evidence. Before the relation was installed, the case still
/// AGREED across all three lanes — on an empty report, because the call lowered to an
/// ordinary triple pattern, matched nothing, and every focus node scored the same.
/// Three lanes agreeing on the answer a resolved relation would never give is exactly
/// the failure this count exists to catch, and the count is what caught it: the case
/// counted toward `AGREED_ON_REPORT_CASES` and not toward this one. It now counts
/// toward both, because the relation resolved and produced the violation the corpus
/// expects.
///
/// # Why it moved from 180 to 310
///
/// The same two arrivals as [`AGREED_ON_REPORT_CASES`]: the first-party
/// built-in-declarations case reports three violations (+1), and 129 of the 144
/// loadable SHACL 1.2 `sht:Validate` entries expect at least one result (+129) —
/// the other 15 expect conformance and are agreed on as empty reports, which is
/// why the two counts moved by different amounts.
///
/// # Why it moved from 310 to 316
///
/// The shapes-graph census reached the loader. The ten SHACL 1.2 list-component
/// cases (`sh:minListLength`, `sh:maxListLength`, `sh:uniqueMembers`,
/// `sh:memberShape`, and the two that reach `sh:minListLength` through their
/// shacl-shacl property shapes) now load, and every one of them reports results
/// (+10); thirteen other cases now stop at load on a term the engine does not
/// evaluate (see [`W3C12_REFUSED_AT_LOAD`]), and four of those had agreed on a
/// report carrying a result (−4).
///
/// # Why it moved from 316 to 319
///
/// `singleLine-001`, `rootClass-001` and `someValue-001` load now that their
/// components are evaluated, and each expects — and reports — violations (+3).
///
/// # Why it moved from 319 to 325
///
/// `equals-002`, `disjoint-002`, `lessThan-003`, `lessThanOrEquals-002`,
/// `subsetOf-001` and `subsetOf-002` load now that a property pair takes any
/// SHACL property path and `sh:subsetOf` is evaluated, and each expects — and
/// reports — violations (+6).
///
/// # Why it moved from 325 to 329
///
/// `uniqueValuesFor-001`, `-002`, `-003` and `-005` load now that the component
/// is evaluated, and each expects — and reports — violations (+4);
/// `uniqueValuesFor-004` expects conformance and is agreed on as an empty report.
///
/// # Why it moved from 329 to 331
///
/// `closed-003` and `closed-004` load now that `sh:closed sh:ByTypes` is
/// evaluated, and each expects — and reports — a violation (+2).
///
/// # Why it moved from 331 to 335
///
/// `severity-003` (a `sh:Warning` result from a reifier `sh:severity`),
/// `severity-004` (a `sh:Debug` result), `severity-005` (a `sh:Trace` result) and
/// `message-002` (a violation carrying the reifier `sh:message`) load now and each
/// reports its result (+4); `deactivated-003`, whose only constraints a reifier
/// deactivates, is agreed on as an empty report.
///
/// # Why it moved from 335 to 339
///
/// `targetClassImplicit-002`, `targetWhere-001` and `targetNode-select-001` load
/// now that their targets are evaluated, and each expects — and reports — a
/// violation (+3). `shape-001` loaded before, and every lane agreed on an EMPTY
/// report, because the data graph's `sh:shape` declarations selected no focus
/// node; they select its two focus nodes now, and the lanes agree on the
/// violation the suite expects (+1).
///
/// # Why it moved from 339 to 340
///
/// `property-sparqlExpr-001` loads now that `sh:values` is evaluated, and its
/// computed URI length fails `sh:hasValue 27` at `ex:Invalid` — the violation the
/// suite expects (+1). `property-select-001` loads too and is agreed on as an
/// empty report: its computed full name satisfies `sh:hasValue "John Muir"`.
///
/// # Why it moved from 340 to 341
///
/// `sparql/node/prefixes-002` loaded before, and every lane agreed on an EMPTY
/// report: a `PREFIX test:` line quoted inside a SECOND constraint's `sh:select`
/// was read as a document prefix and rebound `test:` for the first constraint too,
/// so its `FILTER (?value = test:Value)` matched nothing. The document prefix map
/// is now the Turtle codec's own record, and the first constraint takes `test:` from
/// the `sh:ShapesGraph`'s implicit `sh:declare`; the lanes agree on the violation
/// the suite expects (+1).
const AGREED_WITH_RESULTS_CASES: usize = 341;

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
    let purrdf_shapes::text_ingest::TurtleDocument {
        dataset: shapes_dataset,
        prefixes: doc_prefixes,
        ..
    } = purrdf_shapes::text_ingest::parse_turtle_document(
        &shapes_text,
        Some(&file_iri(&case.shapes_path)),
    )
    .map_err(|errors| format!("shapes graph parse error: {}", errors.join("; ")))?;
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
    // The corpus relation, installed for the whole run. The first-party corpus has a
    // case whose `sh:SPARQLFunction` body reaches it, and this harness must grade that
    // case under the SAME environment `conformance.rs` does — otherwise the three
    // lanes here would agree with each other on an answer the conformance harness
    // disagrees with, which is agreement without correctness.
    //
    // It changes nothing about ADMISSION: a product's identity is compared against the
    // `HostBindings` handed to `admit`, not against this ambient scope, so a product
    // written with `to_product` still restores under the empty host exactly as before.
    // The scope affects only what the validations then read.
    let (corpus_relations, corpus_opens) = shacl_corpora::corpus_relations();
    let _relations = purrdf_shapes::sparql::enter_property_function_scope(corpus_relations);
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
    for case in shacl12_cases() {
        let Body::Validate(case) = case.body else {
            continue;
        };
        let id = format!("w3c12/{}", case.id);
        cases.push(Case {
            refusal_is_declared: matches!(case.expected, Expected::Failure)
                || W3C12_REFUSED_AT_LOAD
                    .iter()
                    .any(|(ledgered, _)| *ledgered == id),
            loaded: load_w3c(&case),
            id,
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
                if W3C12_REFUSED_AT_LOAD
                    .iter()
                    .any(|(ledgered, _)| *ledgered == id)
                {
                    errors.push(format!(
                        "XLOAD [{id}]: W3C12_REFUSED_AT_LOAD says this shapes graph is refused \
                         at load, but it loads, packs and agrees now; remove the entry"
                    ));
                }
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
    assert!(
        corpus_opens.load(std::sync::atomic::Ordering::Relaxed) > 0,
        "no case reached the corpus relation, so this harness is grading the relation \
         case over an empty environment while `conformance.rs` grades it over a \
         resolved call — three lanes agreeing here would prove nothing about it",
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
