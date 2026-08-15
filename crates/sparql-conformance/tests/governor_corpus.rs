// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Normative vector corpus for the execution-governor profile `purrdf-sparql-governors`.
//!
//! The corpus at `vectors/sparql-governors/` is the executable half of the profile a
//! consumer pins. A fuel budget is a number about *this* charge schedule, so a consumer
//! that pins `(profile id, profile version, profile digest, corpus digest)` needs a body
//! of evidence saying what those numbers actually buy — otherwise the identity names a
//! schedule nobody can reproduce.
//!
//! # What each case pins, and why it is three separate records
//!
//! Every case pins three facts a naive corpus would collapse into one:
//!
//! 1. **the outcome discriminant** — completed, or which governor stopped it
//!    (`manifest.tsv`'s `outcome` column, spelled in the kernel's own
//!    [`TrippedGovernor::label`] vocabulary);
//! 2. **the certified rows and their certificate class** (`expected/<case>.answer`) —
//!    what the caller receives, and what those rows are licensed to be used as;
//! 3. **what the execution spent** (`expected/<case>.spend`), pinned INDEPENDENTLY of
//!    the rows.
//!
//! The third record is the one easy to leave out and impossible to reconstruct afterwards.
//! A corpus that pins only rows cannot detect a charge-schedule change that happens to cut
//! in the same place: the answer is unchanged, the receipt is not, and every budget a
//! consumer sized against the old schedule is silently wrong while the corpus stays green.
//! So consumption is recorded on its own, per dimension, and compared on its own.
//!
//! # The boundary is measured, never guessed
//!
//! A "boundary" case whose ceiling was typed in by hand tests whatever its author
//! believed. Each numeric case therefore also carries `expected/<case>.metered`: the
//! consumption vector of the *same* case run under [`QueryGovernors::METERED`], which
//! engages every counter and bounds nothing. The injected-deadline cases analogously
//! measure the complete run's stop-signal poll count with a never-firing signal. A
//! `boundary` ceiling must equal that measurement exactly and an `over-bound` ceiling
//! exactly one less — and the relation is re-derived and re-checked on every run rather
//! than trusted. A charge- or poll-schedule change therefore cannot leave a stale
//! boundary behind looking authoritative.
//!
//! # Deterministic deadline polling, but not wall time
//!
//! Three cases inject a deterministic deadline signal whose only input is its poll count.
//! They pin zero, boundary and over-bound behavior just like the numeric dimensions, so a
//! moved stop-poll site is visible. A real wall deadline is time-dependent and carries no
//! determinism claim, so the separate wall-clock smoke case pins exactly what is guaranteed
//! — that a trip happened and that it named the deadline — and carries no rows, no spend
//! and no metered cost. Pinning bytes for that case would publish a promise this engine does
//! not make.
//!
//! # The relation lane
//!
//! Several cases wire a **scripted property-function relation** instead of a remote
//! endpoint. It is the second producer whose bag size an outside party picks, so it owes
//! the same two records the transport lane owes: how many invocations host code was
//! entered for, and how many calls to `PfCursor::next` it received (the terminating,
//! exhausted call included, so this is not a row count). Neither is derivable from
//! governor evidence, and both are what separate "the ceiling prevented the work" from
//! "the work was done and its rows discarded". `relations.tsv` holds them.
//!
//! It also owes a fourth record no other lane needs. Fuel is one number, and v5 of the
//! schedule put **two** new charge points inside it — `property-function-invocation` and
//! `property-function-row`. A band on the aggregate is satisfied by any schedule whose
//! total happens to land in the same place, so it pins the pair rather than either point.
//! Each relation case therefore also carries `expected/<case>.charges`: the metered run's
//! fuel decomposed **per charge point**, read off the public per-node ledger
//! ([`QueryExplanation::ledger`]), whose own totals must add back up to the `.metered`
//! fuel. Two of the lanes then exist purely to separate the two points: an
//! *invocation-isolating* pair whose relation emits nothing at all (many invocations, zero
//! row charges) and a *row-isolating* pair driven by a single row (one invocation, many
//! row charges). A cost change at either point moves exactly one of them.
//!
//! # The aggregate lane
//!
//! v6 put a second pair of charge points inside the same fuel number —
//! `aggregate-invocation` and `aggregate-accumulation` — for the evaluator's third
//! producer whose per-group work an outside party sizes: `GROUP BY`'s aggregate fold,
//! built-in and custom alike. It owes the same fourth record the relation lane does, taken
//! the same way but over a plain `explain_query` (no property-function registry needed,
//! since an aggregate is ordinary algebra rather than an injected seam): `aggregate-invocation-fuel-*`
//! isolates the invocation point with many aggregate expressions folding an empty group
//! (many invocation charges, zero accumulation charges), and `aggregate-accumulation-fuel-*`
//! isolates the accumulation point with one aggregate expression folding many rows (one
//! invocation charge, many accumulation charges) — the same isolating-pair shape the
//! relation lane uses, for the same reason.
//!
//! Every case above names the built-in `SUM`. `Source::Aggregate` wires a registered
//! [`CustomAggregate`](purrdf_sparql_eval::CustomAggregate) instead
//! ([`registered_custom_aggregate`]), through `QueryOptions::aggregates` — the one seam
//! `explain_query`/`explain_query_with_property_functions` cannot express (there is no
//! `explain_query_with_aggregates`: see `NativeSparqlEngine::explain_for`'s doc comment in
//! the evaluator crate), so a `Source::Aggregate` case carries no `.charges` decomposition,
//! only `.answer`/`.spend`/`.metered` like an ordinary band. `aggregate-custom-fuel-*` folds
//! the SAME group shape `aggregate-accumulation-*` does through the custom path instead of
//! the built-in one, so the two lanes' fuel is directly comparable
//! ([`a_custom_aggregate_costs_the_same_fuel_as_a_built_in_over_the_same_group_shape`]).
//! `aggregate-custom-scratch-bytes-*` bands the ONE dimension no built-in aggregate ever
//! charges at all — [`CustomAggregate::state_bound`](purrdf_sparql_eval::CustomAggregate::state_bound)
//! — over a group large enough to cross the within-group chunk threshold, freezing the claim
//! that this charge (and the fold's answer) is a pure function of the group's row count,
//! never of the host's thread count
//! (see [`the_custom_aggregate_scratch_bytes_band_exercises_the_folds_retained_state`]).
//!
//! # The parallel drive
//!
//! One relation pair drives more rows than `parallel::PARALLEL_MIN_ROWS`, so the BGP
//! feeding the call folds its rows through the rayon chunk driver. Its band is the parity
//! statement, and it needs no test-only seam to make it: `QueryGovernors::METERED` engages
//! the intermediate-cell counter, which routes the measuring run through the **sequential**
//! cell-bounded driver, while the band run sets fuel alone and takes the **parallel** one.
//! A boundary that is exactly the metered cost — and an over-bound that is one less and
//! must trip — is therefore the claim "the two drivers spend the same, and stop at the same
//! item", stated as a frozen vector instead of as prose.
//!
//! # Regenerating
//!
//! ```sh
//! PURRDF_UPDATE_GOVERNOR_CORPUS=1 \
//!   cargo test -p purrdf-sparql-conformance --test governor_corpus
//! python3 scripts/check-corpus-frozen.py --update
//! # then re-pin GOVERNOR_CORPUS_DIGEST from the freeze manifest's sha256sum
//! ```
//!
//! Three steps on purpose: a regeneration that moved a charge is a profile-version change,
//! and the friction is what stops it being mistaken for a no-op.
//!
//! [`TrippedGovernor::label`]: purrdf_core::TrippedGovernor::label
//! [`QueryGovernors::METERED`]: purrdf_sparql_eval::QueryGovernors::METERED
//! [`QueryExplanation::ledger`]: purrdf_sparql_eval::QueryExplanation::ledger

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

use purrdf_core::{
    GovernorEvidence, RdfDataset, RdfTextDirection, ResourceDimension, SparqlRequest, SparqlResult,
    StopCause, TermValue, TrippedGovernor,
};
use purrdf_sparql_eval::{
    AggregateAccumulator, AggregateRegistry, AlgebraicClass, Arity, BindingPattern,
    CancellationFlag, ChargePoint, CustomAggregate, EvalError, GovernedOutcome,
    HttpRemoteQuerySource, HttpRequest, HttpTransport, NativeSparqlEngine, PartialAnswers, PfArgs,
    PfArity, PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry, QueryGovernors,
    QueryOptions, RemoteError, StopSignal, Volatility, WallDeadline,
};

/// The dimensions a case may set a ceiling on, in the order every pinned consumption
/// record lists them.
///
/// Spelled through [`ResourceDimension::label`](purrdf_core::ResourceDimension::label), so
/// a consumer reading a `.spend` file and a consumer reading a [`GovernorEvidence`] are
/// reading one vocabulary rather than two that happen to agree today.
const PINNED_DIMENSIONS: [ResourceDimension; 5] = [
    ResourceDimension::Fuel,
    ResourceDimension::AnswerRows,
    ResourceDimension::IntermediateCells,
    ResourceDimension::ScratchBytes,
    ResourceDimension::RemoteRequests,
];

/// The floor on the zero/boundary/over-bound matrix: one of each band for each of the five
/// numeric dimensions (15), an injected deterministic deadline (3), the property-function
/// charge points' joint fuel lane (3), and the three relation lanes that exist to take that
/// joint lane apart — invocation-isolating, row-isolating, and the parallel drive, a
/// boundary and an over-bound each (6) — plus the two aggregate lanes that isolate
/// `aggregate-invocation` and `aggregate-accumulation` from each other, a boundary and an
/// over-bound each (4) — plus the two `Source::Aggregate` lanes that exercise the CUSTOM
/// path those four never touch: `aggregate-custom-fuel-*`, the same group shape as
/// `aggregate-accumulation-*` but folded through a registered
/// [`CustomAggregate`](purrdf_sparql_eval::CustomAggregate) instead of the built-in `SUM`,
/// so the two lanes' `.metered` fuel is directly comparable
/// ([`a_custom_aggregate_costs_the_same_fuel_as_a_built_in_over_the_same_group_shape`]), and
/// `aggregate-custom-scratch-bytes-*`, whose 1200-row single implicit group crosses the
/// within-group chunk threshold and bands the custom accumulator's declared
/// `ScratchBytes` state bound — a boundary and an over-bound each (4).
const REQUIRED_BAND_CASES: usize = 35;

/// The driving-row count above which the evaluator's chunk drivers fork.
///
/// A copy of `purrdf_sparql_eval`'s crate-private `parallel::PARALLEL_MIN_ROWS`, which is
/// deliberately not public: it is a scheduling threshold, and a schedule change that made
/// it *observable* in a receipt would be the bug this corpus exists to catch. The copy
/// exists so [`the_parallel_drive_is_actually_above_the_fork_threshold`] can state what the
/// parallel lane's fixture is for. Raising the real constant above this value does not make
/// any case wrong — it makes the parallel pair an ordinary band pair, and that test is what
/// says so out loud instead of leaving the lane quietly inert.
const PARALLEL_FORK_MIN_ROWS: usize = 1024;

// ---------------------------------------------------------------------------
// Corpus locations
// ---------------------------------------------------------------------------

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vectors/sparql-governors")
}

fn updating() -> bool {
    std::env::var_os("PURRDF_UPDATE_GOVERNOR_CORPUS").is_some()
}

// ---------------------------------------------------------------------------
// The manifest
// ---------------------------------------------------------------------------

/// Which side of a ceiling a case sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Band {
    /// The ceiling is zero: valid, and admitting no charged work at all.
    Zero,
    /// The ceiling IS the metered cost, so the execution must complete.
    Boundary,
    /// The ceiling is one below the metered cost, so the execution must trip.
    OverBound,
    /// Not a band case — a stop-signal case, or a ceiling chosen to expose a seam.
    NotApplicable,
}

impl Band {
    fn parse(token: &str) -> Self {
        match token {
            "zero" => Self::Zero,
            "boundary" => Self::Boundary,
            "over-bound" => Self::OverBound,
            "n/a" => Self::NotApplicable,
            other => panic!("manifest.tsv: unknown band {other:?}"),
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Zero => "zero",
            Self::Boundary => "boundary",
            Self::OverBound => "over-bound",
            Self::NotApplicable => "n/a",
        }
    }
}

/// The stop signal a case attaches, spelled as a manifest token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StopSpec {
    /// A cancellation flag that has ALREADY fired before the query starts.
    Cancelled,
    /// A cancellation flag that has not fired, and that an injected transport can fire
    /// mid-execution.
    Cancellation,
    /// A zero-budget wall deadline: expired on its first poll, by construction.
    WallDeadlineZero,
}

