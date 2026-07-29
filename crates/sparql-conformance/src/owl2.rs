// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The vendored W3C OWL 2 corpus: ingester, grader, and divergence ledger.
//!
//! # What this grades, and what it does not
//!
//! Every case in `entailment-suite/w3c-owl2/` is a **satisfiability** case — the
//! W3C published either `otest:ConsistencyTest` or `otest:InconsistencyTest` for
//! it, and nothing else. The grader loads the case's verbatim RDF/XML premise
//! ontology and asks PurRDF's open-world `OWL-Direct` ALCOIQ tableau (through
//! [`purrdf_entail::materialize_dl`]) whether it is consistent, then compares that
//! answer with the published one.
//!
//! So this corpus validates **the DL / tableau lane's verdicts**. It does **not**
//! validate the OWL 2 RL rule table: that lane is a forward-materialization chase
//! over a declared rule program and is covered by authored per-rule fixtures in
//! `purrdf-entail`, not here. The `Entailment` row of the conformance matrix that
//! this module feeds must be read as "open-world DL consistency", never as "OWL 2
//! RL rule coverage".
//!
//! There is likewise not one `otest:PositiveEntailmentTest` or
//! `otest:NegativeEntailmentTest` in the tree — the upstream W3C material this
//! corpus was flattened from contains none. See the tree's `PROVENANCE.md`.
//!
//! # Three outcomes, never two
//!
//! A reasoner has three honest answers, so the grader has three buckets
//! ([`Grade`]):
//!
//! * **agree** — PurRDF decided the case and matched the published verdict;
//! * **withhold** — PurRDF *refused to decide*: an [`EntailError`] other than
//!   [`EntailError::Inconsistent`] (an unparsable class-expression graph, a
//!   tableau step-cap trip). A refusal is a capability gap, not a wrong answer,
//!   and is never silently scored as a pass;
//!   [`EntailError`]: purrdf_entail::EntailError
//! * **disagree** — PurRDF decided the case and got the other answer.
//!
//! Every withhold and every disagreement must appear in [`LEDGER`] with a typed
//! [`Owl2Gap`]. An unledgered one fails the harness; a ledgered one that starts
//! agreeing also fails it, so the ledger cannot rot.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use purrdf_entail::EntailError;

/// The verdict the W3C published for a case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The premise ontology is satisfiable (`otest:ConsistencyTest`).
    Consistent,
    /// The premise ontology is unsatisfiable (`otest:InconsistencyTest`).
    Inconsistent,
}

impl Verdict {
    /// The token used in `profile.json` and in the harness log.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Consistent => "consistent",
            Self::Inconsistent => "inconsistent",
        }
    }
}

/// Why PurRDF diverges from the published verdict on a ledgered case.
///
/// Each variant names the concrete OWL 2 construct or reasoner behaviour
/// responsible, so the ledger doubles as a precise inventory of what the tableau
/// lane does not yet model. A catch-all would defeat the point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owl2Gap {
    /// `owl:AsymmetricProperty` is not in the tableau's vocabulary, so the role
    /// axiom it states cannot clash.
    AsymmetricProperty,
    /// `owl:IrreflexiveProperty` is not in the tableau's vocabulary, so the role
    /// axiom it states cannot clash.
    IrreflexiveProperty,
    /// Property disjointness — `owl:propertyDisjointWith` and
    /// `owl:AllDisjointProperties` — is not modelled.
    PropertyDisjointness,
    /// `owl:NegativePropertyAssertion` (the `owl:sourceIndividual` /
    /// `owl:assertionProperty` / `owl:targetIndividual`-or-`owl:targetValue`
    /// reification) is not modelled, so a negative assertion cannot contradict
    /// its positive counterpart.
    NegativePropertyAssertion,
    /// `owl:hasKey` is not modelled, so a key violation cannot be detected.
    HasKey,
    /// `owl:bottomObjectProperty` / `owl:bottomDataProperty` are read as ordinary
    /// named roles rather than as the empty role, so an assertion over one cannot
    /// clash.
    BottomProperty,
    /// Individual difference — `owl:differentFrom`, `owl:AllDifferent` (both the
    /// `owl:members` and the legacy `owl:distinctMembers` spellings) — is not
    /// modelled. Without it, no cardinality restriction can be violated by
    /// distinct fillers.
    DifferentIndividuals,
    /// `owl:AllDisjointClasses` (the n-ary spelling; the binary
    /// `owl:disjointWith` *is* modelled) is not read.
    AllDisjointClasses,
    /// OWL 2 requires every interpretation domain to be non-empty, so
    /// `owl:Thing owl:equivalentClass owl:Nothing` is unsatisfiable. The tableau
    /// admits the empty model instead and reports it satisfiable.
    EmptyDomain,
    /// `owl:hasSelf` is not a recognized `owl:Restriction` constraint, so the
    /// class-expression parser refuses the graph and the run withholds.
    SelfRestriction,
    /// The realization pass would assert a type triple whose subject is a
    /// literal, which the RDF IR rejects on freeze, so the run withholds. A
    /// datatype-property premise reaches this.
    LiteralSubjectInjection,
    /// The class-expression graph is cyclic, which the parser refuses rather than
    /// unfolding, so the run withholds.
    CyclicClassExpression,
}

