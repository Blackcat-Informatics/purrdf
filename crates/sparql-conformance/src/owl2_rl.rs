// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The vendored W3C OWL 2 **entailment** corpus: ingester, grader, and ledger.
//!
//! # Why this module exists
//!
//! [`crate::owl2`] grades *satisfiability*: it asks the `OWL-Direct` tableau
//! whether an ontology has a model. That says nothing about the OWL 2 **RL** rule
//! table, which is a forward-materialization chase over a declared rule program.
//! Until this module existed, the only thing grading that chase was fixtures
//! written by the same change that wrote the rules — the implementation and its
//! oracle had a common author.
//!
//! This module is the independent oracle. Its cases are W3C's, taken from
//! <https://www.w3.org/2009/11/owl-test/all.rdf>, and it grades PurRDF's
//! [`purrdf_entail::Regime::OwlRl`] closure against W3C's published entailment
//! verdicts.
//!
//! # What is graded, and how
//!
//! Two lanes, distinguished by which target file a case directory carries:
//!
//! * **positive** (`conclusion.rdf`) — the W3C published
//!   `otest:PositiveEntailmentTest`: the premise entails the conclusion. PurRDF
//!   passes when the OWL-RL closure of the premise *simple-entails* the
//!   conclusion, i.e. the conclusion graph maps into the closure with its blank
//!   nodes read as existentials. The vendored positives are exactly the cases
//!   W3C declares `otest:profile RL` under `otest:semantics RDF-BASED`.
//! * **negative** (`non-conclusion.rdf`) — the W3C published
//!   `otest:NegativeEntailmentTest`: the premise does **not** entail the
//!   non-conclusion. Because a rule chase is *sound*, deriving it would be an
//!   unsoundness, so PurRDF passes when the non-conclusion does **not** map into
//!   the closure. This lane is profile-independent: soundness is owed on every
//!   case, not only on RL-profile ones.
//!
//! # What `otest:profile RL` does and does not promise
//!
//! It says the case's *ontology* is inside the OWL 2 RL profile. It does **not**
//! say the OWL 2 RL/RDF rule table (Profiles §4.3) reaches the conclusion: the
//! measured ledger below shows W3C tagging cases `RL` whose **conclusion** uses
//! `owl:complementOf`, which is not even in the RL syntax. So a divergence on one
//! of these 27 is not automatically a defect — but it is always something the
//! ledger must name concretely, and [`RlGap`] draws the line between "the rule
//! table omits a sound, in-shape rule" ([`RlGap::MissingRule`]) and "no rule of
//! this shape could have that head" ([`RlGap::NegativeConclusion`],
//! [`RlGap::SchemaConclusion`], [`RlGap::ConstructOutsideRl`]).
//!
//! Grading a positive case is one-sided in the honest direction: matching proves
//! entailment (the chase is sound), and failing to match is always a real,
//! reportable limit of the lane. Grading a negative case is one-sided the other
//! way: a match is a proven unsoundness, and a non-match is the expected answer —
//! which makes the negative lane a *soundness* gate with weak discriminating
//! power, and it is reported as such rather than counted as if it proved
//! completeness. The positive lane carries the discrimination: a reasoner that
//! derives nothing at all fails all 27 positive cases.
//!
//! # Three outcomes, never two
//!
//! Matching [`crate::owl2`]'s discipline, a run has three buckets ([`Grade`]):
//!
//! * **agree** — PurRDF's closure gave the published answer;
//! * **withhold** — PurRDF *refused to decide*: the RDF/XML would not parse, the
//!   chase returned an [`EntailError`](purrdf_entail::EntailError), or the
//!   blank-node match exhausted its budget. A refusal is a capability gap and is
//!   never scored as a pass;
//! * **disagree** — PurRDF produced a closure and it gave the other answer.
//!
//! Every withhold and every disagreement must appear in [`LEDGER`] with a typed
//! [`RlGap`]. An unledgered one fails the harness, a ledgered one that starts
//! agreeing fails it, and a ledger entry naming a case that is not vendored fails
//! it — so the ledger can neither rot nor inflate the budget.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use purrdf_core::{RdfTextDirection, TermRef};

/// The blank-node match budget, in candidate triples visited.
///
/// The conclusion graphs are tiny (the largest vendored one is 15 triples), so a
/// budget this size is never reached by a well-formed case; it exists so that a
/// pathological conclusion produces a **withhold** — an honest refusal that must
/// be ledgered — instead of an unbounded search inside a required gate.
const MATCH_BUDGET: u64 = 5_000_000;

/// What the W3C published for a case, which is also which target file it carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// `otest:PositiveEntailmentTest`: the premise entails `conclusion.rdf`.
    Positive,
    /// `otest:NegativeEntailmentTest`: the premise does *not* entail
    /// `non-conclusion.rdf`.
    Negative,
}

impl Direction {
    /// The target file name a case of this direction carries.
    #[must_use]
    pub const fn target_file(self) -> &'static str {
        match self {
            Self::Positive => "conclusion.rdf",
            Self::Negative => "non-conclusion.rdf",
        }
    }

    /// The token used in the census and in the harness log.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negative => "negative",
        }
    }

    /// Whether a closure that *matches* the target is the published answer.
    #[must_use]
    pub const fn expects_match(self) -> bool {
        matches!(self, Self::Positive)
    }
}