impl StopSpec {
    fn parse(token: &str) -> Self {
        match token {
            "cancelled" => Self::Cancelled,
            "cancellation" => Self::Cancellation,
            "deadline-zero" => Self::WallDeadlineZero,
            other => panic!("manifest.tsv: unknown stop signal {other:?}"),
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::Cancellation => "cancellation",
            Self::WallDeadlineZero => "deadline-zero",
        }
    }
}

/// Where a case's rows come from, beyond the dataset it loads.
///
/// A fourth value rather than a third: the injected seams — a remote endpoint, a host
/// relation, and a host **aggregate** — are the producers whose bag size (or, for the
/// aggregate, whose per-accumulator retained state) an outside party picks, and a case
/// wires at most one of them. The aggregate seam needs no per-case sidecar table the way
/// the transport and relation seams do — see [`registered_custom_aggregate`] — because it
/// is deterministic by construction (`Volatility::Stable`, a pure fold over its rows'
/// already-bound argument tuples), so nothing about a run needs to be counted and pinned
/// beyond what `.answer`/`.spend`/`.metered` already record for every other lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    /// The dataset and nothing else.
    Dataset,
    /// The injected HTTP transport described in `transport.tsv`.
    Http,
    /// The scripted property-function relation described in `relations.tsv`.
    Relation,
    /// The registered custom aggregate from [`registered_custom_aggregate`], wired through
    /// `QueryOptions::aggregates` rather than through an injected graph-pattern seam.
    Aggregate,
}

impl Source {
    fn parse(token: &str, line: usize) -> Self {
        match token {
            "none" => Self::Dataset,
            "http" => Self::Http,
            "relation" => Self::Relation,
            "aggregate" => Self::Aggregate,
            other => panic!("manifest.tsv line {line}: unknown source {other:?}"),
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Dataset => "none",
            Self::Http => "http",
            Self::Aggregate => "aggregate",
            Self::Relation => "relation",
        }
    }
}

/// One manifest row.
#[derive(Debug, Clone)]
struct Case {
    name: String,
    data: String,
    query: String,
    /// Which injected seam, if any, this case wires.
    source: Source,
    /// The ceiling this case sets, if it sets one.
    ceiling: Option<(ResourceDimension, u64)>,
    /// The stop signal this case attaches, if it attaches one.
    stop: Option<StopSpec>,
    /// Polls a deterministic injected deadline admits before it fires.
    deadline_polls: Option<u64>,
    band: Band,
    /// The pinned outcome discriminant.
    outcome: String,
}

impl Case {
    /// Whether this case pins rows, spend, and a metered cost.
    ///
    /// Every case does except the deadline one: a wall deadline trip is time-dependent, so
    /// the only honest pinned fact about it is that it happened and what it named.
    fn is_pinned_deterministic(&self) -> bool {
        self.stop != Some(StopSpec::WallDeadlineZero)
    }
}

/// The caller-settable dimension a ceiling token names.
fn dimension_for(token: &str) -> ResourceDimension {
    PINNED_DIMENSIONS
        .into_iter()
        .find(|dimension| dimension.label() == token)
        .unwrap_or_else(|| panic!("manifest.tsv: {token:?} is not a caller-settable dimension"))
}

/// Split a `governors` cell into its ceiling and its stop signal.
///
/// A cell names at most one ceiling and at most one signal: a case setting two ceilings
/// would be measuring precedence rather than a boundary, and precedence is tested where a
/// scripted clock can drive it.
fn parse_governors(
    cell: &str,
    line: usize,
) -> (
    Option<(ResourceDimension, u64)>,
    Option<StopSpec>,
    Option<u64>,
) {
    let mut ceiling = None;
    let mut stop = None;
    let mut deadline_polls = None;
    for setting in cell.split(',') {
        let (key, value) = setting
            .split_once('=')
            .unwrap_or_else(|| panic!("manifest.tsv line {line}: malformed setting {setting:?}"));
        if key == "stop" {
            assert!(stop.is_none(), "manifest.tsv line {line}: two stop signals");
            stop = Some(StopSpec::parse(value));
        } else if key == "deadline-polls" {
            assert!(
                deadline_polls.is_none(),
                "manifest.tsv line {line}: two injected deadlines"
            );
            deadline_polls = Some(if value == "?" {
                u64::MAX
            } else {
                value.parse().unwrap_or_else(|error| {
                    panic!(
                        "manifest.tsv line {line}: {value:?} is not a deadline poll ceiling: \
                         {error}"
                    )
                })
            });
        } else {
            assert!(ceiling.is_none(), "manifest.tsv line {line}: two ceilings");
            // `?` is the placeholder a regeneration replaces with the measured value. It is
            // never valid in a committed manifest, and the only run that reads one is the
            // regeneration that is about to overwrite it.
            let amount = if value == "?" {
                u64::MAX
            } else {
                value.parse().unwrap_or_else(|error| {
                    panic!("manifest.tsv line {line}: {value:?} is not a ceiling: {error}")
                })
            };
            ceiling = Some((dimension_for(key), amount));
        }
    }
    assert!(
        stop.is_none() || deadline_polls.is_none(),
        "manifest.tsv line {line}: wall and injected deadlines are mutually exclusive"
    );
    (ceiling, stop, deadline_polls)
}

/// Parse `manifest.tsv`. Blank lines and `#` comments are skipped; every other line must
/// carry exactly seven tab-separated fields, because a manifest that tolerates a malformed
/// row is one that can silently drop a case.
fn load_manifest() -> Vec<Case> {
    let text =
        std::fs::read_to_string(corpus_root().join("manifest.tsv")).expect("corpus manifest");
    let mut cases = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let number = index + 1;
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(
            fields.len(),
            7,
            "manifest.tsv line {number} is malformed: {line:?}"
        );
        let source = Source::parse(fields[3], number);
        let (ceiling, stop, deadline_polls) = parse_governors(fields[4], number);
        assert!(
            ceiling.is_some() || stop.is_some() || deadline_polls.is_some(),
            "manifest.tsv line {number}: a case must declare a governor"
        );
        cases.push(Case {
            name: fields[0].to_owned(),
            data: fields[1].to_owned(),
            query: fields[2].to_owned(),
            source,
            ceiling,
            stop,
            deadline_polls,
            band: Band::parse(fields[5]),
            outcome: fields[6].to_owned(),
        });
    }
    assert!(!cases.is_empty(), "the corpus manifest is empty");
    cases
}

/// The case with this name, or a failure that says which name the corpus owes.
fn case_named<'a>(cases: &'a [Case], name: &str) -> &'a Case {
    cases
        .iter()
        .find(|case| case.name == name)
        .unwrap_or_else(|| panic!("the corpus must carry a case named {name}"))
}

// ---------------------------------------------------------------------------
// The transport table
// ---------------------------------------------------------------------------

/// One `transport.tsv` row: how the injected transport behaves, and how many exchanges it
/// must be asked to perform.
#[derive(Debug, Clone, Copy)]
struct TransportSpec {
    /// Whether the transport polls [`HttpRequest::stop`] and abandons on a fired signal.
    honours_stop: bool,
    /// Whether the transport cancels the caller's flag during its first exchange.
    cancel_on_first_post: bool,
    /// The exact number of exchanges expected. `None` is the regeneration placeholder.
    posts: Option<usize>,
}

impl TransportSpec {
    const fn stop_handling(self) -> &'static str {
        if self.honours_stop {
            "honours"
        } else {
            "ignores"
        }
    }

    const fn on_first_post(self) -> &'static str {
        if self.cancel_on_first_post {
            "cancel"
        } else {
            "nothing"
        }
    }
}

fn load_transport() -> BTreeMap<String, TransportSpec> {
    let text =
        std::fs::read_to_string(corpus_root().join("transport.tsv")).expect("transport table");
    let mut specs = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let number = index + 1;
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(
            fields.len(),
            4,
            "transport.tsv line {number} is malformed: {line:?}"
        );
        let honours_stop = match fields[1] {
            "honours" => true,
            "ignores" => false,
            other => panic!("transport.tsv line {number}: unknown stop-handling {other:?}"),
        };
        let cancel_on_first_post = match fields[2] {
            "nothing" => false,
            "cancel" => true,
            other => panic!("transport.tsv line {number}: unknown on-first-post {other:?}"),
        };
        let posts = if fields[3] == "?" {
            None
        } else {
            Some(fields[3].parse().unwrap_or_else(|error| {
                panic!(
                    "transport.tsv line {number}: {:?} is not a count: {error}",
                    fields[3]
                )
            }))
        };
        let previous = specs.insert(
            fields[0].to_owned(),
            TransportSpec {
                honours_stop,
                cancel_on_first_post,
                posts,
            },
        );
        assert!(
            previous.is_none(),
            "transport.tsv line {number}: {:?} is listed twice",
            fields[0]
        );
    }
    specs
}

// ---------------------------------------------------------------------------
// The relation table
// ---------------------------------------------------------------------------

/// The predicate IRI every relation-lane case's query calls, and the IRI the scripted
/// relation is registered under.
///
/// A fixture IRI under `example.org`, exactly as every other fixture in this corpus is:
/// the property-function seam is caller configuration, so the corpus supplies its own
/// vocabulary rather than depending on one the engine mints.
const RELATION_IRI: &str = "http://example.org/pf/emit";

/// The subject value every emitted row echoes back into is the invocation's own bound
/// subject, so a `bf` invocation's rows survive the engine's bound-position filter.
const RELATION_OBJECT_PREFIX: &str = "http://example.org/pf/r";

/// One `relations.tsv` row: how the scripted relation behaves, and how much work it must
/// be asked to perform.
#[derive(Debug, Clone, Copy)]
struct RelationSpec {
    /// Rows the relation emits per invocation, which is also the bound it declares —
    /// the declaration is held to an upper-bound honesty contract, and the corpus keeps
    /// it exact so a `rows_per_invocation` change is visible as a spend change.
    emits: u64,
    /// Whether the cursor polls the caller's stop signal and abandons the invocation on a
    /// fired one — what a relation CAN do, never what makes the query bounded.
    honours_stop: bool,
    /// The 1-based pull at which the cursor fires the caller's cancellation flag, standing
    /// in for a host that cancels while the evaluator is inside a relation and can poll
    /// nothing.
    cancel_on_pull: Option<u64>,
    /// The exact number of invocations expected. `None` is the regeneration placeholder.
    invocations: Option<u64>,
    /// The exact number of row pulls expected. `None` is the regeneration placeholder.
    pulls: Option<u64>,
}

impl RelationSpec {
    const fn stop_handling(self) -> &'static str {
        if self.honours_stop {
            "honours"
        } else {
            "ignores"
        }
    }

    fn cancel_cell(self) -> String {
        self.cancel_on_pull
            .map_or_else(|| "never".to_owned(), |pull| pull.to_string())
    }
}

fn load_relations() -> BTreeMap<String, RelationSpec> {
    let text =
        std::fs::read_to_string(corpus_root().join("relations.tsv")).expect("relation table");
    let mut specs = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let number = index + 1;
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(
            fields.len(),
            6,
            "relations.tsv line {number} is malformed: {line:?}"
        );
        let emits = fields[1].parse().unwrap_or_else(|error| {
            panic!(
                "relations.tsv line {number}: {:?} is not a row count: {error}",
                fields[1]
            )
        });
        let honours_stop = match fields[2] {
            "honours" => true,
            "ignores" => false,
            other => panic!("relations.tsv line {number}: unknown stop-handling {other:?}"),
        };
        let cancel_on_pull = if fields[3] == "never" {
            None
        } else {
            Some(fields[3].parse().unwrap_or_else(|error| {
                panic!(
                    "relations.tsv line {number}: {:?} is not a pull ordinal: {error}",
                    fields[3]
                )
            }))
        };
        let counted = |cell: &str| -> Option<u64> {
            if cell == "?" {
                None
            } else {
                Some(cell.parse().unwrap_or_else(|error| {
                    panic!("relations.tsv line {number}: {cell:?} is not a count: {error}")
                }))
            }
        };
        let previous = specs.insert(
            fields[0].to_owned(),
            RelationSpec {
                emits,
                honours_stop,
                cancel_on_pull,
                invocations: counted(fields[4]),
                pulls: counted(fields[5]),
            },
        );
        assert!(
            previous.is_none(),
            "relations.tsv line {number}: {:?} is listed twice",
            fields[0]
        );
    }
    specs
}

/// The scripted property-function relation: a fixed-width table emitted per invocation,
/// counting what it was asked for and optionally firing the caller's flag mid-iteration.
///
/// It mints nothing — every term it emits is under `example.org`, like every other fixture
/// here — and it is deterministic by construction: its emission order is a pure function
/// of the invocation's bound subject and the row count `relations.tsv` declares.
#[derive(Debug)]
struct ScriptedRelation {
    spec: RelationSpec,
    /// The declared mode, held so `modes` can borrow it.
    modes: Vec<BindingPattern>,
    /// The caller's cancellation flag, when the case attached one.
    flag: Option<CancellationFlag>,
    invocations: AtomicU64,
    /// Shared with every cursor this relation opens: a cursor is `Box<dyn PfCursor>`, which
    /// owns its contents, so the counter travels by handle rather than by borrow.
    pulls: Arc<AtomicU64>,
}