impl Owl2Gap {
    /// A short human-readable label for the ledger tally and the log.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::AsymmetricProperty => "asymmetric-property",
            Self::IrreflexiveProperty => "irreflexive-property",
            Self::PropertyDisjointness => "property-disjointness",
            Self::NegativePropertyAssertion => "negative-property-assertion",
            Self::HasKey => "has-key",
            Self::BottomProperty => "bottom-property",
            Self::DifferentIndividuals => "different-individuals",
            Self::AllDisjointClasses => "all-disjoint-classes",
            Self::EmptyDomain => "empty-domain",
            Self::SelfRestriction => "self-restriction",
            Self::LiteralSubjectInjection => "literal-subject-injection",
            Self::CyclicClassExpression => "cyclic-class-expression",
        }
    }

    /// Whether this gap is an **unsoundness** — PurRDF committing to a verdict the
    /// W3C contradicts in the direction that matters (claiming `inconsistent`
    /// where the ontology is satisfiable).
    ///
    /// **No variant is one today.** Every gap left in this enum is an
    /// incompleteness: an axiom that goes unread, so a real clash is missed, or a
    /// graph the run refuses outright. The predicate stays — pinned by a test
    /// asserting the unsound set is empty, and surfaced as `[UNSOUND]` in the
    /// harness log — because that is the ledger's most consequential distinction:
    /// an incompleteness withholds an answer PurRDF is entitled to, whereas an
    /// unsoundness asserts one it is not. The match below is exhaustive on
    /// purpose, so a new gap cannot be added without classifying itself here.
    #[must_use]
    pub const fn is_unsound(self) -> bool {
        match self {
            Self::AsymmetricProperty
            | Self::IrreflexiveProperty
            | Self::PropertyDisjointness
            | Self::NegativePropertyAssertion
            | Self::HasKey
            | Self::BottomProperty
            | Self::DifferentIndividuals
            | Self::AllDisjointClasses
            | Self::EmptyDomain
            | Self::SelfRestriction
            | Self::LiteralSubjectInjection
            | Self::CyclicClassExpression => false,
        }
    }
}

/// One ledgered divergence: the case's directory name plus its typed gap.
#[derive(Debug)]
pub struct LedgerEntry {
    /// The case directory name under `entailment-suite/w3c-owl2/cases/`.
    pub case: &'static str,
    /// Why PurRDF diverges.
    pub gap: Owl2Gap,
}