/// Why PurRDF's OWL 2 RL lane diverges from the published verdict on a ledgered
/// case.
///
/// Each variant names the concrete rule or construct responsible, so the ledger
/// doubles as an inventory of what the rule table does not yet do. A catch-all
/// would defeat the point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RlGap {
    /// The conclusion is a **positive assertional triple over named terms** that a
    /// sound rule of OWL 2 RL's own shape would produce, and no such rule fires.
    ///
    /// Read this variant carefully, because it is where the "`OWL-RL` 78 / 78"
    /// headline and W3C conformance come apart. 78 / 78 is a statement about
    /// *table coverage*: PurRDF implements every one of the 78 rules of OWL 2
    /// Profiles §4.3 Tables 4–9. It is not a statement about entailment
    /// conformance, and this variant is the case where both are true at once —
    /// the missing rule is sound and shaped exactly like a rule the table already
    /// has, yet **it is not one of the 78**, so complete coverage of the table
    /// still does not reach the conclusion.
    ///
    /// Closing one of these means adding a rule *beyond* the normative table,
    /// which is a decision with a cost (the lane would no longer be exactly
    /// Tables 4–9) and is why the scoreboard counts these separately instead of
    /// burying them among the profile's structural limits.
    MissingRule,
    /// The conclusion is a **schema axiom** — a property characteristic, an
    /// `rdfs:range`, an anonymous class expression, an `owl:AllDifferent`
    /// collection. Every head in the OWL 2 RL/RDF rule table (Profiles §4.3) is
    /// either an assertional triple over named terms or `false`; not one concludes
    /// a new axiom of these shapes, so no conforming RL rule set derives them.
    SchemaConclusion,
    /// The conclusion is a **negative fact** — an `owl:differentFrom`, or
    /// membership in an `owl:complementOf` class. It follows from the premise only
    /// by refutation (assume the negation, reach `false`), and a forward chase
    /// over definite rules cannot perform refutation: the RL rule table sends every
    /// contradiction to `false` and never turns one back into a conclusion.
    NegativeConclusion,
    /// The entailment turns on an OWL 2 construct **outside the OWL 2 RL syntax**
    /// (`owl:ReflexiveProperty`, …), for which the profile's rule table states no
    /// rule at all.
    ConstructOutsideRl,
    /// The premise `owl:imports` a document the upstream manifest keeps as a
    /// separate support file rather than inline, so the vendored premise is not
    /// the whole premise and **no** reasoner could reach the conclusion from what
    /// is vendored. Ledgered rather than dropped, so the incompleteness of the
    /// upstream export stays visible.
    ImportsUnresolved,
    /// The premise or target RDF/XML did not parse, or the chase returned an
    /// error, so the run refused to decide.
    Refused,
    /// The chase derived a triple the W3C says is **not** entailed. This is an
    /// unsoundness: PurRDF asserted a conclusion it is not entitled to.
    UnsoundDerivation,
}

impl RlGap {
    /// A short human-readable label for the ledger tally and the log.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::MissingRule => "missing-rule",
            Self::SchemaConclusion => "schema-conclusion",
            Self::NegativeConclusion => "negative-conclusion",
            Self::ConstructOutsideRl => "construct-outside-rl",
            Self::ImportsUnresolved => "imports-unresolved",
            Self::Refused => "refused",
            Self::UnsoundDerivation => "unsound-derivation",
        }
    }

    /// Whether this gap is an **unsoundness** — PurRDF asserting a conclusion the
    /// W3C contradicts — rather than an incompleteness.
    ///
    /// The distinction is the ledger's most consequential one: an incompleteness
    /// withholds a conclusion PurRDF is entitled to, whereas an unsoundness
    /// asserts one it is not. The match is exhaustive on purpose, so a new gap
    /// cannot be added without classifying itself here.
    #[must_use]
    pub const fn is_unsound(self) -> bool {
        match self {
            Self::MissingRule
            | Self::SchemaConclusion
            | Self::NegativeConclusion
            | Self::ConstructOutsideRl
            | Self::ImportsUnresolved
            | Self::Refused => false,
            Self::UnsoundDerivation => true,
        }
    }

    /// Whether this gap names a conclusion a **sound rule of RL's own shape**
    /// could reach — as opposed to one no rule of that shape can have as a head,
    /// or a defect in the vendored premise.
    ///
    /// The scoreboard reports this count separately, because it is the only part
    /// of the ledger that is a decision to take rather than a description of the
    /// profile's structural limits.
    #[must_use]
    pub const fn is_actionable(self) -> bool {
        match self {
            Self::MissingRule | Self::UnsoundDerivation => true,
            Self::SchemaConclusion
            | Self::NegativeConclusion
            | Self::ConstructOutsideRl
            | Self::ImportsUnresolved
            | Self::Refused => false,
        }
    }
}

/// One ledgered divergence: the case's directory name plus its typed gap.
#[derive(Debug)]
pub struct LedgerEntry {
    /// The case directory name under `entailment-suite/w3c-owl2-rl/cases/`.
    pub case: &'static str,
    /// Why PurRDF diverges.
    pub gap: RlGap,
}