impl ScriptedRelation {
    fn new(spec: RelationSpec, flag: Option<CancellationFlag>) -> Self {
        Self {
            spec,
            // `bf`: the subject is bound by the driving pattern, the object is the
            // relation's output. Declaring only `bf` is what makes the feasibility
            // ordering pass schedule the data pattern first.
            modes: vec![BindingPattern::from_code("bf")],
            flag,
            invocations: AtomicU64::new(0),
            pulls: Arc::new(AtomicU64::new(0)),
        }
    }

    fn invocations(&self) -> u64 {
        self.invocations.load(Ordering::Relaxed)
    }

    fn pulls(&self) -> u64 {
        self.pulls.load(Ordering::Relaxed)
    }
}

impl PropertyFunction for ScriptedRelation {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 1)
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        self.spec.emits
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        self.invocations.fetch_add(1, Ordering::Relaxed);
        let subject = args.get(0).cloned().ok_or_else(|| {
            EvalError::function(format!("<{RELATION_IRI}> needs a bound subject"))
        })?;
        Ok(Box::new(ScriptedCursor {
            subject,
            emitted: 0,
            spec: self.spec,
            flag: self.flag.clone(),
            pulls: Arc::clone(&self.pulls),
        }))
    }
}

/// [`ScriptedRelation`]'s cursor: one invocation's rows, in order, once.
struct ScriptedCursor {
    subject: TermValue,
    emitted: u64,
    spec: RelationSpec,
    flag: Option<CancellationFlag>,
    pulls: Arc<AtomicU64>,
}

impl PfCursor for ScriptedCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        let pull = self.pulls.fetch_add(1, Ordering::Relaxed) + 1;
        if self.spec.cancel_on_pull == Some(pull)
            && let Some(flag) = &self.flag
        {
            flag.cancel();
        }
        // A relation that declines to read the signal is the whole point of the deaf case:
        // the poll below is what a host CAN write, never what makes the query bounded. The
        // evaluator polls between successive pulls either way, so both readings are
        // bounded — they differ only in how much of the invocation already in flight the
        // caller pays for.
        let abandon = self.spec.honours_stop
            && self
                .flag
                .as_ref()
                .is_some_and(CancellationFlag::is_cancelled);
        if abandon || self.emitted >= self.spec.emits {
            return Ok(None);
        }
        let row = vec![
            self.subject.clone(),
            TermValue::iri(format!("{RELATION_OBJECT_PREFIX}{}", self.emitted)),
        ];
        self.emitted += 1;
        Ok(Some(row))
    }
}

/// The injected transport: a counted, network-free exchange that answers from a pinned
/// SPARQL-results JSON document per endpoint.
///
/// The response is corpus data rather than something the harness computes, so the bytes a
/// federated case ingests are frozen exactly as its query and its dataset are.
struct FixtureTransport<'a> {
    /// Endpoint key — the endpoint IRI's last path segment — to its pinned response bytes.
    responses: &'a BTreeMap<String, Vec<u8>>,
    /// Exchanges performed so far.
    posts: &'a AtomicUsize,
    /// Whether this transport reads [`HttpRequest::stop`] at all.
    honours_stop: bool,
    /// A flag this transport cancels during its first exchange, standing in for a host
    /// that cancels while the evaluator is blocked here and can poll nothing.
    cancel_on_first_post: Option<&'a CancellationFlag>,
}

impl HttpTransport for FixtureTransport<'_> {
    fn post(&self, request: HttpRequest<'_>) -> Result<Vec<u8>, RemoteError> {
        let previous = self.posts.fetch_add(1, Ordering::Relaxed);
        if previous == 0
            && let Some(flag) = self.cancel_on_first_post
        {
            flag.cancel();
        }
        // A transport that declines to read the signal is the whole point of the deaf
        // cases: the poll below is what a host CAN write, never what makes the query
        // bounded.
        if self.honours_stop
            && let Some(cause) = request.stop.and_then(StopSignal::poll)
        {
            return Err(RemoteError::Governed(TrippedGovernor::Stopped { cause }));
        }
        let key = request
            .endpoint
            .rsplit('/')
            .next()
            .expect("an endpoint IRI has at least one segment");
        self.responses
            .get(key)
            .cloned()
            .ok_or_else(|| RemoteError::Transport(format!("no pinned response for {key:?}")))
    }
}

/// The pinned endpoint responses for a case, keyed by the endpoint IRI's last segment.
///
/// Discovered from the query fixture's stem (`cases/federated.rq` finds
/// `cases/federated.<key>.srj`) rather than declared, so a response file cannot be listed
/// under a case it does not belong to.
fn responses_for(case: &Case) -> BTreeMap<String, Vec<u8>> {
    let root = corpus_root();
    let stem = Path::new(&case.query)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .expect("a query path has a stem");
    let prefix = format!("{stem}.");
    let mut responses = BTreeMap::new();
    for entry in std::fs::read_dir(root.join("cases")).expect("corpus cases directory") {
        let path = entry.expect("directory entry").path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("srj") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a response file has a name");
        let Some(rest) = name.strip_prefix(&prefix) else {
            continue;
        };
        let key = rest
            .strip_suffix(".srj")
            .expect("the extension was just matched");
        responses.insert(
            key.to_owned(),
            std::fs::read(&path).unwrap_or_else(|error| panic!("read {path:?}: {error}")),
        );
    }
    assert!(
        !responses.is_empty(),
        "{} names the http source but has no pinned endpoint responses",
        case.name
    );
    responses
}

// ---------------------------------------------------------------------------
// The aggregate seam
// ---------------------------------------------------------------------------

/// The IRI [`registered_custom_aggregate`]'s one entry resolves under.
///
/// A fixture IRI under `example.org`, exactly as every other fixture in this corpus is:
/// the custom-aggregate seam is caller configuration, so the corpus supplies its own
/// vocabulary rather than depending on one the engine mints.
const AGGREGATE_IRI: &str = "http://example.org/agg/customSum";

/// [`CorpusSumAggregate`]'s declared per-accumulator [`CustomAggregate::state_bound`].
///
/// Nonzero on purpose, unlike every built-in fold (a built-in aggregate's accumulator —
/// see `crate::modifier`'s `CountAccumulator`/`SumAccumulator`/etc. in the evaluator
/// crate — declares no `state_bound` at all; only `CustomAggregate` does): the fold
/// itself is stateless (a running `i64`), so this is a FICTITIOUS bound standing in for the opaque
/// host state a real custom accumulator (a running median, a per-value histogram) would
/// actually retain. It exists so the `aggregate-custom-scratch-bytes-*` lane has a real,
/// nonzero `ScratchBytes` charge to band.
const AGGREGATE_STATE_BOUND: u64 = 64;

/// A `Commutative` custom `SUM`-alike over a single `xsd:integer`-lexical argument:
/// `step` parses and adds, and `combine` merges through `other.finish()` because the
/// running total already IS this accumulator's whole mergeable state (see
/// [`AggregateAccumulator::combine`]'s doc comment on when that shortcut is sound, in the
/// evaluator crate). Registered under [`AGGREGATE_IRI`] by every `Source::Aggregate` case
/// via [`registered_custom_aggregate`].
struct CorpusSumAccumulator {
    total: i64,
}

impl AggregateAccumulator for CorpusSumAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        if let Some(TermValue::Literal { lexical_form, .. }) = args.first()
            && let Ok(n) = lexical_form.parse::<i64>()
        {
            self.total += n;
        }
        Ok(())
    }

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        if let Some(TermValue::Literal { lexical_form, .. }) = other.finish()?
            && let Ok(n) = lexical_form.parse::<i64>()
        {
            self.total += n;
        }
        Ok(())
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(Some(TermValue::typed_literal(
            self.total.to_string(),
            "http://www.w3.org/2001/XMLSchema#integer",
        )))
    }
}

struct CorpusSumAggregate;

impl CustomAggregate for CorpusSumAggregate {
    fn arity(&self) -> Arity {
        Arity::Exact(1)
    }
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::Commutative
    }
    fn state_bound(&self) -> u64 {
        AGGREGATE_STATE_BOUND
    }
    fn init(&self, _scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
        Box::new(CorpusSumAccumulator { total: 0 })
    }
}

/// The registry every `Source::Aggregate` case wires — one entry, [`AGGREGATE_IRI`].
///
/// Freshly built per call rather than shared, mirroring [`ScriptedRelation`]'s own
/// per-observation construction: a governed run must never carry state left over from a
/// previous one.
fn registered_custom_aggregate() -> AggregateRegistry {
    let mut registry = AggregateRegistry::new();
    registry.register(AGGREGATE_IRI, Arc::new(CorpusSumAggregate));
    registry
}

// ---------------------------------------------------------------------------
// Running a case
// ---------------------------------------------------------------------------

/// The media type a case's data extension selects. Driven off the extension rather than
/// recorded in the manifest, so a fixture cannot be listed under a syntax it is not
/// written in.
fn media_type_for(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("ttl") => "text/turtle",
        Some("trig") => "application/trig",
        other => panic!("unhandled corpus data extension {other:?} for {path:?}"),
    }
}

fn load_dataset(case: &Case) -> Arc<RdfDataset> {
    let path = corpus_root().join(&case.data);
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("read {path:?}: {error}"));
    purrdf::parse_dataset(&bytes, media_type_for(&path), None)
        .unwrap_or_else(|error| panic!("{} data must parse: {error}", case.name))
}

fn load_query(case: &Case) -> String {
    let path = corpus_root().join(&case.query);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {path:?}: {error}"))
}

/// What one governed run of a case produced.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Observation {
    /// The outcome discriminant, in the manifest's spelling.
    outcome: String,
    /// The rendered answer: the rows and their certificate class.
    answer: String,
    /// The rendered consumption vector.
    spend: String,
    /// Exchanges the injected transport was asked to perform.
    posts: usize,
    /// Polls observed by an injected deterministic deadline.
    stop_polls: u64,
    /// Invocations the scripted relation was opened for.
    invocations: u64,
    /// Calls to `PfCursor::next` the scripted relation's cursor received, across every
    /// invocation. The counter increments before the exhaustion check, so the terminating
    /// `Ok(None)` call is counted too — this is a call count, not a row count.
    pulls: u64,
}

/// The governors a case declares, together with the live cancellation flag they carry.
///
/// The flag travels beside the configuration rather than inside it: `QueryGovernors`
/// publishes its signal as an `Arc<dyn StopSignal>`, which is the right shape for the
/// engine to POLL and the wrong one for a transport that has to FIRE it.
struct Configured {
    governors: QueryGovernors,
    flag: Option<CancellationFlag>,
    deadline: Option<Arc<PollDeadline>>,
}

/// A deterministic deadline fixture: admit `allowed` polls and fire on the next.
///
/// This is deliberately not [`WallDeadline`]. A wall clock has no reproducible boundary;
/// this signal gives the frozen corpus an injected timeline on which zero, exact-boundary,
/// and one-before-boundary cases have stable meanings.
#[derive(Debug)]
struct PollDeadline {
    allowed: u64,
    polls: AtomicU64,
}

impl PollDeadline {
    fn new(allowed: u64) -> Arc<Self> {
        Arc::new(Self {
            allowed,
            polls: AtomicU64::new(0),
        })
    }

    fn polls(&self) -> u64 {
        self.polls.load(Ordering::Relaxed)
    }
}

impl StopSignal for PollDeadline {
    fn poll(&self) -> Option<StopCause> {
        let previous = self.polls.fetch_add(1, Ordering::Relaxed);
        (previous >= self.allowed).then_some(StopCause::Deadline)
    }
}

fn governors_for(case: &Case) -> Configured {
    let mut governors = QueryGovernors::UNBOUNDED;
    let mut flag = None;
    if let Some((dimension, amount)) = case.ceiling {
        governors = match dimension {
            ResourceDimension::Fuel => governors.with_fuel(amount),
            ResourceDimension::AnswerRows => governors.with_max_answers(amount),
            ResourceDimension::IntermediateCells => governors.with_max_intermediate_cells(amount),
            ResourceDimension::ScratchBytes => governors.with_max_scratch_bytes(amount),
            ResourceDimension::RemoteRequests => governors.with_max_remote_requests(amount),
            other => panic!(
                "{} sets {}, which is not caller-settable",
                case.name,
                other.label()
            ),
        };
    }
    match case.stop {
        None => {}
        Some(StopSpec::WallDeadlineZero) => {
            governors = governors.with_stop_signal(Arc::new(WallDeadline::after(Duration::ZERO)));
        }
        Some(spec) => {
            let cancellation = CancellationFlag::new();
            if spec == StopSpec::Cancelled {
                cancellation.cancel();
            }
            governors = governors.with_stop_signal(Arc::new(cancellation.clone()));
            flag = Some(cancellation);
        }
    }
    let deadline = case.deadline_polls.map(PollDeadline::new);
    if let Some(deadline) = &deadline {
        governors = governors.with_stop_signal(Arc::clone(deadline) as Arc<dyn StopSignal>);
    }
    Configured {
        governors,
        flag,
        deadline,
    }
}