/// The divergence ledger: every vendored case PurRDF does not agree with today.
///
/// Nothing is skipped at discovery time — all 261 cases run, and a case absent
/// from this table must agree. Entries are grouped by root cause; each group's
/// comment states the construct the tableau does not read and what that costs.
pub const LEDGER: &[LedgerEntry] = &[
    // --- `owl:AsymmetricProperty` / `owl:IrreflexiveProperty` ----------------
    //     Both are OWL 2 role axioms whose whole content is a negative
    //     constraint (`¬∃r.Self`, and `r ⊓ r⁻ ⊑ ⊥`). Neither IRI is in the
    //     tableau's vocabulary table, so the premise's asserted role edge meets
    //     no constraint and the ontology reads as satisfiable.
    LedgerEntry {
        case: "new-feature-asymmetricproperty-001",
        gap: Owl2Gap::AsymmetricProperty,
    },
    LedgerEntry {
        case: "rdfbased-sem-char-asymmetric-inst",
        gap: Owl2Gap::AsymmetricProperty,
    },
    LedgerEntry {
        case: "rdfbased-sem-char-asymmetric-term",
        gap: Owl2Gap::AsymmetricProperty,
    },
    LedgerEntry {
        case: "new-feature-irreflexiveproperty-001",
        gap: Owl2Gap::IrreflexiveProperty,
    },
    LedgerEntry {
        case: "rdfbased-sem-char-irreflexive-inst",
        gap: Owl2Gap::IrreflexiveProperty,
    },
    // --- Property disjointness ----------------------------------------------
    //     `owl:propertyDisjointWith` and its n-ary `owl:AllDisjointProperties`
    //     spelling are unread, so two asserted edges over disjoint roles never
    //     clash.
    LedgerEntry {
        case: "new-feature-disjointdataproperties-001",
        gap: Owl2Gap::PropertyDisjointness,
    },
    LedgerEntry {
        case: "rdfbased-sem-eqdis-disprop-eqprop",
        gap: Owl2Gap::PropertyDisjointness,
    },
    LedgerEntry {
        case: "rdfbased-sem-eqdis-disprop-inst",
        gap: Owl2Gap::PropertyDisjointness,
    },
    LedgerEntry {
        case: "rdfbased-sem-eqdis-disprop-irrflxv",
        gap: Owl2Gap::PropertyDisjointness,
    },
    LedgerEntry {
        case: "rdfbased-sem-ndis-alldisjointproperties-fw",
        gap: Owl2Gap::PropertyDisjointness,
    },
    // --- `owl:NegativePropertyAssertion` -------------------------------------
    //     The reified negative assertion is read as four ordinary triples about
    //     a blank node, so it never contradicts the positive assertion the case
    //     pairs it with.
    LedgerEntry {
        case: "new-feature-negativedatapropertyassertion-001",
        gap: Owl2Gap::NegativePropertyAssertion,
    },
    LedgerEntry {
        case: "new-feature-negativeobjectpropertyassertion-001",
        gap: Owl2Gap::NegativePropertyAssertion,
    },
    LedgerEntry {
        case: "rdfbased-sem-npa-dat-fw",
        gap: Owl2Gap::NegativePropertyAssertion,
    },
    LedgerEntry {
        case: "rdfbased-sem-npa-ind-fw",
        gap: Owl2Gap::NegativePropertyAssertion,
    },
    // --- `owl:hasKey` --------------------------------------------------------
    LedgerEntry {
        case: "new-feature-keys-002",
        gap: Owl2Gap::HasKey,
    },
    LedgerEntry {
        case: "new-feature-keys-006",
        gap: Owl2Gap::HasKey,
    },
    // --- `owl:bottomObjectProperty` / `owl:bottomDataProperty` ---------------
    //     Read as ordinary named roles, so an assertion over one is admitted
    //     instead of clashing against the empty role.
    LedgerEntry {
        case: "new-feature-bottomdataproperty-001",
        gap: Owl2Gap::BottomProperty,
    },
    LedgerEntry {
        case: "new-feature-bottomobjectproperty-001",
        gap: Owl2Gap::BottomProperty,
    },
    // --- Individual difference ------------------------------------------------
    //     `owl:differentFrom` and `owl:AllDifferent` are unread. The tableau's
    //     `≤n r.C` clash rule needs pairwise-distinct fillers to fire, so the
    //     max-cardinality case below is a casualty of the same gap rather than a
    //     cardinality bug.
    LedgerEntry {
        case: "rdfbased-sem-eqdis-different-irrflxv",
        gap: Owl2Gap::DifferentIndividuals,
    },
    LedgerEntry {
        case: "rdfbased-sem-eqdis-different-sameas",
        gap: Owl2Gap::DifferentIndividuals,
    },
    LedgerEntry {
        case: "rdfbased-sem-ndis-alldifferent-fw",
        gap: Owl2Gap::DifferentIndividuals,
    },
    LedgerEntry {
        case: "rdfbased-sem-ndis-alldifferent-fw-distinctmembers",
        gap: Owl2Gap::DifferentIndividuals,
    },
    LedgerEntry {
        case: "webont-maxcardinality-001",
        gap: Owl2Gap::DifferentIndividuals,
    },
    // --- `owl:AllDisjointClasses` --------------------------------------------
    //     The binary `owl:disjointWith` is modelled; the n-ary
    //     `owl:AllDisjointClasses` / `owl:members` spelling is not.
    LedgerEntry {
        case: "rdfbased-sem-ndis-alldisjointclasses-fw",
        gap: Owl2Gap::AllDisjointClasses,
    },
    // --- Non-empty interpretation domain --------------------------------------
    //     `owl:Thing owl:equivalentClass owl:Nothing` is unsatisfiable ONLY
    //     because OWL 2 forbids the empty domain. The tableau finds the empty
    //     model and stops.
    LedgerEntry {
        case: "webont-thing-003",
        gap: Owl2Gap::EmptyDomain,
    },
    // --- Withheld: the run refused to decide ---------------------------------
    //     Three cases where PurRDF returns an `EntailError` rather than a
    //     verdict. A refusal is an honest capability gap and is bucketed apart
    //     from a wrong answer, but it is still ledgered — never scored as a pass.
    LedgerEntry {
        case: "new-feature-selfrestriction-001",
        gap: Owl2Gap::SelfRestriction,
    },
    LedgerEntry {
        case: "webont-datatypeproperty-001",
        gap: Owl2Gap::LiteralSubjectInjection,
    },
    LedgerEntry {
        case: "webont-i5-26-007",
        gap: Owl2Gap::CyclicClassExpression,
    },
    // --- Unsound: PurRDF commits to the wrong verdict -------------------------
    //     Empty, and it must stay empty. `webont-oneof-003` — which types a
    //     fourth individual `myT` into an `owl:oneOf` enumeration of three
    //     others — used to sit here: the tableau clashed because `myT` was a
    //     named individual outside the enumeration, an implicit unique-name
    //     assumption OWL 2 does not make. The `o`-rule now *identifies* `myT`
    //     with a member instead of clashing, and clashes only when every such
    //     identification is blocked by a recorded `≠`, so the case agrees. Every
    //     entry above is an incompleteness (a missed clash) or a refusal to
    //     decide; none is an invented clash.
];