/// The divergence ledger: every vendored entailment case PurRDF does not answer
/// as the W3C published it.
///
/// Nothing is skipped at discovery time — all 50 cases run, and a case absent
/// from this table must agree.
pub const LEDGER: &[LedgerEntry] = &[
    // --- MISSING RULE: `owl:differentFrom` is not closed under symmetry -------
    //     The whole premise of `webont-differentfrom-001` is `a owl:differentFrom
    //     b`, and its whole conclusion is `b owl:differentFrom a`. That is
    //     symmetry: the head is a positive assertional triple over two named
    //     individuals, exactly the shape `prp-symp` already has, and stating it is
    //     sound.
    //
    //     And it is NOT one of the 78 rules of OWL 2 Profiles §4.3 Tables 4–9.
    //     Table 4's `owl:differentFrom` rules (`eq-diff1..3`) only ever conclude
    //     `false`; nothing in the normative table has an `owl:differentFrom` head.
    //     So PurRDF implements the whole table, the chase still stops one triple
    //     short of a W3C-published entailment, and BOTH facts are true. This entry
    //     is the concrete reason "OWL-RL 78 / 78" must not be read as "conformant
    //     on W3C's OWL 2 RL entailment tests".
    LedgerEntry {
        case: "webont-differentfrom-001",
        gap: RlGap::MissingRule,
    },
    // --- NEGATIVE CONCLUSION: only reachable by refutation --------------------
    //     Each of these concludes a negative fact — an `owl:differentFrom`, or
    //     membership in an anonymous `owl:complementOf` class. The premise refutes
    //     the opposite (`prp-fp` / `prp-ifp` / `prp-pdw` / `cax-dw` would send the
    //     negation to `false`), but a forward chase over definite rules cannot run
    //     that argument backwards: OWL 2 RL's rule table has no rule whose head is
    //     a negative fact, so no conforming RL rule set derives these either.
    LedgerEntry {
        case: "disjointclasses-001",
        gap: RlGap::NegativeConclusion,
    },
    LedgerEntry {
        case: "disjointclasses-003",
        gap: RlGap::NegativeConclusion,
    },
    LedgerEntry {
        case: "new-feature-disjointobjectproperties-001",
        gap: RlGap::NegativeConclusion,
    },
    LedgerEntry {
        case: "new-feature-objectqcr-002",
        gap: RlGap::NegativeConclusion,
    },
    LedgerEntry {
        case: "owl2-rl-rules-fp-differentfrom",
        gap: RlGap::NegativeConclusion,
    },
    LedgerEntry {
        case: "owl2-rl-rules-ifp-differentfrom",
        gap: RlGap::NegativeConclusion,
    },
    // --- SCHEMA CONCLUSION: the head shape does not exist in the rule table ---
    //     `chain2trans1` concludes `p rdf:type owl:TransitiveProperty`;
    //     `new-feature-disjoint{data,object}properties-002` conclude an
    //     `owl:AllDifferent` collection; `webont-i5-26-010` concludes an anonymous
    //     `owl:Restriction`; `webont-i5-5-005` concludes an anonymous
    //     `owl:unionOf` class; the three `webont-i5-8-*` cases conclude an
    //     `rdfs:range` narrowed to an XSD datatype. Every head in OWL 2 RL/RDF's
    //     rule table is an assertional triple over named terms or `false`; not one
    //     concludes an axiom, so these are outside what the profile's rule set can
    //     produce rather than outside what this implementation happens to do.
    LedgerEntry {
        case: "chain2trans1",
        gap: RlGap::SchemaConclusion,
    },
    LedgerEntry {
        case: "new-feature-disjointdataproperties-002",
        gap: RlGap::SchemaConclusion,
    },
    LedgerEntry {
        case: "new-feature-disjointobjectproperties-002",
        gap: RlGap::SchemaConclusion,
    },
    LedgerEntry {
        case: "webont-i5-26-010",
        gap: RlGap::SchemaConclusion,
    },
    LedgerEntry {
        case: "webont-i5-5-005",
        gap: RlGap::SchemaConclusion,
    },
    LedgerEntry {
        case: "webont-i5-8-006",
        gap: RlGap::SchemaConclusion,
    },
    LedgerEntry {
        case: "webont-i5-8-008",
        gap: RlGap::SchemaConclusion,
    },
    LedgerEntry {
        case: "webont-i5-8-009",
        gap: RlGap::SchemaConclusion,
    },
    // --- CONSTRUCT OUTSIDE OWL 2 RL -------------------------------------------
    //     `owl:ReflexiveProperty` is not in the OWL 2 RL syntax, so Profiles §4.3
    //     states no `prp-rfl` rule to fire. W3C still tags the case `otest:profile
    //     RL`, which is why the profile tag is a selector for this corpus and not
    //     a promise about the rule table.
    LedgerEntry {
        case: "new-feature-reflexiveproperty-001",
        gap: RlGap::ConstructOutsideRl,
    },
    // --- THE VENDORED PREMISE IS NOT THE WHOLE PREMISE ------------------------
    //     `webont-imports-011`'s premise `owl:imports` `support011-A`, which the
    //     upstream `all.rdf` export does not carry inline — the conclusion is about
    //     a class defined only in that support document. No reasoner reaches it
    //     from the vendored bytes. Ledgered rather than dropped so the gap in the
    //     upstream export stays visible instead of being silently deselected.
    LedgerEntry {
        case: "webont-imports-011",
        gap: RlGap::ImportsUnresolved,
    },
];

/// Look a case up in [`LEDGER`].
#[must_use]
pub fn ledger_lookup(case: &str) -> Option<RlGap> {
    LEDGER.iter().find(|e| e.case == case).map(|e| e.gap)
}

/// One vendored entailment case.
#[derive(Debug)]
pub struct RlCase {
    /// The case directory name, the slugified `otest:identifier`.
    pub name: String,
    /// The verbatim W3C RDF/XML premise ontology.
    pub premise: PathBuf,
    /// The verbatim W3C RDF/XML conclusion / non-conclusion ontology.
    pub target: PathBuf,
    /// Which of the two the target is.
    pub direction: Direction,
}

/// What PurRDF answered for a case.
#[derive(Debug)]
pub enum Answer {
    /// The OWL-RL closure of the premise simple-entails the target graph.
    Entailed,
    /// The closure was computed and does **not** entail the target graph,
    /// carrying the diagnosis of what was missing.
    NotEntailed(MissReason),
    /// The run refused to answer, carrying the refusal's own message.
    Withheld(String),
}

/// How PurRDF's answer compares to the published verdict.
#[derive(Debug)]
pub enum Grade {
    /// Answered, and matching the published verdict.
    Agree,
    /// Refused to answer, with the refusal's message.
    Withhold(String),
    /// Answered, and contradicting the published verdict.
    Disagree {
        /// The direction the W3C published.
        published: Direction,
        /// `Some(reason)` when the closure LACKED the target graph — which
        /// contradicts a `Positive` case (an incompleteness). `None` when the
        /// closure CONTAINED it — which contradicts a `Negative` case (an
        /// unsoundness).
        miss: Option<MissReason>,
    },
}