/// Evaluate `case` under `configured`, with the transport wired as `spec` says.
fn observe(
    case: &Case,
    configured: &Configured,
    spec: Option<TransportSpec>,
    relation_spec: Option<RelationSpec>,
) -> Observation {
    let dataset = load_dataset(case);
    let query = load_query(case);
    let engine = NativeSparqlEngine::new();
    let request = SparqlRequest {
        query: &query,
        base_iri: None,
        substitutions: &[],
    };

    let posts = AtomicUsize::new(0);
    let mut relation: Option<Arc<ScriptedRelation>> = None;
    let outcome = match case.source {
        Source::Http => {
            let spec = spec.unwrap_or_else(|| {
                panic!("{} names the http source with no transport row", case.name)
            });
            let responses = responses_for(case);
            let source = HttpRemoteQuerySource::new(FixtureTransport {
                responses: &responses,
                posts: &posts,
                honours_stop: spec.honours_stop,
                cancel_on_first_post: if spec.cancel_on_first_post {
                    configured.flag.as_ref()
                } else {
                    None
                },
            });
            engine.query_governed_with_source(
                &dataset,
                request,
                &source,
                QueryOptions::EMPTY,
                &configured.governors,
            )
        }
        Source::Relation => {
            let spec = relation_spec.unwrap_or_else(|| {
                panic!(
                    "{} names the relation source with no relations.tsv row",
                    case.name
                )
            });
            let scripted = Arc::new(ScriptedRelation::new(spec, configured.flag.clone()));
            relation = Some(Arc::clone(&scripted));
            let mut registry = PropertyFunctionRegistry::new();
            registry.register(
                RELATION_IRI,
                Arc::clone(&scripted) as Arc<dyn PropertyFunction>,
            );
            // The relation lane is the headline governed entry, with the registry
            // handed to it in the options: same entry, same per-call state, same
            // budget as every other lane. It differs from `Source::Dataset` in
            // exactly one field.
            engine.query_governed(
                &dataset,
                request,
                QueryOptions {
                    property_functions: &registry,
                    ..QueryOptions::EMPTY
                },
                &configured.governors,
            )
        }
        Source::Aggregate => {
            let registry = registered_custom_aggregate();
            // Same shape as the relation lane's own comment: the aggregate lane differs
            // from `Source::Dataset` in exactly one `QueryOptions` field.
            engine.query_governed(
                &dataset,
                request,
                QueryOptions {
                    aggregates: &registry,
                    ..QueryOptions::EMPTY
                },
                &configured.governors,
            )
        }
        Source::Dataset => engine.query_governed(
            &dataset,
            request,
            QueryOptions::EMPTY,
            &configured.governors,
        ),
    }
    .unwrap_or_else(|error| panic!("{} must evaluate: {error}", case.name));

    Observation {
        outcome: render_outcome(&outcome),
        answer: render_answer(&outcome),
        spend: render_consumption(outcome.evidence()),
        posts: posts.load(Ordering::Relaxed),
        stop_polls: configured
            .deadline
            .as_ref()
            .map_or(0, |signal| signal.polls()),
        invocations: relation
            .as_ref()
            .map_or(0, |scripted| scripted.invocations()),
        pulls: relation.as_ref().map_or(0, |scripted| scripted.pulls()),
    }
}

/// The measuring run: every counter engaged, nothing bounded.
///
/// This is what every boundary is derived FROM, and it is spelled as the named constant
/// rather than as a hand-written near-maximum ceiling so the corpus teaches the same way
/// of sizing a budget the library documents.
struct Measurement {
    spend: String,
    stop_polls: u64,
}

fn metered(
    case: &Case,
    spec: Option<TransportSpec>,
    relation_spec: Option<RelationSpec>,
) -> Measurement {
    let configured = Configured {
        governors: QueryGovernors::METERED,
        flag: None,
        deadline: None,
    };
    let observation = observe(case, &configured, spec, relation_spec);
    assert_eq!(
        observation.outcome, "complete",
        "{}: the measuring run must complete — METERED bounds nothing, so a trip means the \
         case has no derivable boundary at all",
        case.name
    );
    let stop_polls = if case.deadline_polls.is_some() {
        let deadline = PollDeadline::new(u64::MAX);
        let configured = Configured {
            governors: QueryGovernors::UNBOUNDED
                .with_stop_signal(Arc::clone(&deadline) as Arc<dyn StopSignal>),
            flag: None,
            deadline: Some(deadline),
        };
        let deadline_observation = observe(case, &configured, spec, relation_spec);
        assert_eq!(
            deadline_observation.outcome, "complete",
            "{}: a never-firing injected deadline must admit the whole query",
            case.name
        );
        deadline_observation.stop_polls
    } else {
        0
    };
    Measurement {
        spend: observation.spend,
        stop_polls,
    }
}

/// The metered run's fuel, decomposed **per charge point**.
///
/// Fuel is one number, and the property-function seam put two charge points inside it. A
/// band on the aggregate cannot tell a schedule that moved
/// [`ChargePoint::PropertyFunctionInvocation`] from one that moved
/// [`ChargePoint::PropertyFunctionRow`] the other way; this record can, and it is what the
/// invocation-isolating and row-isolating lanes are read through.
///
/// Taken from the public per-node ledger
/// ([`NativeSparqlEngine::explain_query_with_property_functions`]), which runs the same
/// query under the same [`QueryGovernors::METERED`] the `.metered` record is measured with —
/// so the totals here must add back up to that record's fuel, and
/// [`every_boundary_is_derived_from_a_metered_run`] checks that they do. A decomposition
/// that did not sum to the quantity it claims to decompose would be a decomposition of
/// something else.
fn charge_decomposition(case: &Case, spec: RelationSpec) -> String {
    let dataset = load_dataset(case);
    let query = load_query(case);
    let scripted = Arc::new(ScriptedRelation::new(spec, None));
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        RELATION_IRI,
        Arc::clone(&scripted) as Arc<dyn PropertyFunction>,
    );
    let explanation = NativeSparqlEngine::new()
        .explain_query_with_property_functions(&dataset, &query, None, &registry)
        .unwrap_or_else(|error| panic!("{} must explain: {error}", case.name));
    let mut out = String::new();
    for point in ChargePoint::ALL {
        let total: u64 = explanation
            .ledger()
            .iter()
            .map(|node| node.fuel_at(point))
            .sum();
        writeln!(out, "{point}\t{total}").expect("writing to a String cannot fail");
    }
    out
}

/// [`charge_decomposition`]'s twin for the aggregate lane, which needs no injected seam:
/// an aggregate is ordinary algebra, not a host relation, so the metered run is taken
/// through plain [`NativeSparqlEngine::explain_query`] rather than the property-function
/// variant. Used only for [`wants_dataset_charge_decomposition`] cases — every other case's
/// `.charges` decomposition would just be its `.metered` fuel restated per point for no
/// reason a caller could act on.
fn charge_decomposition_dataset(case: &Case) -> String {
    let dataset = load_dataset(case);
    let query = load_query(case);
    let explanation = NativeSparqlEngine::new()
        .explain_query(&dataset, &query, None)
        .unwrap_or_else(|error| panic!("{} must explain: {error}", case.name));
    let mut out = String::new();
    for point in ChargePoint::ALL {
        let total: u64 = explanation
            .ledger()
            .iter()
            .map(|node| node.fuel_at(point))
            .sum();
        writeln!(out, "{point}\t{total}").expect("writing to a String cannot fail");
    }
    out
}

/// Whether `name` is one of the two lanes that separate `aggregate-invocation` from
/// `aggregate-accumulation` — the aggregate lane's counterpart to a relation case's
/// `relation_spec.is_some()` test, since neither aggregate lane wires an injected seam for
/// `load_relations`/`load_transport` to key on.
fn wants_dataset_charge_decomposition(name: &str) -> bool {
    name.starts_with("aggregate-invocation-fuel-")
        || name.starts_with("aggregate-accumulation-fuel-")
}

/// The fuel a rendered decomposition attributes to `point`.
fn charged_at(record: &str, point: ChargePoint) -> u64 {
    for line in record.lines() {
        let Some((label, value)) = line.split_once('\t') else {
            continue;
        };
        if label == point.label() {
            return value.parse().expect("a rendered charge count is a number");
        }
    }
    panic!("no {point} row in {record:?}")
}

/// Every charge point's fuel in a rendered decomposition, summed.
fn charged_total(record: &str) -> u64 {
    ChargePoint::ALL
        .into_iter()
        .map(|point| charged_at(record, point))
        .sum()
}

// ---------------------------------------------------------------------------
// Deterministic rendering
// ---------------------------------------------------------------------------

fn render_consumption(evidence: &GovernorEvidence) -> String {
    let mut out = String::new();
    for dimension in PINNED_DIMENSIONS {
        writeln!(
            out,
            "{}\t{}",
            dimension.label(),
            evidence.consumed_in(dimension)
        )
        .expect("writing to a String cannot fail");
    }
    out
}

/// The consumption a rendered record reports for `dimension`.
fn consumed_in(record: &str, dimension: ResourceDimension) -> u64 {
    for line in record.lines() {
        let Some((label, value)) = line.split_once('\t') else {
            continue;
        };
        if label == dimension.label() {
            return value.parse().expect("a rendered consumption is a number");
        }
    }
    panic!("no {} row in {record:?}", dimension.label())
}

const fn stop_cause_label(cause: StopCause) -> &'static str {
    match cause {
        StopCause::Cancelled => "cancelled",
        StopCause::Deadline => "deadline",
    }
}

/// The outcome discriminant, in the manifest's spelling.
fn render_outcome(outcome: &GovernedOutcome) -> String {
    match outcome.tripped() {
        None => "complete".to_owned(),
        Some(TrippedGovernor::Budget { dimension, .. }) => {
            format!("budget-exhausted {}", dimension.label())
        }
        Some(TrippedGovernor::Refused { dimension, .. }) => {
            format!("refused {}", dimension.label())
        }
        Some(TrippedGovernor::Stopped { cause }) => format!("stopped {}", stop_cause_label(cause)),
        // The kernel vocabulary is `#[non_exhaustive]`. A governor it grows and this corpus
        // has never seen has no pinned spelling, and inventing one — "unknown", say — would
        // put a discriminant into a receipt a consumer pins that no specification defines.
        Some(other) => panic!("unpinned governor variant {other:?}"),
    }
}

/// The full governor record, including the numbers the discriminant leaves out.
fn render_tripped(governor: TrippedGovernor) -> String {
    match governor {
        TrippedGovernor::Budget {
            dimension,
            limit,
            consumed,
        } => format!(
            "budget\t{}\t{}\tlimit={limit}\tconsumed={consumed}",
            dimension.label(),
            governor.label()
        ),
        TrippedGovernor::Refused {
            dimension,
            limit,
            estimate,
        } => format!(
            "refused\t{}\t{}\tlimit={limit}\testimate={estimate}",
            dimension.label(),
            governor.label()
        ),
        TrippedGovernor::Stopped { cause } => format!("stopped\t{}", stop_cause_label(cause)),
        other => panic!("unpinned governor variant {other:?}"),
    }
}