/// Look a case up in [`LEDGER`].
#[must_use]
pub fn ledger_lookup(case: &str) -> Option<Owl2Gap> {
    LEDGER.iter().find(|e| e.case == case).map(|e| e.gap)
}

/// One vendored case.
#[derive(Debug)]
pub struct Owl2Case {
    /// The case directory name, which is also the suite's `otest:identifier`.
    pub name: String,
    /// The verbatim W3C RDF/XML premise ontology.
    pub premise: PathBuf,
    /// The verdict the W3C published for it.
    pub published: Verdict,
}

/// What PurRDF answered for a case.
#[derive(Debug)]
pub enum Answer {
    /// The tableau decided the case.
    Decided(Verdict),
    /// The run refused to decide, carrying the refusal's own message.
    Withheld(String),
}

/// How PurRDF's answer compares to the published verdict.
#[derive(Debug)]
pub enum Grade {
    /// Decided, and matching.
    Agree,
    /// Refused to decide, with the refusal's message.
    Withhold(String),
    /// Decided, and not matching: PurRDF said `got` where the W3C published
    /// `published`.
    Disagree {
        /// The W3C's verdict.
        published: Verdict,
        /// PurRDF's verdict.
        got: Verdict,
    },
}

/// The vendored corpus root (`entailment-suite/w3c-owl2/cases`).
#[must_use]
pub fn suite_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("entailment-suite/w3c-owl2/cases")
}

/// Read `w3c_published_verdict` out of a vendored `profile.json`.
///
/// The vendored profiles are a fixed, machine-generated four-key shape, so this
/// is a deliberate hand-rolled scan rather than a JSON dependency: the harness
/// crate ships no JSON reader, and adding one to read a 131-byte file with a
/// known shape would be the larger change. It is strict — an absent key, an
/// unquoted value, or any token other than the two the suite emits is a hard
/// error, never a default.
fn parse_published_verdict(text: &str, path: &Path) -> Result<Verdict, String> {
    const KEY: &str = "\"w3c_published_verdict\"";
    let Some(rest) = text.split_once(KEY).map(|(_, rest)| rest) else {
        return Err(format!("{}: no {KEY} key", path.display()));
    };
    let rest = rest.trim_start();
    let Some(rest) = rest.strip_prefix(':') else {
        return Err(format!("{}: {KEY} is not followed by ':'", path.display()));
    };
    let rest = rest.trim_start();
    let Some(rest) = rest.strip_prefix('"') else {
        return Err(format!(
            "{}: {KEY}'s value is not a quoted string",
            path.display()
        ));
    };
    let Some((value, _)) = rest.split_once('"') else {
        return Err(format!("{}: unterminated {KEY} value", path.display()));
    };
    match value {
        "consistent" => Ok(Verdict::Consistent),
        "inconsistent" => Ok(Verdict::Inconsistent),
        other => Err(format!(
            "{}: unknown published verdict {other:?} (expected \"consistent\" or \"inconsistent\")",
            path.display()
        )),
    }
}