/// The vendored corpus root (`entailment-suite/w3c-owl2-rl`).
#[must_use]
pub fn suite_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("entailment-suite/w3c-owl2-rl")
}

/// Discover every vendored case under `root/cases`, in case-name order.
///
/// A case's direction is read from which target file it carries, so there is no
/// derived metadata file to drift out of step with the payload: a directory with
/// both targets, or neither, is a hard error rather than a silent default.
///
/// # Errors
///
/// Returns a message if the corpus root cannot be read, if it holds a
/// non-directory entry, or if a case directory does not hold exactly a
/// `premise.rdf` plus exactly one of `conclusion.rdf` / `non-conclusion.rdf`.
pub fn discover(root: &Path) -> Result<Vec<RlCase>, String> {
    let cases_dir = root.join("cases");
    let mut names: Vec<String> = Vec::new();
    let entries = std::fs::read_dir(&cases_dir)
        .map_err(|e| format!("cannot read {}: {e}", cases_dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot read {}: {e}", cases_dir.display()))?;
        let file_type = entry
            .file_type()
            .map_err(|e| format!("cannot stat {}: {e}", entry.path().display()))?;
        if !file_type.is_dir() {
            return Err(format!(
                "{}: unexpected non-directory entry in the corpus root",
                entry.path().display()
            ));
        }
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    names.sort();

    let mut cases = Vec::with_capacity(names.len());
    for name in names {
        let dir = cases_dir.join(&name);
        let premise = dir.join("premise.rdf");
        if !premise.is_file() {
            return Err(format!("{}: missing premise", premise.display()));
        }
        let positive = dir.join(Direction::Positive.target_file());
        let negative = dir.join(Direction::Negative.target_file());
        let direction = match (positive.is_file(), negative.is_file()) {
            (true, false) => Direction::Positive,
            (false, true) => Direction::Negative,
            (true, true) => {
                return Err(format!(
                    "{}: carries BOTH conclusion.rdf and non-conclusion.rdf; a case is one \
                     direction or the other",
                    dir.display()
                ));
            }
            (false, false) => {
                return Err(format!(
                    "{}: carries neither conclusion.rdf nor non-conclusion.rdf",
                    dir.display()
                ));
            }
        };
        let target = if direction.expects_match() {
            positive
        } else {
            negative
        };
        cases.push(RlCase {
            name,
            premise,
            target,
            direction,
        });
    }
    Ok(cases)
}

/// An owned RDF term, comparable across two independently-parsed datasets.
///
/// [`TermRef`] borrows into one dataset's term table and carries the datatype as
/// an interned id local to it, so it cannot be compared with a term from another
/// dataset. This is the resolved, owned form used for matching. Blank nodes keep
/// their scope because two blank nodes with the same label in different scopes are
/// different nodes (`purrdf_core` C0.2).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Term {
    Iri(String),
    Blank(String, u32),
    Literal {
        lexical: String,
        datatype: String,
        language: Option<String>,
        direction: Option<RdfTextDirection>,
    },
    Triple(Box<[Self; 3]>),
}

/// A `(subject, predicate, object)` triple of owned terms.
type Triple = [Term; 3];

/// Resolve one [`TermRef`] into an owned [`Term`].
fn own_term(ds: &purrdf_core::RdfDataset, term: TermRef<'_>) -> Term {
    match term {
        TermRef::Iri(iri) => Term::Iri(iri.to_owned()),
        TermRef::Blank { label, scope } => Term::Blank(label.to_owned(), scope.ordinal()),
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => Term::Literal {
            lexical: lexical.to_owned(),
            datatype: match ds.resolve(datatype) {
                TermRef::Iri(iri) => iri.to_owned(),
                other => format!("{other:?}"),
            },
            language: language.map(str::to_owned),
            direction,
        },
        TermRef::Triple { s, p, o } => Term::Triple(Box::new([
            own_term(ds, ds.resolve(s)),
            own_term(ds, ds.resolve(p)),
            own_term(ds, ds.resolve(o)),
        ])),
    }
}

/// Every default-graph triple of `ds`, as owned terms.
///
/// The RDF/XML codec puts a document's whole content in the default graph and the
/// chase derives into it, so restricting to it loses nothing and keeps a stray
/// named graph from being read as an entailment.
fn owned_triples(ds: &purrdf_core::RdfDataset) -> Vec<Triple> {
    ds.quad_refs()
        .filter(|q| q.g.is_none())
        .map(|q| [own_term(ds, q.s), own_term(ds, q.p), own_term(ds, q.o)])
        .collect()
}

/// Parse an RDF/XML document with PurRDF's first-party codec.
///
/// The base IRI is synthetic and `example.org`-scoped, per the repository's
/// no-fabricated-vocabulary rule. Every vendored document declares its own
/// `xml:base`, so the supplied base is not consulted (asserted by the corpus
/// tripwire in the harness).
fn parse(path: &Path, base: &str) -> Result<std::sync::Arc<purrdf_core::RdfDataset>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    purrdf::parse_dataset(&bytes, "application/rdf+xml", Some(base))
        .map_err(|e| format!("RDF/XML parse of {}: {e}", path.display()))
}

/// A blank-node variable: `(label, scope)`.
type VarKey = (String, u32);

/// Whether `pat` unifies with `ground`, binding blank-node variables as it goes.
///
/// Every key this newly binds is recorded on `trail`, so a caller whose later
/// patterns fail can undo exactly the bindings this attempt introduced.
fn try_unify(
    pat: &Term,
    ground: &Term,
    bound: &mut BTreeMap<VarKey, Term>,
    trail: &mut Vec<VarKey>,
) -> bool {
    match pat {
        Term::Blank(label, scope) => {
            let key = (label.clone(), *scope);
            if let Some(prev) = bound.get(&key) {
                return prev == ground;
            }
            bound.insert(key.clone(), ground.clone());
            trail.push(key);
            true
        }
        Term::Triple(inner) => match ground {
            Term::Triple(g) => (0..3).all(|i| try_unify(&inner[i], &g[i], bound, trail)),
            _ => false,
        },
        other => other == ground,
    }
}