fn escape_lexical(lexical: &str, out: &mut String) {
    for character in lexical.chars() {
        match character {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
}

const fn direction_label(direction: RdfTextDirection) -> &'static str {
    match direction {
        RdfTextDirection::Ltr => "ltr",
        RdfTextDirection::Rtl => "rtl",
    }
}

/// One solution cell, in N-Triples term syntax extended with RDF 1.2 triple terms.
///
/// Written here rather than routed through a SPARQL Results writer for the reason the
/// differential harness gives: every results format has a support matrix, and a term shape
/// a writer declines would silently drop out of the comparison — which is exactly the
/// class of case this corpus exists to hold.
fn render_term(value: &TermValue, out: &mut String) {
    match value {
        TermValue::Iri(iri) => {
            out.push('<');
            out.push_str(iri);
            out.push('>');
        }
        TermValue::Blank { label, scope } => {
            write!(out, "_:{label}/{}", scope.ordinal()).expect("writing to a String cannot fail");
        }
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => {
            out.push('"');
            escape_lexical(lexical_form, out);
            out.push('"');
            let suffix = match (language, direction) {
                (Some(language), None) => format!("@{language}"),
                (Some(language), Some(direction)) => {
                    format!("@{language}--{}", direction_label(*direction))
                }
                _ => format!("^^<{datatype}>"),
            };
            out.push_str(&suffix);
        }
        TermValue::Triple { s, p, o } => {
            out.push_str("<<( ");
            render_term(s, out);
            out.push(' ');
            render_term(p, out);
            out.push(' ');
            render_term(o, out);
            out.push_str(" )>>");
        }
    }
}

/// A total, deterministic rendering of the whole egress model.
///
/// Graphs go through the RDFC-1.0 canonicalizer, which is what makes a `CONSTRUCT` case
/// comparable at all: two runs may legitimately label a fresh blank differently, and
/// canonical N-Quads is the byte form in which they must nevertheless agree. It also
/// lowers the RDF 1.2 statement layer, so a reifier in a constructed graph appears in the
/// pinned bytes instead of hiding in a side table.
fn render_result(result: &SparqlResult, out: &mut String) {
    match result {
        SparqlResult::Boolean(value) => {
            writeln!(out, "boolean\t{value}").expect("writing to a String cannot fail");
        }
        SparqlResult::Graph(graph) => {
            out.push_str("graph\n");
            out.push_str(&purrdf_core::canonicalize(graph).nquads);
        }
        SparqlResult::Solutions {
            variables,
            rows,
            aux,
        } => {
            out.push_str("variables");
            for variable in variables {
                out.push('\t');
                out.push_str(variable);
            }
            out.push('\n');
            for row in rows {
                out.push_str("row");
                for cell in row {
                    out.push('\t');
                    match cell {
                        None => out.push_str("UNBOUND"),
                        Some(value) => render_term(value, out),
                    }
                }
                out.push('\n');
            }
            // Emitted only when a value-constructing builtin actually minted something, so
            // an ordinary SELECT's record is not padded with an empty section.
            if aux.quad_count() > 0 {
                out.push_str("aux\n");
                out.push_str(&purrdf_core::canonicalize(aux).nquads);
            }
        }
    }
}

/// The rows a case's caller receives, and what those rows are certified to be.
fn render_answer(outcome: &GovernedOutcome) -> String {
    let mut out = String::new();
    let exhausted = match outcome {
        GovernedOutcome::Complete { result, .. } => {
            out.push_str("outcome\tcomplete\n");
            render_result(result, &mut out);
            return out;
        }
        GovernedOutcome::BudgetExhausted(exhausted) => exhausted,
    };
    out.push_str("outcome\tbudget-exhausted\n");
    writeln!(out, "tripped\t{}", render_tripped(exhausted.tripped))
        .expect("writing to a String cannot fail");
    match &exhausted.partial {
        PartialAnswers::Certain(partial) | PartialAnswers::AtMost(partial) => {
            let class = if exhausted.partial.is_certain() {
                "certain"
            } else {
                "at-most"
            };
            writeln!(
                out,
                "certificate\t{class}\tpositional-prefix={}",
                partial.is_positional_prefix()
            )
            .expect("writing to a String cannot fail");
            render_result(partial.result(), &mut out);
        }
        PartialAnswers::Unknown(barrier) => {
            // No rows, deliberately: rows that bound the answer on neither side offer no
            // sound use, so there is nothing here a reader could mistake for an answer.
            writeln!(out, "certificate\tunknown\tbarrier={}", barrier.operator())
                .expect("writing to a String cannot fail");
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Expectation files
// ---------------------------------------------------------------------------

fn expected_path(case: &Case, extension: &str) -> PathBuf {
    corpus_root()
        .join("expected")
        .join(format!("{}.{extension}", case.name))
}

fn read_expected(case: &Case, extension: &str) -> String {
    let path = expected_path(case, extension);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "{} has no pinned {extension} record ({}): {error}",
            case.name,
            path.display()
        )
    })
}

fn write_expected(case: &Case, extension: &str, contents: &str) {
    let path = expected_path(case, extension);
    std::fs::create_dir_all(path.parent().expect("expected/ has a parent"))
        .expect("create expected/");
    std::fs::write(&path, contents).unwrap_or_else(|error| panic!("write {path:?}: {error}"));
}

/// The result-rendering half of a pinned answer record: everything from the egress
/// model's own first line onward, with the outcome and certificate headers dropped.
///
/// Lets two cases be compared on the rows alone, so a difference in what the rows ARE is
/// not confused with a difference in what they are certified to be.
fn answer_rows(record: &str) -> &str {
    for start in ["variables", "graph\n", "boolean\t"] {
        if let Some(index) = record.find(start) {
            return &record[index..];
        }
    }
    ""
}

/// The ceiling a band demands of a dimension whose metered cost is `cost`.
fn band_ceiling(case: &Case, band: Band, cost: u64) -> u64 {
    match band {
        Band::Zero => 0,
        Band::Boundary => cost,
        Band::OverBound => cost.checked_sub(1).unwrap_or_else(|| {
            panic!(
                "{}: an over-bound case needs a metered cost of at least one to sit below",
                case.name
            )
        }),
        Band::NotApplicable => panic!("{}: no band ceiling to derive", case.name),
    }
}

// ---------------------------------------------------------------------------
// Regeneration
// ---------------------------------------------------------------------------

/// The manifest row a freshly measured case produces.
fn manifest_row(case: &Case, outcome: &str) -> String {
    let ceiling = case
        .ceiling
        .map(|(dimension, amount)| format!("{}={amount}", dimension.label()));
    let stop = case.stop.map(|stop| format!("stop={}", stop.label()));
    let deadline = case
        .deadline_polls
        .map(|polls| format!("deadline-polls={polls}"));
    let settings = [ceiling, stop, deadline]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(",");
    assert!(!settings.is_empty(), "a case must name a governor");
    format!(
        "{}\t{}\t{}\t{}\t{settings}\t{}\t{outcome}",
        case.name,
        case.data,
        case.query,
        case.source.label(),
        case.band.label(),
    )
}

/// What regenerating one case produced: its manifest row, plus the measured sidecar row
/// for each side table the case has one in, keyed by the case name.
///
/// A named record rather than a tuple, because the two sidecars are both
/// `Option<(String, _)>` and nothing about their positions says which is which.
struct Regenerated {
    /// The case's `manifest.tsv` row.
    manifest_row: String,
    /// The `transport.tsv` row, for a federated case.
    transport: Option<(String, TransportSpec)>,
    /// The `relations.tsv` row, for a property-function case.
    relation: Option<(String, RelationSpec)>,
}

/// Measure one case and produce its regenerated manifest row, its sidecar rows, and its
/// `expected/` records.
fn regenerate_case(
    case: &Case,
    spec: Option<TransportSpec>,
    relation_spec: Option<RelationSpec>,
) -> Regenerated {
    let mut case = case.clone();
    if let Some(relation_spec) = relation_spec {
        write_expected(
            &case,
            "charges",
            &charge_decomposition(&case, relation_spec),
        );
    } else if wants_dataset_charge_decomposition(&case.name) {
        write_expected(&case, "charges", &charge_decomposition_dataset(&case));
    }
    if case.is_pinned_deterministic() {
        let measured = metered(&case, spec, relation_spec);
        write_expected(&case, "metered", &measured.spend);
        if case.band != Band::NotApplicable {
            if let Some((dimension, _)) = case.ceiling {
                let derived =
                    band_ceiling(&case, case.band, consumed_in(&measured.spend, dimension));
                case.ceiling = Some((dimension, derived));
            } else if case.deadline_polls.is_some() {
                case.deadline_polls = Some(band_ceiling(&case, case.band, measured.stop_polls));
            } else {
                panic!("{}: a band case must set a ceiling", case.name);
            }
        }
    }

    let configured = governors_for(&case);
    let observation = observe(&case, &configured, spec, relation_spec);
    if case.is_pinned_deterministic() {
        write_expected(&case, "answer", &observation.answer);
        write_expected(&case, "spend", &observation.spend);
    }
    let transport = spec.map(|spec| {
        (
            case.name.clone(),
            TransportSpec {
                posts: Some(observation.posts),
                ..spec
            },
        )
    });
    let relation = relation_spec.map(|spec| {
        (
            case.name.clone(),
            RelationSpec {
                invocations: Some(observation.invocations),
                pulls: Some(observation.pulls),
                ..spec
            },
        )
    });
    Regenerated {
        manifest_row: manifest_row(&case, &observation.outcome),
        transport,
        relation,
    }
}

/// Rewrite the derived cells of `manifest.tsv` and `transport.tsv`, and every `expected/`
/// record, from freshly measured runs.
///
/// Driven from the ordinary test entry point under an environment variable rather than
/// from a separate binary, so a regeneration and a verification share one code path and
/// cannot drift into producing what the checker does not accept.
fn regenerate(
    cases: &[Case],
    specs: &BTreeMap<String, TransportSpec>,
    relations: &BTreeMap<String, RelationSpec>,
) {
    let mut manifest_rows = BTreeMap::new();
    let mut transport_rows = BTreeMap::new();
    let mut relation_rows = BTreeMap::new();
    for case in cases {
        let regenerated = regenerate_case(
            case,
            specs.get(&case.name).copied(),
            relations.get(&case.name).copied(),
        );
        manifest_rows.insert(case.name.clone(), regenerated.manifest_row);
        if let Some((name, spec)) = regenerated.transport {
            transport_rows.insert(
                name.clone(),
                format!(
                    "{name}\t{}\t{}\t{}",
                    spec.stop_handling(),
                    spec.on_first_post(),
                    spec.posts.expect("a regenerated row carries its count")
                ),
            );
        }
        if let Some((name, spec)) = regenerated.relation {
            relation_rows.insert(
                name.clone(),
                format!(
                    "{name}\t{}\t{}\t{}\t{}\t{}",
                    spec.emits,
                    spec.stop_handling(),
                    spec.cancel_cell(),
                    spec.invocations
                        .expect("a regenerated row carries its invocation count"),
                    spec.pulls
                        .expect("a regenerated row carries its pull count"),
                ),
            );
        }
    }
    rewrite_table(&corpus_root().join("manifest.tsv"), &manifest_rows);
    rewrite_table(&corpus_root().join("transport.tsv"), &transport_rows);
    rewrite_table(&corpus_root().join("relations.tsv"), &relation_rows);
}

/// Rewrite a TSV's data rows from `rows`, preserving its comment header and the authored
/// row order.
///
/// Order is preserved rather than sorted so the manifest keeps reading as a case
/// inventory: the lanes stay together, and a regeneration diff shows what moved rather
/// than a reshuffle.
fn rewrite_table(path: &Path, rows: &BTreeMap<String, String>) {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|error| panic!("read {path:?}: {error}"));
    let mut out = String::new();
    for line in text.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            out.push_str(line);
        } else {
            let key = line.split('\t').next().unwrap_or_default();
            out.push_str(
                rows.get(key)
                    .unwrap_or_else(|| panic!("{path:?}: no regenerated row for {key:?}")),
            );
        }
        out.push('\n');
    }
    std::fs::write(path, out).unwrap_or_else(|error| panic!("write {path:?}: {error}"));
}

// ---------------------------------------------------------------------------
// The tests
// ---------------------------------------------------------------------------

/// Every case reaches its pinned outcome, hands back its pinned rows under its pinned
/// certificate, and spends exactly what the corpus says it spends.
#[test]
fn the_corpus_matches_its_pinned_expectations() {
    let cases = load_manifest();
    let specs = load_transport();
    let relations = load_relations();

    if updating() {
        regenerate(&cases, &specs, &relations);
        return;
    }

    for case in &cases {
        let spec = specs.get(&case.name).copied();
        let relation_spec = relations.get(&case.name).copied();
        assert_eq!(
            spec.is_some(),
            case.source == Source::Http,
            "{}: transport.tsv membership must match the manifest's source column",
            case.name
        );
        assert_eq!(
            relation_spec.is_some(),
            case.source == Source::Relation,
            "{}: relations.tsv membership must match the manifest's source column",
            case.name
        );

        let observation = observe(case, &governors_for(case), spec, relation_spec);
        assert_eq!(
            observation.outcome, case.outcome,
            "{} reached a different outcome than the manifest pins",
            case.name
        );

        if let Some(spec) = relation_spec {
            // The two counts no governor evidence carries: whether host code was entered
            // at all, and how much of an invocation already in flight the caller paid for.
            assert_eq!(
                observation.invocations,
                spec.invocations.unwrap_or_else(|| {
                    panic!("{}: relations.tsv still carries a placeholder", case.name)
                }),
                "{} opened a different number of invocations than relations.tsv pins",
                case.name
            );
            assert_eq!(
                observation.pulls,
                spec.pulls.unwrap_or_else(|| {
                    panic!("{}: relations.tsv still carries a placeholder", case.name)
                }),
                "{} pulled a different number of rows than relations.tsv pins — the only \
                 observation separating a prevented pull from one that was made and \
                 discarded",
                case.name
            );
        }

        if let Some(spec) = spec {
            let expected = spec.posts.unwrap_or_else(|| {
                panic!("{}: transport.tsv still carries a placeholder", case.name)
            });
            assert_eq!(
                observation.posts, expected,
                "{} performed a different number of exchanges than transport.tsv pins — the \
                 only observation that separates a prevented request from one that was made \
                 and discarded",
                case.name
            );
        }

        if !case.is_pinned_deterministic() {
            // A wall deadline trip is time-dependent. The discriminant above is the whole
            // of what is guaranteed; pinning rows or a cost for it would publish a
            // determinism claim this engine does not make.
            continue;
        }

        assert_eq!(
            observation.answer,
            read_expected(case, "answer"),
            "{} produced different certified rows than the corpus pins",
            case.name
        );
        // Compared SEPARATELY from the rows, and the separation is the point: a charge
        // schedule that moved but happened to cut in the same place leaves the rows
        // identical and this record different.
        assert_eq!(
            observation.spend,
            read_expected(case, "spend"),
            "{} spent a different amount than the corpus pins, for the same rows",
            case.name
        );
    }

    // Scraped by scripts/conformance-matrix.py. Printed only after every case matched, so
    // the scoreboard cannot report a pass count for a run that failed.
    let bands = cases
        .iter()
        .filter(|case| case.band != Band::NotApplicable)
        .count();
    println!(
        "GOVERNOR-CORPUS: passed {} total {} bands {bands}",
        cases.len(),
        cases.len()
    );
}