/// Discover every vendored case under `root`, in case-name order.
///
/// # Errors
///
/// Returns a message if `root` cannot be read, if a case directory is missing
/// either of its two payload files, or if a `profile.json` does not declare a
/// verdict this harness recognizes. A malformed corpus is a hard error: silently
/// dropping a case would shrink the ledger without anyone noticing.
pub fn discover(root: &Path) -> Result<Vec<Owl2Case>, String> {
    let mut names: Vec<String> = Vec::new();
    let entries =
        std::fs::read_dir(root).map_err(|e| format!("cannot read {}: {e}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot read {}: {e}", root.display()))?;
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
        let dir = root.join(&name);
        let profile = dir.join("profile.json");
        let premise = dir.join("source/premise.rdf");
        if !premise.is_file() {
            return Err(format!("{}: missing premise", premise.display()));
        }
        let text = std::fs::read_to_string(&profile)
            .map_err(|e| format!("cannot read {}: {e}", profile.display()))?;
        let published = parse_published_verdict(&text, &profile)?;
        cases.push(Owl2Case {
            name,
            premise,
            published,
        });
    }
    Ok(cases)
}

/// Decide one case with PurRDF's `OWL-Direct` ALCOIQ tableau.
///
/// The premise is parsed by the first-party RDF/XML codec into the default graph
/// (which is the only graph the DL knowledge-base builder reads) under a
/// synthetic `example.org` base IRI. No vendored premise contains a relative
/// RDF/XML reference without also declaring its own `xml:base`, so that base is
/// never consulted and no verdict depends on it — see the tree's `PROVENANCE.md`.
///
/// [`purrdf_entail::materialize_dl`] with an empty query pattern is the public
/// consistency seam: it runs the tableau over the whole knowledge base and
/// short-circuits to [`EntailError::Inconsistent`] before doing any
/// query-directed work, so the classification and realization passes it would
/// otherwise perform only ever run on a satisfiable knowledge base.
#[must_use]
pub fn decide(case: &Owl2Case) -> Answer {
    let bytes = match std::fs::read(&case.premise) {
        Ok(bytes) => bytes,
        Err(e) => return Answer::Withheld(format!("cannot read premise: {e}")),
    };
    let base = format!("http://example.org/w3c-owl2/{}", case.name);
    let dataset = match purrdf::parse_dataset(&bytes, "application/rdf+xml", Some(&base)) {
        Ok(dataset) => dataset,
        Err(e) => return Answer::Withheld(format!("RDF/XML parse: {e}")),
    };
    match purrdf_entail::materialize_dl(&dataset, &[]) {
        Ok(_) => Answer::Decided(Verdict::Consistent),
        Err(EntailError::Inconsistent) => Answer::Decided(Verdict::Inconsistent),
        Err(e) => Answer::Withheld(e.to_string()),
    }
}

/// Decide `case` and grade the answer against its published verdict.
#[must_use]
pub fn grade(case: &Owl2Case) -> Grade {
    match decide(case) {
        Answer::Withheld(why) => Grade::Withhold(why),
        Answer::Decided(got) if got == case.published => Grade::Agree,
        Answer::Decided(got) => Grade::Disagree {
            published: case.published,
            got,
        },
    }
}

/// One graded case, kept so the harness can report and cross-check the ledger.
#[derive(Debug)]
pub struct GradedCase {
    /// The case name.
    pub name: String,
    /// The W3C's verdict.
    pub published: Verdict,
    /// How PurRDF's answer compared.
    pub grade: Grade,
    /// Its ledger entry, if it has one.
    pub ledgered: Option<Owl2Gap>,
}

/// The whole corpus run.
#[derive(Debug, Default)]
pub struct Owl2Summary {
    /// Every case, in case-name order.
    pub cases: Vec<GradedCase>,
}

impl Owl2Summary {
    /// Cases that agreed with the published verdict and are not ledgered.
    #[must_use]
    pub fn agreed(&self) -> usize {
        self.cases
            .iter()
            .filter(|c| matches!(c.grade, Grade::Agree) && c.ledgered.is_none())
            .count()
    }

    /// Cases that diverged (withheld or disagreed) and are ledgered — the
    /// "XFail/Skip" column of the conformance matrix.
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

    /// Ledgered cases that now AGREE — a stale ledger entry. A hard failure, so
    /// a closed gap must be removed from the table rather than left to rot.
    #[must_use]
    pub fn stale(&self) -> Vec<&GradedCase> {
        self.cases
            .iter()
            .filter(|c| matches!(c.grade, Grade::Agree) && c.ledgered.is_some())
            .collect()
    }