/// A closure indexed by predicate IRI — the only position that is always ground.
type Index = BTreeMap<String, Vec<Triple>>;

fn index_by_predicate(triples: Vec<Triple>) -> Index {
    let mut index: Index = BTreeMap::new();
    for triple in triples {
        let key = match &triple[1] {
            Term::Iri(iri) => iri.clone(),
            // A non-IRI predicate cannot occur in RDF; keying it by its debug form
            // simply means nothing will ever match it.
            other => format!("{other:?}"),
        };
        index.entry(key).or_default().push(triple);
    }
    index
}

/// Solve patterns `pats[i..]` against `index`, backtracking over blank-node
/// bindings.
///
/// # Errors
///
/// Returns a message when the candidate budget is exhausted, which the caller
/// turns into a **withhold** rather than a verdict.
fn solve(
    pats: &[Triple],
    index: &Index,
    i: usize,
    bound: &mut BTreeMap<VarKey, Term>,
    budget: &mut u64,
) -> Result<bool, String> {
    let Some(pat) = pats.get(i) else {
        return Ok(true);
    };
    let key = match &pat[1] {
        Term::Iri(iri) => iri.clone(),
        other => format!("{other:?}"),
    };
    let Some(candidates) = index.get(&key) else {
        return Ok(false);
    };
    for candidate in candidates {
        *budget = budget.checked_sub(1).ok_or_else(|| {
            format!("blank-node match exceeded its {MATCH_BUDGET}-candidate budget")
        })?;
        let mut trail = Vec::new();
        let matched = try_unify(&pat[0], &candidate[0], bound, &mut trail)
            && try_unify(&pat[2], &candidate[2], bound, &mut trail);
        if matched && solve(pats, index, i + 1, bound, budget)? {
            return Ok(true);
        }
        for undo in trail {
            bound.remove(&undo);
        }
    }
    Ok(false)
}

/// How many blank-node variables a pattern triple mentions.
fn var_count(term: &Term) -> usize {
    match term {
        Term::Blank(..) => 1,
        Term::Triple(inner) => inner.iter().map(var_count).sum(),
        _ => 0,
    }
}

/// Render a term the way the diagnostic prints it.
fn show(term: &Term) -> String {
    match term {
        Term::Iri(iri) => format!("<{iri}>"),
        Term::Blank(label, scope) => format!("_:{label}#{scope}"),
        Term::Literal {
            lexical, language, ..
        } => language.as_ref().map_or_else(
            || format!("{lexical:?}"),
            |lang| format!("{lexical:?}@{lang}"),
        ),
        Term::Triple(inner) => format!(
            "<<{} {} {}>>",
            show(&inner[0]),
            show(&inner[1]),
            show(&inner[2])
        ),
    }
}

/// Why a target graph did not map into a closure.
///
/// The distinction matters when writing a ledger entry: a target triple with no
/// candidate at all names a conclusion the chase never produced, whereas a target
/// whose triples are individually present but jointly unmappable names a
/// blank-node identity the chase did not establish.
#[derive(Debug)]
pub enum MissReason {
    /// These target triples have no candidate in the closure at all.
    NoCandidate(Vec<String>),
    /// Every target triple has a candidate, but no single blank-node mapping
    /// satisfies them all at once.
    NoConsistentMapping,
}

impl MissReason {
    /// A one-line summary for the harness log and the ledger skeleton.
    #[must_use]
    pub fn summary(&self) -> String {
        match self {
            Self::NoCandidate(triples) => format!("closure lacks {}", triples.join(" ; ")),
            Self::NoConsistentMapping => {
                "every target triple is present but no consistent blank-node mapping exists"
                    .to_owned()
            }
        }
    }
}

/// Whether `closure` simple-entails `target`: does the target graph map into the
/// closure with its blank nodes read as existentials?
///
/// `Ok(None)` means it does; `Ok(Some(reason))` means it does not, and carries the
/// diagnosis a ledger entry needs.
///
/// # Errors
///
/// Returns a message when the search exhausts [`MATCH_BUDGET`].
fn entails(closure: Vec<Triple>, mut target: Vec<Triple>) -> Result<Option<MissReason>, String> {
    // Most-constrained-first: fully ground triples fail (or fix bindings) before
    // the search branches. A stable sort keeps the order reproducible.
    target.sort_by_key(|t| t.iter().map(var_count).sum::<usize>());
    let index = index_by_predicate(closure);
    let mut bound = BTreeMap::new();
    let mut budget = MATCH_BUDGET;
    if solve(&target, &index, 0, &mut bound, &mut budget)? {
        return Ok(None);
    }
    // Diagnose: which target triples have no candidate at all, on their own?
    let mut orphans = Vec::new();
    for pat in &target {
        let mut solo_bound = BTreeMap::new();
        let mut solo_budget = MATCH_BUDGET;
        if !solve(
            std::slice::from_ref(pat),
            &index,
            0,
            &mut solo_bound,
            &mut solo_budget,
        )? {
            orphans.push(format!(
                "{} {} {}",
                show(&pat[0]),
                show(&pat[1]),
                show(&pat[2])
            ));
        }
    }
    Ok(Some(if orphans.is_empty() {
        MissReason::NoConsistentMapping
    } else {
        MissReason::NoCandidate(orphans)
    }))
}