/// Every boundary in the corpus is the measured cost, not a number somebody typed.
///
/// Re-measured on every run through [`QueryGovernors::METERED`] and compared against both
/// the pinned metered record and the manifest's ceiling, so a charge-schedule change
/// cannot leave a stale boundary sitting in the manifest looking authoritative.
#[test]
fn every_boundary_is_derived_from_a_metered_run() {
    if updating() {
        return;
    }
    let cases = load_manifest();
    let specs = load_transport();
    let relations = load_relations();

    let mut bands = 0_usize;
    for case in &cases {
        if !case.is_pinned_deterministic() {
            continue;
        }
        let measured = metered(
            case,
            specs.get(&case.name).copied(),
            relations.get(&case.name).copied(),
        );
        assert_eq!(
            measured.spend,
            read_expected(case, "metered"),
            "{}: the measuring run's cost moved without the corpus being regenerated",
            case.name
        );

        // A relation case's fuel is two new charge points plus the generic accounting, and
        // an aggregate band cannot tell them apart. The decomposition is re-derived here
        // for the same reason the boundary is: a number nobody re-measures is a number
        // nobody can trust.
        if let Some(relation_spec) = relations.get(&case.name).copied() {
            let decomposition = charge_decomposition(case, relation_spec);
            assert_eq!(
                decomposition,
                read_expected(case, "charges"),
                "{}: the per-charge-point decomposition of the metered fuel moved without \
                 the corpus being regenerated",
                case.name
            );
            assert_eq!(
                charged_total(&decomposition),
                consumed_in(&measured.spend, ResourceDimension::Fuel),
                "{}: the pinned decomposition does not add up to the fuel it decomposes",
                case.name
            );
        } else if wants_dataset_charge_decomposition(&case.name) {
            // The aggregate lane's twin of the check above: `aggregate-invocation` and
            // `aggregate-accumulation` are the same kind of fuel-internal pair, taken
            // through a plain `explain_query` rather than a scripted relation.
            let decomposition = charge_decomposition_dataset(case);
            assert_eq!(
                decomposition,
                read_expected(case, "charges"),
                "{}: the per-charge-point decomposition of the metered fuel moved without \
                 the corpus being regenerated",
                case.name
            );
            assert_eq!(
                charged_total(&decomposition),
                consumed_in(&measured.spend, ResourceDimension::Fuel),
                "{}: the pinned decomposition does not add up to the fuel it decomposes",
                case.name
            );
        }

        if case.band == Band::NotApplicable {
            continue;
        }
        bands += 1;
        if let Some((dimension, ceiling)) = case.ceiling {
            assert_eq!(
                ceiling,
                band_ceiling(case, case.band, consumed_in(&measured.spend, dimension),),
                "{}: the manifest's {} ceiling is not the {} of the metered cost",
                case.name,
                dimension.label(),
                case.band.label()
            );
        } else if let Some(deadline_polls) = case.deadline_polls {
            assert_eq!(
                deadline_polls,
                band_ceiling(case, case.band, measured.stop_polls),
                "{}: the injected deadline is not the {} of the measured poll count",
                case.name,
                case.band.label()
            );
        } else {
            panic!("{}: a band case must set a ceiling", case.name);
        }
    }
    assert!(
        bands >= REQUIRED_BAND_CASES,
        "the band matrix shrank: only {bands} zero/boundary/over-bound cases remain, and the \
         corpus owes one of each to every caller-settable dimension"
    );
}

/// Every caller-settable dimension and the injected deadline carry all three bands, and a
/// boundary case really does complete while its zero and over-bound siblings really trip.
///
/// A count alone would pass on fifteen cases that all governed fuel.
#[test]
fn every_governor_carries_a_zero_a_boundary_and_an_over_bound_case() {
    if updating() {
        return;
    }
    let cases = load_manifest();
    for dimension in PINNED_DIMENSIONS {
        let mut seen: BTreeMap<&'static str, usize> = BTreeMap::new();
        for case in &cases {
            if case.ceiling.map(|(named, _)| named) != Some(dimension)
                || case.band == Band::NotApplicable
            {
                continue;
            }
            *seen.entry(case.band.label()).or_default() += 1;
            match case.band {
                Band::Boundary => assert_eq!(
                    case.outcome, "complete",
                    "{}: a ceiling equal to the measured cost must be admitted — the boundary \
                     is inclusive on every dimension",
                    case.name
                ),
                Band::Zero | Band::OverBound => assert_ne!(
                    case.outcome, "complete",
                    "{}: a ceiling below the measured cost must stop the execution",
                    case.name
                ),
                Band::NotApplicable => unreachable!("filtered above"),
            }
        }
        for band in ["zero", "boundary", "over-bound"] {
            assert!(
                seen.contains_key(band),
                "{} has no {band} case; a governor with an untested band is one whose boundary \
                 nobody has ever crossed",
                dimension.label()
            );
        }
    }

    let mut deadline_bands = BTreeMap::new();
    for case in &cases {
        if case.deadline_polls.is_some() && case.band != Band::NotApplicable {
            *deadline_bands.entry(case.band.label()).or_insert(0_usize) += 1;
            match case.band {
                Band::Boundary => assert_eq!(case.outcome, "complete"),
                Band::Zero | Band::OverBound => assert_eq!(case.outcome, "stopped deadline"),
                Band::NotApplicable => unreachable!("filtered above"),
            }
        }
    }
    for band in ["zero", "boundary", "over-bound"] {
        assert!(
            deadline_bands.contains_key(band),
            "the injected deterministic deadline has no {band} case"
        );
    }
}

/// The RDF 1.2 statement layer is inside the governed perimeter, not beside it.
///
/// A reification layer is a whole encoding a query can be written around: reifier and
/// annotation rows live in side tables, so a governor that counted only quads would report
/// a satisfied ceiling for a query that read — or emitted — an unbounded number of them.
/// The corpus therefore owes coverage on both halves: a charge over the reifier expansion,
/// and an answer cap that denominates output statements over a CONSTRUCTED reification
/// layer.
#[test]
fn the_rdf12_statement_layer_is_governed() {
    if updating() {
        return;
    }
    let cases = load_manifest();

    // The reifier expansion is charged at all.
    let reifier = case_named(&cases, "rdf12-reifier-fuel-boundary");
    assert!(
        consumed_in(&read_expected(reifier, "metered"), ResourceDimension::Fuel) > 0,
        "a reifier query that charges nothing is one no budget can bound"
    );

    // The answer cap denominates OUTPUT STATEMENTS over a constructed reification layer,
    // so the same solutions cost more answer units through a reifying template than they
    // do as plain rows. Read off the pinned records rather than recomputed, so this
    // asserts the published numbers.
    let construct = case_named(&cases, "rdf12-construct-answer-boundary");
    let statements = consumed_in(
        &read_expected(construct, "metered"),
        ResourceDimension::AnswerRows,
    );
    let rows = consumed_in(
        &read_expected(
            case_named(&cases, "rdf12-reifier-answer-boundary"),
            "metered",
        ),
        ResourceDimension::AnswerRows,
    );
    assert!(
        statements > rows,
        "the same solutions produced {statements} answer units through a reifying template \
         and {rows} as plain rows; were they equal the reifier rows would sit OUTSIDE the \
         cap, and a caller who capped the query would receive more statements than they \
         asked for"
    );

    // A truncated CONSTRUCT still hands back a certified prefix rather than a graph the
    // caller cannot place.
    let truncated = read_expected(
        case_named(&cases, "rdf12-construct-answer-over-bound"),
        "answer",
    );
    assert!(
        truncated.contains("\ncertificate\tcertain\t"),
        "a cap that stopped the GRAPH, not the pattern, leaves the rows their own positional \
         prefix and therefore a certified lower bound; got:\n{truncated}"
    );
}

/// A transport that cannot abandon an in-flight exchange still degrades to a known,
/// bounded amount of overshoot rather than to a silent one.
///
/// [`HttpRequest::stop`] can only be honoured by a transport capable of abandoning a call
/// it is already inside, and nothing forces a host to write one. What this pins is the
/// consequence: the two governors that act OUTSIDE the exchange — the remote-request
/// ceiling and the source's pre-dispatch poll — still bound the query at **per-request**
/// granularity, so a deaf transport costs the caller at most the exchange already in
/// flight, and reaches an outcome indistinguishable from an honouring transport's.
#[test]
fn a_transport_that_ignores_the_stop_signal_is_bounded_per_request() {
    if updating() {
        return;
    }
    let cases = load_manifest();
    let specs = load_transport();

    let deaf = case_named(&cases, "service-deaf-transport-cancel-mid-exchange");
    let honouring = case_named(&cases, "service-honouring-transport-cancel-mid-exchange");
    assert!(
        !specs[&deaf.name].honours_stop,
        "the deaf case must actually be deaf, or it pins nothing"
    );
    assert!(
        specs[&honouring.name].honours_stop,
        "the contrast case must actually poll the signal"
    );
    assert_eq!(
        specs[&deaf.name].posts, specs[&honouring.name].posts,
        "a transport that ignores the signal performed a different number of exchanges than \
         one that honours it — the degradation is then unbounded, not per-request"
    );
    assert_eq!(
        deaf.outcome, honouring.outcome,
        "the two transports must reach the same outcome: reading the signal inside the \
         exchange is an optimisation, never the thing that makes the query bounded"
    );
    let deaf_answer = read_expected(deaf, "answer");
    let honouring_answer = read_expected(honouring, "answer");
    assert_eq!(
        answer_rows(&deaf_answer),
        answer_rows(&honouring_answer),
        "the rows a deaf transport leaves in hand must be the rows an honouring one leaves"
    );

    // The one thing that genuinely differs — pinned rather than smoothed over, because
    // "known" is half of what this case claims. An honouring transport abandons the
    // exchange it is inside, so the truncation ORIGINATES at the dispatch and the (empty)
    // bag it hands back is a positional prefix of the answer. A deaf transport finishes
    // that exchange, its rows are ingested, and the trip is observed afterwards — so the
    // rows in hand are no longer the answer's first rows in order, and the certificate
    // says so instead of claiming a resumption point the caller does not have. Both
    // certify a LOWER bound either way; only the resumption licence is withdrawn.
    assert!(
        honouring_answer.contains("certificate\tcertain\tpositional-prefix=true"),
        "an abandoned exchange leaves a resumable prefix; got:\n{honouring_answer}"
    );
    assert!(
        deaf_answer.contains("certificate\tcertain\tpositional-prefix=false"),
        "a completed-then-discarded exchange must withdraw the resumption licence rather \
         than claim a prefix it cannot deliver; got:\n{deaf_answer}"
    );

    // And the request ceiling alone — no signal at all — still cuts between requests.
    let ceiling = case_named(&cases, "service-deaf-transport-request-ceiling");
    assert_eq!(ceiling.outcome, "budget-exhausted remote-requests");
    assert_eq!(
        specs[&ceiling.name].posts,
        Some(1),
        "the ceiling admits exactly the requests it was set to admit, whatever the transport \
         does with the signal it was handed"
    );
}