    /// How many cases carried each published verdict, as `(consistent,
    /// inconsistent)`.
    #[must_use]
    pub fn by_published(&self) -> (usize, usize) {
        let consistent = self
            .cases
            .iter()
            .filter(|c| c.published == Verdict::Consistent)
            .count();
        (consistent, self.cases.len() - consistent)
    }

    /// The single machine-readable line the conformance matrix scrapes.
    #[must_use]
    pub fn scoreboard_line(&self) -> String {
        format!(
            "OWL2-ENTAILMENT: agreed {} ledgered {} unledgered {} stale {} total {}",
            self.agreed(),
            self.ledgered(),
            self.unledgered().len(),
            self.stale().len(),
            self.cases.len(),
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
                Grade::Disagree { published, got } => {
                    format!(
                        "published {} but PurRDF said {}",
                        published.label(),
                        got.label()
                    )
                }
            };
            lines.push(format!(
                "  • UNLEDGERED DIVERGENCE {}: {detail} — add it to LEDGER with a typed Owl2Gap, \
                 or fix the reasoner",
                case.name
            ));
        }
        for case in self.stale() {
            lines.push(format!(
                "  • STALE LEDGER ENTRY {}: it now AGREES with the published verdict — remove it \
                 from LEDGER and lower the budget in scripts/conformance-baseline.json",
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
/// case that is not in the corpus (a ledger entry for a case that no longer
/// exists would silently inflate the budget).
pub fn run(root: &Path) -> Result<Owl2Summary, String> {
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
    let mut summary = Owl2Summary::default();
    for case in &cases {
        summary.cases.push(GradedCase {
            name: case.name.clone(),
            published: case.published,
            grade: grade(case),
            ledgered: ledger_lookup(&case.name),
        });
    }
    Ok(summary)
}

/// Render the measured divergences as a paste-ready [`LEDGER`] skeleton.
///
/// Used by the `--ignored` regeneration path after a re-vendor. Every emitted
/// entry gets a `TypeMe` placeholder rather than a guessed [`Owl2Gap`]: the point
/// of the ledger is the typed reason, and a machine cannot supply it.
#[must_use]
pub fn render_ledger_skeleton(summary: &Owl2Summary) -> String {
    let mut out = String::from(
        "// Paste into LEDGER and replace every `Owl2Gap::TypeMe` with the\n// construct actually responsible.\n",
    );
    for case in &summary.cases {
        let detail = match &case.grade {
            Grade::Agree => continue,
            Grade::Withhold(why) => format!("withheld: {why}"),
            Grade::Disagree { published, got } => {
                format!("published {} / PurRDF {}", published.label(), got.label())
            }
        };
        let known = case.ledgered.map_or_else(
            || "Owl2Gap::TypeMe".to_owned(),
            |g| format!("Owl2Gap::{g:?}"),
        );
        let _ = write!(
            out,
            "\n// {detail}\nLedgerEntry {{ case: {:?}, gap: {known} }},",
            case.name
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_verdict_parses_both_tokens() {
        let path = Path::new("profile.json");
        let consistent = r#"{"mode":"native","w3c_published_verdict":"consistent"}"#;
        let inconsistent = r#"{"w3c_published_verdict": "inconsistent", "mode": "native"}"#;
        assert_eq!(
            parse_published_verdict(consistent, path).unwrap(),
            Verdict::Consistent
        );
        assert_eq!(
            parse_published_verdict(inconsistent, path).unwrap(),
            Verdict::Inconsistent
        );
    }

    #[test]
    fn published_verdict_never_defaults() {
        let path = Path::new("profile.json");
        for bad in [
            r#"{"mode":"native"}"#,
            r#"{"w3c_published_verdict":"maybe"}"#,
            r#"{"w3c_published_verdict" = "consistent"}"#,
            r#"{"w3c_published_verdict": consistent}"#,
            r#"{"w3c_published_verdict": "consistent"#,
        ] {
            assert!(
                parse_published_verdict(bad, path).is_err(),
                "must reject {bad:?} rather than default"
            );
        }
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
            "the set of unsound divergences must be EMPTY — every ledgered gap is an \
             incompleteness (an unread axiom, so a real clash is missed) or a refusal to \
             decide, never a verdict PurRDF is not entitled to. These claim otherwise and \
             must be reviewed by hand: {unsound:?}"
        );
    }
}