/// Answer one case: materialize the premise under OWL 2 RL and test the target
/// graph against the closure.
#[must_use]
pub fn decide(case: &RlCase) -> Answer {
    let base = format!("http://example.org/w3c-owl2-rl/{}", case.name);
    let premise = match parse(&case.premise, &base) {
        Ok(dataset) => dataset,
        Err(e) => return Answer::Withheld(e),
    };
    let target = match parse(&case.target, &base) {
        Ok(dataset) => owned_triples(&dataset),
        Err(e) => return Answer::Withheld(e),
    };
    let closure = match purrdf_entail::materialize(&premise, purrdf_entail::Regime::OwlRl) {
        Ok((closure, _report)) => owned_triples(&closure),
        Err(e) => return Answer::Withheld(format!("OWL-RL chase: {e}")),
    };
    match entails(closure, target) {
        Ok(None) => Answer::Entailed,
        Ok(Some(reason)) => Answer::NotEntailed(reason),
        Err(e) => Answer::Withheld(e),
    }
}

/// Answer `case` and grade the answer against its published direction.
#[must_use]
pub fn grade(case: &RlCase) -> Grade {
    match decide(case) {
        Answer::Withheld(why) => Grade::Withhold(why),
        Answer::Entailed if case.direction.expects_match() => Grade::Agree,
        Answer::NotEntailed(_) if !case.direction.expects_match() => Grade::Agree,
        Answer::Entailed => Grade::Disagree {
            published: case.direction,
            miss: None,
        },
        Answer::NotEntailed(reason) => Grade::Disagree {
            published: case.direction,
            miss: Some(reason),
        },
    }
}

/// One graded case, kept so the harness can report and cross-check the ledger.
#[derive(Debug)]
pub struct GradedCase {
    /// The case name.
    pub name: String,
    /// The direction the W3C published.
    pub direction: Direction,
    /// How PurRDF's answer compared.
    pub grade: Grade,
    /// Its ledger entry, if it has one.
    pub ledgered: Option<RlGap>,
}

/// The whole corpus run.
#[derive(Debug, Default)]
pub struct RlSummary {
    /// Every case, in case-name order.
    pub cases: Vec<GradedCase>,
}

impl RlSummary {
    /// Cases that agreed with the published verdict and are not ledgered.
    #[must_use]
    pub fn agreed(&self) -> usize {
        self.cases
            .iter()
            .filter(|c| matches!(c.grade, Grade::Agree) && c.ledgered.is_none())
            .count()
    }

    /// Cases that diverged (withheld or disagreed) and are ledgered.
    #[must_use]
    pub fn ledgered(&self) -> usize {
        self.cases
            .iter()
            .filter(|c| !matches!(c.grade, Grade::Agree) && c.ledgered.is_some())
            .count()
    }

    /// Cases that diverged with NO ledger entry. A hard failure.
    #[must_use]
    pub fn unledgered(&self) -> Vec<&GradedCase> {
        self.cases
            .iter()
            .filter(|c| !matches!(c.grade, Grade::Agree) && c.ledgered.is_none())
            .collect()
    }

    /// Ledgered cases that now AGREE — a stale entry. A hard failure, so a closed
    /// gap must be removed from the table rather than left to rot.
    #[must_use]
    pub fn stale(&self) -> Vec<&GradedCase> {
        self.cases
            .iter()
            .filter(|c| matches!(c.grade, Grade::Agree) && c.ledgered.is_some())
            .collect()
    }

    /// How many cases were published in each direction, as `(positive,
    /// negative)`.
    #[must_use]
    pub fn by_direction(&self) -> (usize, usize) {
        let positive = self
            .cases
            .iter()
            .filter(|c| c.direction == Direction::Positive)
            .count();
        (positive, self.cases.len() - positive)
    }

    /// Ledgered divergences that name something PurRDF could fix inside the rule
    /// table, rather than a conclusion no conforming OWL 2 RL rule set reaches.
    ///
    /// Reported separately so the ledger's size is never mistaken for a to-do
    /// list: most of it describes the profile, and this is the part that does not.
    #[must_use]
    pub fn actionable(&self) -> usize {
        self.cases
            .iter()
            .filter(|c| {
                !matches!(c.grade, Grade::Agree) && c.ledgered.is_some_and(RlGap::is_actionable)
            })
            .count()
    }

    /// The single machine-readable line the conformance matrix can scrape.
    #[must_use]
    pub fn scoreboard_line(&self) -> String {
        format!(
            "OWL2-RL-ENTAILMENT: agreed {} ledgered {} unledgered {} stale {} total {} actionable {}",
            self.agreed(),
            self.ledgered(),
            self.unledgered().len(),
            self.stale().len(),
            self.cases.len(),
            self.actionable(),
        )
    }