/// A host relation that cannot abandon an invocation it is already inside reaches the
/// **same** certified outcome, row for row, as one that can.
///
/// The property-function seam's half of the deaf-transport doctrine, and it lands harder
/// here than it does for `SERVICE`. `PfCursor::next` is handed no signal it is obliged to
/// read, and nothing forces a host to write a cursor that stops early — but a relation's
/// output is a row STREAM rather than one atomic exchange, and every row of it crosses the
/// engine's per-row admission point on its way into the bag. That point is also a bounded
/// work checkpoint, so a fired signal is observed there whether or not the relation ever
/// looked at it.
///
/// The two cases below therefore fire the caller's flag at the identical pull through a
/// cursor that abandons the invocation and one that ignores the signal entirely, and pin
/// that the results are indistinguishable: the deaf cursor's extra row is pulled and then
/// REFUSED at admission rather than ingested. Ignoring the signal buys the relation
/// nothing and costs the caller nothing — which is what "an optimisation, never the thing
/// that makes the query bounded" means, stated as evidence instead of as prose.
#[test]
fn a_relation_that_ignores_the_stop_signal_is_bounded_per_invocation() {
    if updating() {
        return;
    }
    let cases = load_manifest();
    let relations = load_relations();

    let deaf = case_named(
        &cases,
        "property-function-deaf-relation-cancel-mid-invocation",
    );
    let cooperating = case_named(
        &cases,
        "property-function-cooperating-relation-cancel-mid-invocation",
    );
    let deaf_spec = relations[&deaf.name];
    let cooperating_spec = relations[&cooperating.name];
    assert!(
        !deaf_spec.honours_stop,
        "the deaf case must actually be deaf, or it pins nothing"
    );
    assert!(
        cooperating_spec.honours_stop,
        "the contrast case must actually poll the signal"
    );
    assert_eq!(
        deaf_spec.cancel_on_pull, cooperating_spec.cancel_on_pull,
        "the two cases must fire the flag at the same pull, or they compare two timelines"
    );
    assert!(
        deaf_spec.cancel_on_pull.is_some_and(|pull| pull > 1),
        "the flag must fire PART WAY through an invocation; firing on the first pull would \
         test the poll before the cursor rather than the one between its rows"
    );
    assert_eq!(
        deaf.outcome, cooperating.outcome,
        "the two relations must reach the same outcome: reading the signal inside the \
         invocation is an optimisation, never the thing that makes the query bounded"
    );

    // Bounded per invocation: the flag fired inside the first one and neither relation was
    // opened again, whatever it did with the signal it was never handed.
    assert_eq!(deaf_spec.invocations, cooperating_spec.invocations);
    assert_eq!(
        deaf_spec.invocations,
        Some(1),
        "the flag fires inside the first invocation, so no second one may be opened"
    );
    let pulls = deaf_spec
        .pulls
        .expect("a pinned case carries its pull count");
    assert_eq!(
        pulls,
        deaf_spec.cancel_on_pull.expect("checked above"),
        "a deaf relation was pulled past the row that fired the flag: the degradation is \
         then unbounded, not per-row"
    );
    assert_eq!(
        deaf_spec.pulls, cooperating_spec.pulls,
        "a cooperating cursor abandons the pull that fired the flag and a deaf one answers \
         it; either way the engine asks exactly once more and then stops"
    );

    // And the answers: byte-identical, including the certificate. The deaf cursor answered
    // its last pull and that row was REFUSED at the per-row admission point, so it never
    // reached the bag the cooperating cursor also never filled.
    let deaf_answer = read_expected(deaf, "answer");
    let cooperating_answer = read_expected(cooperating, "answer");
    assert_eq!(
        deaf_answer, cooperating_answer,
        "a deaf relation must not change what the caller receives, nor what those rows are \
         certified to be"
    );
    assert!(
        deaf_answer.contains("certificate\tcertain\t"),
        "the rows in hand are every one an answer, so the bound is a certified lower one; \
         got:\n{deaf_answer}"
    );
    let rows = answer_rows(&deaf_answer)
        .lines()
        .filter(|line| line.starts_with("row"))
        .count();
    assert!(
        rows < usize::try_from(pulls).expect("a pull count fits a usize"),
        "the deaf cursor emitted more rows than reached the answer; if every pulled row \
         were ingested the per-row admission point would not be a checkpoint at all"
    );
}

/// The number of `row` records a pinned answer carries.
fn pinned_row_count(case: &Case) -> usize {
    read_expected(case, "answer")
        .lines()
        .filter(|line| line.starts_with("row"))
        .count()
}

/// The two property-function charge points are banded **separately**, not only jointly.
///
/// `property-function-invocation` and `property-function-row` both denominate fuel, so a
/// band on the aggregate is satisfied by any schedule whose total lands in the same place —
/// including one that doubled the invocation cost and halved the row cost. Two lanes exist
/// to take the pair apart, and this is the test that says what makes each of them isolating
/// rather than merely differently shaped:
///
/// - the **invocation** lane's relation emits nothing at all, so its band contains many
///   invocation charges and provably **zero** row charges. The seam really fires — the
///   pinned invocation count is what says so — and a row-cost change cannot move it.
/// - the **row** lane is driven by a single row, so its band contains exactly **one**
///   invocation charge and many row charges. An invocation-cost change moves it by one; a
///   row-cost change moves it by the emitted count.
///
/// Read off the pinned `.charges` decomposition rather than recomputed, so what this
/// asserts is the published numbers. There is deliberately no `zero` member on either lane:
/// a zero fuel ceiling trips at the first node entry, before any relation is resolved, so a
/// zero band would pin a case in which the seam never fires at all — which is exactly the
/// thing this test exists to rule out.
#[test]
fn the_two_property_function_charge_points_are_banded_separately() {
    if updating() {
        return;
    }
    let cases = load_manifest();
    let relations = load_relations();

    for band in ["boundary", "over-bound"] {
        let invocation = case_named(&cases, &format!("property-function-invocation-fuel-{band}"));
        let spec = relations[&invocation.name];
        assert_eq!(
            spec.emits, 0,
            "{}: the invocation lane isolates by emitting nothing; a relation that emits \
             rows puts row charges back into the band",
            invocation.name
        );
        let charges = read_expected(invocation, "charges");
        assert_eq!(
            charged_at(&charges, ChargePoint::PropertyFunctionRow),
            0,
            "{}: the invocation lane's fuel must contain no row charge at all",
            invocation.name
        );
        let invocations = charged_at(&charges, ChargePoint::PropertyFunctionInvocation);
        assert!(
            invocations > 1,
            "{}: the seam must actually fire, many times over — a band that charged the \
             invocation point once or not at all isolates nothing",
            invocation.name
        );
        let observed = spec
            .invocations
            .expect("a pinned case carries its invocation count");
        // At the boundary the whole measurement is admitted, so the charge point counts
        // exactly the invocations the relation observed. One unit below it, the trip
        // arrives first and the relation is opened strictly fewer times — which is the
        // charge point doing its job rather than a discrepancy.
        if band == "boundary" {
            assert_eq!(
                observed, invocations,
                "{}: the charge point counts the invocations the relation observed",
                invocation.name
            );
        } else {
            assert!(
                observed > 0 && observed < invocations,
                "{}: an over-bound ceiling must stop the drive part way through, having \
                 entered host code at least once",
                invocation.name
            );
        }
        assert_eq!(
            pinned_row_count(invocation),
            0,
            "{}: a relation that emits nothing hands back nothing",
            invocation.name
        );

        let row = case_named(&cases, &format!("property-function-row-fuel-{band}"));
        let spec = relations[&row.name];
        let charges = read_expected(row, "charges");
        assert_eq!(
            charged_at(&charges, ChargePoint::PropertyFunctionInvocation),
            1,
            "{}: the row lane isolates by driving exactly one invocation",
            row.name
        );
        assert_eq!(
            charged_at(&charges, ChargePoint::PropertyFunctionRow),
            spec.emits,
            "{}: every row the relation emitted crossed the admission point and was \
             charged there",
            row.name
        );
        assert!(
            spec.emits > 1,
            "{}: the row point must dominate its own lane, or the lane is a second \
             invocation band",
            row.name
        );
    }
}

/// [`the_two_property_function_charge_points_are_banded_separately`]'s twin for the
/// aggregate lane: `aggregate-invocation` and `aggregate-accumulation` both denominate
/// fuel, so a band on the aggregate alone is satisfied by any schedule whose total lands in
/// the same place — including one that doubled the invocation cost and halved the
/// accumulation cost. Two lanes take the pair apart:
///
/// - `aggregate-invocation-fuel-*` folds twelve aggregate expressions over one implicit
///   group whose input matches nothing, so its band carries twelve invocation charges and
///   provably **zero** accumulation charges — `COUNT(*)`'s empty-group answer is `0`, not
///   unbound, so the query still completes and commits a row.
/// - `aggregate-accumulation-fuel-*` folds one aggregate expression (`SUM`) over one
///   implicit group holding twelve rows, so its band carries exactly **one** invocation
///   charge and twelve accumulation charges.
///
/// Read off the pinned `.charges` decomposition rather than recomputed, so what this
/// asserts is the published numbers. Neither lane carries a `zero` member, for the same
/// reason the property-function lanes do not: a zero fuel ceiling trips at the first
/// algebra-node entry, before the `Group` node — let alone any aggregate expression — is
/// ever reached.
#[test]
fn the_two_aggregate_charge_points_are_banded_separately() {
    if updating() {
        return;
    }
    let cases = load_manifest();

    // `.charges` decomposes the unconstrained METERED measurement (see
    // `charge_decomposition_dataset`'s docs), exactly as a relation case's does — so a
    // lane's boundary and over-bound members share the identical record, and what it
    // shows is the shape of the LANE's full cost, not of whichever ceiling a given case
    // happens to set. That is what the boundary/over-bound pair's `.spend` and `.answer`
    // records are for instead: they pin what the ACTUAL banded run did.
    for band in ["boundary", "over-bound"] {
        let invocation = case_named(&cases, &format!("aggregate-invocation-fuel-{band}"));
        let charges = read_expected(invocation, "charges");
        assert_eq!(
            charged_at(&charges, ChargePoint::AggregateAccumulation),
            0,
            "{}: the invocation lane isolates by folding an empty group; a non-empty group \
             would put accumulation charges back into the band",
            invocation.name
        );
        let invocations = charged_at(&charges, ChargePoint::AggregateInvocation);
        assert_eq!(
            invocations, 12,
            "{}: the lane's full cost must attribute exactly twelve charges to the \
             invocation point — one per aggregate expression the query names",
            invocation.name
        );

        let accumulation = case_named(&cases, &format!("aggregate-accumulation-fuel-{band}"));
        let charges = read_expected(accumulation, "charges");
        assert_eq!(
            charged_at(&charges, ChargePoint::AggregateInvocation),
            1,
            "{}: the accumulation lane isolates by driving exactly one aggregate expression \
             over one implicit group",
            accumulation.name
        );
        let accumulated = charged_at(&charges, ChargePoint::AggregateAccumulation);
        assert_eq!(
            accumulated, 12,
            "{}: the lane's full cost must attribute exactly twelve charges to the \
             accumulation point — one per row the single group folds",
            accumulation.name
        );

        // The two lanes' full costs must not be interchangeable, or a schedule that
        // doubled `aggregate-invocation` and halved `aggregate-accumulation` (say) would
        // leave both `.charges` records looking the same.
        assert_ne!(
            charged_total(&read_expected(invocation, "charges")),
            charged_total(&read_expected(accumulation, "charges")),
            "the two lanes drive different query shapes and must not coincidentally cost \
             the same total fuel"
        );

        // The banded run itself. Both fixtures fold a SINGLE implicit group to completion
        // well inside their boundary ceiling — the fold's own charges (the twelve
        // `aggregate-invocation` or `aggregate-accumulation` units above) are a small
        // fraction of the lane's total cost, which also pays the generic per-node
        // accounting every node pays (`algebra-node-entry` on the way in,
        // `committed-output-row` on the way out for the group's own already-computed
        // row). So the over-bound ceiling — one unit below the boundary — lands on that
        // TRAILING generic charge rather than inside the fold itself: the group's one row
        // is fully computed either way, and what differs is only whether committing it
        // crosses the ceiling. The certificate says so directly: `Truncation::origin`
        // reports the node's own complete output as a positional prefix of itself, not an
        // opaque withholding — a caller paging by raising the ceiling by exactly one gets
        // the same row back, now as `complete` rather than `budget-exhausted`.
        for case in [invocation, accumulation] {
            let expected_outcome = if band == "boundary" {
                "complete"
            } else {
                "budget-exhausted fuel"
            };
            assert_eq!(case.outcome, expected_outcome, "{}", case.name);
            assert_eq!(
                pinned_row_count(case),
                1,
                "{}: the group's one row is computed in full before either ceiling is \
                 reached, so it is reported at both boundary and over-bound",
                case.name
            );
        }
    }

    // And the pair really is one fuel unit apart, exactly like every other boundary/
    // over-bound pair in the corpus — re-checked here rather than assumed, because it is
    // the fact that makes "over-bound" a meaningful name for the second member.
    for prefix in ["aggregate-invocation", "aggregate-accumulation"] {
        let boundary = case_named(&cases, &format!("{prefix}-fuel-boundary"));
        let over_bound = case_named(&cases, &format!("{prefix}-fuel-over-bound"));
        let boundary_fuel =
            consumed_in(&read_expected(boundary, "metered"), ResourceDimension::Fuel);
        let (dimension, ceiling) = boundary.ceiling.expect("a band case sets a ceiling");
        assert_eq!(dimension, ResourceDimension::Fuel);
        assert_eq!(ceiling, boundary_fuel);
        let (_, over_bound_ceiling) = over_bound.ceiling.expect("a band case sets a ceiling");
        assert_eq!(over_bound_ceiling, boundary_fuel - 1);
    }
}