    /// A per-gap tally of the ledger, in label order, for the run log.
    #[must_use]
    pub fn ledger_tally(&self) -> String {
        let mut counts: Vec<(&'static str, usize, bool)> = Vec::new();
        for case in &self.cases {
            let Some(gap) = case.ledgered else { continue };
            if matches!(case.grade, Grade::Agree) {
                continue;
            }
            if let Some(slot) = counts.iter_mut().find(|(l, _, _)| *l == gap.label()) {
                slot.1 += 1;
            } else {
                counts.push((gap.label(), 1, gap.is_unsound()));
            }
        }
        counts.sort_unstable();
        let mut out = String::new();
        for (label, n, unsound) in counts {
            let mark = if unsound { " [UNSOUND]" } else { "" };
            let _ = write!(out, "\n  {n:>3}  {label}{mark}");
        }
        out
    }

    /// A detailed report of everything that must fail the harness.
    #[must_use]
    pub fn failure_report(&self) -> String {
        let mut lines = Vec::new();
        for case in self.unledgered() {
            let detail = match &case.grade {
                Grade::Agree => unreachable!("agreeing cases are not unledgered"),
                Grade::Withhold(why) => format!("WITHHELD ({why})"),
                Grade::Disagree {
                    published: Direction::Positive,
                    miss,
                } => format!(
                    "W3C published a POSITIVE entailment and the OWL-RL closure does not contain \
                     the conclusion — the RL rule table is INCOMPLETE for a case W3C declared \
                     inside the RL profile ({})",
                    miss.as_ref()
                        .map_or_else(|| "no diagnosis".to_owned(), MissReason::summary)
                ),
                Grade::Disagree {
                    published: Direction::Negative,
                    ..
                } => "W3C published a NEGATIVE entailment and the OWL-RL closure DOES contain \
                      the non-conclusion — the RL rule table is UNSOUND"
                    .to_owned(),
            };
            lines.push(format!(
                "  • UNLEDGERED DIVERGENCE {}: {detail} — add it to LEDGER with a typed RlGap, \
                 or fix the rule table",
                case.name
            ));
        }
        for case in self.stale() {
            lines.push(format!(
                "  • STALE LEDGER ENTRY {}: it now AGREES with the published verdict — remove it \
                 from LEDGER",
                case.name
            ));
        }
        lines.join("\n")
    }
}

/// Grade every case under `root`.
///
/// # Errors
///
/// Returns a message if the corpus cannot be discovered, or if [`LEDGER`] names a
/// case that is not vendored (an entry for a case that no longer exists would
/// silently inflate the budget).
pub fn run(root: &Path) -> Result<RlSummary, String> {
    let cases = discover(root)?;
    for entry in LEDGER {
        if !cases.iter().any(|c| c.name == entry.case) {
            return Err(format!(
                "LEDGER names {:?}, which is not a vendored case under {}",
                entry.case,
                root.display()
            ));
        }
    }
    let mut summary = RlSummary::default();
    for case in &cases {
        summary.cases.push(GradedCase {
            name: case.name.clone(),
            direction: case.direction,
            grade: grade(case),
            ledgered: ledger_lookup(&case.name),
        });
    }
    Ok(summary)
}

/// Render the measured divergences as a paste-ready [`LEDGER`] skeleton.
///
/// Used by the `--ignored` regeneration path after a re-vendor. Every emitted
/// entry gets a `TypeMe` placeholder rather than a guessed [`RlGap`]: the point of
/// the ledger is the typed reason, and a machine cannot supply it.
#[must_use]
pub fn render_ledger_skeleton(summary: &RlSummary) -> String {
    let mut out = String::from(
        "// Paste into LEDGER and replace every `RlGap::TypeMe` with the\n// rule or construct \
         actually responsible.\n",
    );
    for case in &summary.cases {
        let detail = match &case.grade {
            Grade::Agree => continue,
            Grade::Withhold(why) => format!("withheld: {why}"),
            Grade::Disagree { published, miss } => format!(
                "published {} / {}",
                published.label(),
                miss.as_ref().map_or_else(
                    || "OWL-RL closure CONTAINS the target".to_owned(),
                    MissReason::summary
                )
            ),
        };
        let known = case
            .ledgered
            .map_or_else(|| "RlGap::TypeMe".to_owned(), |g| format!("RlGap::{g:?}"));
        let _ = write!(
            out,
            "\n// {detail}\nLedgerEntry {{ case: {:?}, gap: {known} }},",
            case.name
        );
    }
    out
}

// -------------------------------------------------------------------------
// The upstream census
// -------------------------------------------------------------------------

/// One row of `census.tsv`: an upstream W3C test case and its disposition here.
#[derive(Debug, Clone)]
pub struct CensusRow {
    /// The upstream `otest:identifier`.
    pub identifier: String,
    /// The slugified case directory name.
    pub case: String,
    /// The `otest:` test types the case declares, `;`-joined.
    pub otest_types: String,
    /// The `otest:semantics` values, `;`-joined.
    pub semantics: String,
    /// The `otest:profile` values, `;`-joined (`-` when none).
    pub profiles: String,
    /// The `otest:status`.
    pub status: String,
    /// The `otest:normativeSyntax` values, `;`-joined.
    pub normative_syntax: String,
    /// Whether the case carries an `otest:rdfXmlPremiseOntology`.
    pub premise: String,
    /// `conclusion`, `non-conclusion`, or `none`.
    pub conclusion: String,
    /// This case's disposition in the OWL 2 RL entailment corpus.
    pub rl_corpus: String,
    /// This case's disposition in the OWL 2 DL consistency corpus.
    pub dl_corpus: String,
    /// For a consistency-shaped case the DL corpus does **not** vendor: what
    /// PurRDF's `OWL-Direct` tableau actually did with it when probed. See
    /// `PROVENANCE.md` for the measurement's date and per-case budget.
    ///
    /// This is the column that makes the DL corpus's exclusions auditable: a
    /// reader can count, by name, how many upstream cases were left out and how
    /// many of those the reasoner genuinely cannot decide.
    pub dl_probe: String,
}

/// The census's column header, asserted so a re-vendor cannot silently reshape it.
pub const CENSUS_HEADER: &str = "identifier\tcase\totest_types\tsemantics\tprofiles\tstatus\t\
     normative_syntax\tpremise\tconclusion\trl_corpus\tdl_corpus\tdl_probe";

/// Read `census.tsv` from a vendored root.
///
/// # Errors
///
/// Returns a message if the file cannot be read, if its header is not
/// [`CENSUS_HEADER`], or if any row does not have exactly twelve columns.
pub fn read_census(root: &Path) -> Result<Vec<CensusRow>, String> {
    let path = root.join("census.tsv");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let mut lines = text.lines();
    let header = lines.next().unwrap_or_default();
    if header != CENSUS_HEADER {
        return Err(format!(
            "{}: header is {header:?}, expected {CENSUS_HEADER:?}",
            path.display()
        ));
    }
    let mut rows = Vec::new();
    for (n, line) in lines.enumerate() {
        let f: Vec<&str> = line.split('\t').collect();
        let [
            identifier,
            case,
            otest_types,
            semantics,
            profiles,
            status,
            normative_syntax,
            premise,
            conclusion,
            rl_corpus,
            dl_corpus,
            dl_probe,
        ] = f[..]
        else {
            return Err(format!(
                "{}:{}: {} columns, expected 12",
                path.display(),
                n + 2,
                f.len()
            ));
        };
        rows.push(CensusRow {
            identifier: identifier.to_owned(),
            case: case.to_owned(),
            otest_types: otest_types.to_owned(),
            semantics: semantics.to_owned(),
            profiles: profiles.to_owned(),
            status: status.to_owned(),
            normative_syntax: normative_syntax.to_owned(),
            premise: premise.to_owned(),
            conclusion: conclusion.to_owned(),
            rl_corpus: rl_corpus.to_owned(),
            dl_corpus: dl_corpus.to_owned(),
            dl_probe: dl_probe.to_owned(),
        });
    }
    Ok(rows)
}

/// Tally a census column into `(value, count)` pairs in value order.
#[must_use]
pub fn census_tally(rows: &[CensusRow], column: fn(&CensusRow) -> &str) -> Vec<(String, usize)> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for row in rows {
        *counts.entry(column(row)).or_default() += 1;
    }
    counts.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iri(s: &str) -> Term {
        Term::Iri(s.to_owned())
    }

    fn blank(s: &str) -> Term {
        Term::Blank(s.to_owned(), 0)
    }

    #[test]
    fn ground_target_must_be_present() {
        let closure = vec![[iri("s"), iri("p"), iri("o")]];
        assert!(
            entails(closure.clone(), vec![[iri("s"), iri("p"), iri("o")]])
                .unwrap()
                .is_none()
        );
        assert!(
            entails(closure, vec![[iri("s"), iri("p"), iri("x")]])
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn blank_nodes_are_existentials() {
        let closure = vec![[iri("s"), iri("p"), iri("o")]];
        assert!(
            entails(closure.clone(), vec![[blank("b"), iri("p"), iri("o")]])
                .unwrap()
                .is_none()
        );
        assert!(
            entails(closure, vec![[blank("b"), iri("p"), blank("c")]])
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn a_repeated_blank_must_map_consistently() {
        // `_:b p o1 . _:b p o2` needs ONE node with both edges; the closure gives
        // two different nodes with one edge each, so it must NOT match.
        let closure = vec![
            [iri("s1"), iri("p"), iri("o1")],
            [iri("s2"), iri("p"), iri("o2")],
        ];
        let target = vec![
            [blank("b"), iri("p"), iri("o1")],
            [blank("b"), iri("p"), iri("o2")],
        ];
        assert!(entails(closure.clone(), target).unwrap().is_some());
        let mut wider = closure;
        wider.push([iri("s1"), iri("p"), iri("o2")]);
        let target = vec![
            [blank("b"), iri("p"), iri("o1")],
            [blank("b"), iri("p"), iri("o2")],
        ];
        assert!(entails(wider, target).unwrap().is_none());
    }

    #[test]
    fn matching_backtracks_over_a_bad_first_choice() {
        // The first candidate for `_:b p ?` is `s1`, which cannot satisfy the
        // second pattern; the search must undo it and try `s2`.
        let closure = vec![
            [iri("s1"), iri("p"), iri("o1")],
            [iri("s2"), iri("p"), iri("o1")],
            [iri("s2"), iri("q"), iri("o2")],
        ];
        let target = vec![
            [blank("b"), iri("p"), iri("o1")],
            [blank("b"), iri("q"), iri("o2")],
        ];
        assert!(entails(closure, target).unwrap().is_none());
    }

    #[test]
    fn literals_compare_on_datatype_too() {
        let lit = |lex: &str, dt: &str| Term::Literal {
            lexical: lex.to_owned(),
            datatype: dt.to_owned(),
            language: None,
            direction: None,
        };
        let closure = vec![[iri("s"), iri("p"), lit("1", "xsd:integer")]];
        assert!(
            entails(
                closure.clone(),
                vec![[iri("s"), iri("p"), lit("1", "xsd:integer")]]
            )
            .unwrap()
            .is_none()
        );
        assert!(
            entails(closure, vec![[iri("s"), iri("p"), lit("1", "xsd:string")]])
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn an_absent_predicate_short_circuits() {
        let closure = vec![[iri("s"), iri("p"), iri("o")]];
        assert!(
            entails(closure, vec![[blank("b"), iri("absent"), blank("c")]])
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn ledger_has_no_duplicate_cases() {
        let mut names: Vec<&str> = LEDGER.iter().map(|e| e.case).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(count, names.len(), "LEDGER holds a duplicate case entry");
    }

    #[test]
    fn no_ledgered_gap_is_an_unsoundness() {
        let unsound: Vec<&str> = LEDGER
            .iter()
            .filter(|e| e.gap.is_unsound())
            .map(|e| e.case)
            .collect();
        assert!(
            unsound.is_empty(),
            "the set of unsound divergences must be EMPTY — an unsoundness is the OWL-RL chase \
             deriving a triple the W3C says is NOT entailed, which no incompleteness can excuse. \
             These claim otherwise and must be reviewed by hand: {unsound:?}"
        );
    }

    #[test]
    fn only_missing_rules_are_actionable() {
        // The ledger's size is not a to-do list. Exactly the entries that name a
        // sound, in-shape rule the table omits (or an unsoundness) are actionable;
        // everything else describes what the OWL 2 RL profile itself cannot reach.
        let actionable: Vec<&str> = LEDGER
            .iter()
            .filter(|e| e.gap.is_actionable())
            .map(|e| e.case)
            .collect();
        assert_eq!(
            actionable,
            ["webont-differentfrom-001"],
            "the actionable set changed; a new entry means the rule table owes a conclusion it \
             does not produce, and a lost entry means one was fixed — either way, say so in the \
             ledger comment and here"
        );
    }

    #[test]
    fn direction_and_target_file_agree() {
        assert_eq!(Direction::Positive.target_file(), "conclusion.rdf");
        assert_eq!(Direction::Negative.target_file(), "non-conclusion.rdf");
        assert!(Direction::Positive.expects_match());
        assert!(!Direction::Negative.expects_match());
    }
}