/// The custom-aggregate path's fuel, compared DIRECTLY against the built-in path's over the
/// identical group shape.
///
/// `aggregate-accumulation-*` and `aggregate-custom-*` share one dataset
/// (`cases/aggregate-accumulation.ttl`, twelve rows, no `GROUP BY`) and differ only in
/// whether the aggregate expression names the built-in `SUM` or a registered
/// [`CustomAggregate`](purrdf_sparql_eval::CustomAggregate)
/// ([`CorpusSumAggregate`]) — `AggregateInvocation`/`AggregateAccumulation` are charged from
/// the ONE call site in the evaluator that dispatches either kind (see
/// `crate::modifier::eval_aggregate`'s docs in the evaluator crate), so this corpus's own
/// in-crate counterpart
/// (`a_custom_aggregate_spends_the_same_invocation_and_accumulation_fuel_as_a_built_in_over_the_same_group_shape`
/// in `purrdf_sparql_eval::governor::charge_points`) is not the only place that claim is
/// checked — it is checked here too, against the FROZEN corpus, so a regression that moved
/// only the custom path's cost cannot hide behind a green in-crate unit test alone.
#[test]
fn a_custom_aggregate_costs_the_same_fuel_as_a_built_in_over_the_same_group_shape() {
    if updating() {
        return;
    }
    let cases = load_manifest();

    let built_in = case_named(&cases, "aggregate-accumulation-fuel-boundary");
    let custom = case_named(&cases, "aggregate-custom-fuel-boundary");
    assert_eq!(
        built_in.data, custom.data,
        "the two lanes must share the identical dataset — the fuel comparison means \
         nothing if the group shape differs"
    );
    let built_in_fuel = consumed_in(&read_expected(built_in, "metered"), ResourceDimension::Fuel);
    let custom_fuel = consumed_in(&read_expected(custom, "metered"), ResourceDimension::Fuel);
    assert_eq!(
        built_in_fuel, custom_fuel,
        "the built-in and custom aggregate paths must cost the SAME fuel over the same \
         group shape — a regression here is a regression in the custom-aggregate charge \
         path this corpus otherwise never exercises at all"
    );
}

/// The custom-aggregate path's `ScratchBytes` charge is a pure function of the group's row
/// count — never of the host's thread count.
///
/// `aggregate-custom-scratch-bytes-*` drives 1200 rows into ONE implicit group (no
/// `GROUP BY`), well above the within-group chunk threshold
/// ([`PARALLEL_FORK_MIN_ROWS`]), through [`CorpusSumAggregate`]'s declared nonzero
/// [`CustomAggregate::state_bound`](purrdf_sparql_eval::CustomAggregate::state_bound). Before
/// the within-group chunk planner was made a pure function of row count, this exact charge
/// scaled with `rayon::current_num_threads()`, so the SAME query/data/ceiling triple this
/// pair pins would admit on one host and refuse on another — see
/// `purrdf_sparql_eval::parallel::aggregate_chunk_size_for`'s doc comment. Freezing this
/// pair's boundary and over-bound `ScratchBytes` ceilings turns that host-invariance claim
/// into something `python3 scripts/check-corpus-frozen.py` (hence CI, on whatever host it
/// happens to run on) checks on every run, not merely something an in-crate test asserts on
/// the host that happened to build it.
#[test]
fn the_custom_aggregate_scratch_bytes_band_exercises_the_folds_retained_state() {
    if updating() {
        return;
    }
    let cases = load_manifest();

    let boundary = case_named(&cases, "aggregate-custom-scratch-bytes-boundary");
    let over_bound = case_named(&cases, "aggregate-custom-scratch-bytes-over-bound");
    let boundary_bytes = consumed_in(
        &read_expected(boundary, "metered"),
        ResourceDimension::ScratchBytes,
    );
    assert!(
        boundary_bytes > 0,
        "the lane must actually cost scratch bytes, or it bands nothing"
    );
    let (dimension, ceiling) = boundary.ceiling.expect("a band case sets a ceiling");
    assert_eq!(dimension, ResourceDimension::ScratchBytes);
    assert_eq!(ceiling, boundary_bytes);
    let (_, over_bound_ceiling) = over_bound.ceiling.expect("a band case sets a ceiling");
    assert_eq!(over_bound_ceiling, boundary_bytes - 1);
    assert_eq!(boundary.outcome, "complete");
    assert_eq!(over_bound.outcome, "budget-exhausted scratch-bytes");
}

/// The parallel drive lane is genuinely above the evaluator's fork threshold.
///
/// Stated against the pinned counts rather than against the evaluator's internals, which
/// are crate-private: the relation is opened once per driving row, so its pinned invocation
/// count IS the number of rows the BGP folded into the call. See
/// [`PARALLEL_FORK_MIN_ROWS`] for what this constant is a copy of, and for why a raised
/// threshold makes this test fail loudly instead of leaving the lane silently inert.
#[test]
fn the_parallel_drive_is_actually_above_the_fork_threshold() {
    if updating() {
        return;
    }
    let cases = load_manifest();
    let relations = load_relations();
    let boundary = case_named(&cases, "property-function-parallel-fuel-boundary");
    let driving = relations[&boundary.name]
        .invocations
        .expect("a pinned case carries its invocation count");
    assert!(
        driving > u64::try_from(PARALLEL_FORK_MIN_ROWS).expect("the threshold fits a u64"),
        "the parallel lane drove {driving} rows, which is at or below the {PARALLEL_FORK_MIN_ROWS}-row \
         fork threshold: the lane then measures the same sequential driver every other case \
         does, and the parity it claims is vacuous"
    );
}

/// The parallel driver spends exactly what the sequential driver spends.
///
/// The claim the whole corpus rests on is that consumption is a pure function of
/// `(query, data, budget)` — not of the worker count, the chunk geometry, or the completion
/// order. The parallel lane is where that is checked against a real fork, and it needs no
/// test-only seam to arrange one:
///
/// - [`QueryGovernors::METERED`] engages the intermediate-cell counter, and a BGP stage with
///   a cell ceiling folds through the evaluator's **sequential** cell-bounded driver — a
///   parallel chunk cannot enforce a ceiling it cannot know the running total of. So the
///   `.metered` measurement of this case is a sequential run.
/// - a band case sets **fuel alone**, leaving the cell counter disengaged, so the same BGP
///   stage folds through the **parallel** chunk driver instead.
///
/// The boundary case therefore completes under a ceiling that is exactly what a *sequential*
/// run cost, and the over-bound case — one unit below it — must trip. Byte-equal totals
/// would already be a strong statement; the over-bound member makes it the sharper one,
/// because a ceiling one below the measurement can only be crossed at all if the parallel
/// fold charged the same items in the same order.
#[test]
fn the_parallel_drive_spends_what_the_sequential_measurement_spent() {
    if updating() {
        return;
    }
    let cases = load_manifest();
    let relations = load_relations();

    let boundary = case_named(&cases, "property-function-parallel-fuel-boundary");
    let over_bound = case_named(&cases, "property-function-parallel-fuel-over-bound");
    let measured = consumed_in(&read_expected(boundary, "metered"), ResourceDimension::Fuel);
    assert_eq!(
        boundary.ceiling,
        Some((ResourceDimension::Fuel, measured)),
        "the parallel boundary must be the sequential measurement, exactly"
    );
    assert_eq!(boundary.outcome, "complete");
    assert_eq!(
        over_bound.ceiling,
        Some((ResourceDimension::Fuel, measured - 1))
    );
    assert_eq!(over_bound.outcome, "budget-exhausted fuel");

    // And the run really is parallel-driven: the observation is taken live, under the
    // manifest's own ceiling, and must reproduce the pinned spend of a measurement the
    // sequential driver produced.
    let spec = relations.get(&boundary.name).copied();
    let observation = observe(boundary, &governors_for(boundary), None, spec);
    assert_eq!(
        consumed_in(&observation.spend, ResourceDimension::Fuel),
        measured,
        "the parallel drive spent a different amount than the sequential measurement: a \
         receipt that depends on which driver ran is not a receipt anyone can size a budget \
         against"
    );
}

/// A fuel ceiling that lands **between** an invocation and its first row truncates at a
/// pinned, deterministic point.
///
/// The narrowest window the property-function seam has, and the one an aggregate band can
/// never sit in: the relation was entered — host code ran, a cursor was opened, one row was
/// produced — and then the budget refused that row at the engine's admission point. The
/// caller receives an empty bag that is nevertheless a certified positional prefix, and the
/// pinned pull count is what separates this from "the ceiling prevented the call".
///
/// The ceiling is authored rather than derived, exactly as
/// `service-deaf-transport-request-ceiling`'s is: it names a seam, not a band. Authored is
/// not unchecked, though — the assertions below re-derive that it is the *unique* such
/// value, by re-running the same case one unit higher and finding exactly one more pull and
/// exactly one admitted row.
#[test]
fn a_fuel_ceiling_between_an_invocation_and_its_first_row_truncates_deterministically() {
    if updating() {
        return;
    }
    let cases = load_manifest();
    let relations = load_relations();
    let case = case_named(&cases, "property-function-first-row-fuel-ceiling");
    let spec = relations[&case.name];
    let (dimension, ceiling) = case.ceiling.expect("the seam case sets a fuel ceiling");
    assert_eq!(dimension, ResourceDimension::Fuel);
    assert_eq!(case.band, Band::NotApplicable, "a seam, not a band");
    assert_eq!(case.outcome, "budget-exhausted fuel");

    // Above the invocation charge: host code was entered and asked for a row.
    assert_eq!(
        spec.invocations,
        Some(1),
        "a ceiling below the invocation charge would pin a case in which the seam never \
         fired at all"
    );
    assert_eq!(
        spec.pulls,
        Some(1),
        "below the first row charge: the cursor answered exactly one pull, and the engine \
         never asked for a second"
    );

    // Below the first row charge: nothing that pull produced reached the caller. The bag is
    // empty and is still CERTIFIED — every row in it is an answer, vacuously — but the
    // resumption licence is withdrawn, and that is pinned rather than smoothed over. The
    // call is driven laterally, so its own emission order is not the joined output's order;
    // a truncation originating inside it bounds the answer from below without saying where
    // in the answer the cut fell. (The standalone shape, where the call IS the node under
    // the projection, keeps the licence — `property-fn-eval`'s own truncation test pins
    // that half.)
    let answer = read_expected(case, "answer");
    assert_eq!(pinned_row_count(case), 0, "got:\n{answer}");
    assert!(
        answer.contains("certificate\tcertain\tpositional-prefix=false"),
        "an empty bag from inside a driven call is a certified lower bound that licenses no \
         resumption point; got:\n{answer}"
    );

    // The ceiling is the unique value with that property: one unit more admits exactly one
    // row, from exactly one more pull.
    let mut admitting = case.clone();
    admitting.ceiling = Some((ResourceDimension::Fuel, ceiling + 1));
    let observation = observe(&admitting, &governors_for(&admitting), None, Some(spec));
    assert_eq!(
        (observation.invocations, observation.pulls),
        (1, 2),
        "one unit above the seam must buy exactly the first row's charge"
    );
    assert_eq!(
        observation
            .answer
            .lines()
            .filter(|line| line.starts_with("row"))
            .count(),
        1,
        "one unit above the seam must admit exactly one row"
    );
}

/// Two runs of the whole corpus agree, byte for byte, on rows and on cost.
///
/// Determinism is the property the corpus exists to publish, so it is checked rather than
/// asserted in prose. Charges are accumulated chunk-locally and folded in source-item
/// order, so a second run of the same query over the same data under the same ceilings has
/// to reach the same trip point — on any worker count, in any completion order.
#[test]
fn the_corpus_is_reproducible_within_a_run() {
    if updating() {
        return;
    }
    let cases = load_manifest();
    let specs = load_transport();
    let relations = load_relations();
    for case in &cases {
        if !case.is_pinned_deterministic() {
            continue;
        }
        let spec = specs.get(&case.name).copied();
        let relation_spec = relations.get(&case.name).copied();
        let first = observe(case, &governors_for(case), spec, relation_spec);
        let second = observe(case, &governors_for(case), spec, relation_spec);
        assert_eq!(
            first, second,
            "{} did not reproduce: a governed run whose outcome depends on the run is not a \
             governor anyone can size a budget against",
            case.name
        );
    }
}
