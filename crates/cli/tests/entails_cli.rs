// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end `entails` coverage that drives the BUILT `purrdf` binary
//! (`env!("CARGO_BIN_EXE_purrdf")`) — never the library — so every assertion pins the
//! shipped executable's conclusion-directed entailment behavior, exactly as
//! `reason_cli.rs` pins its materialization behavior.
//!
//! ## What is asserted, and why each case is here
//!
//! The question this subcommand answers is not "is the conclusion in the closure": it is
//! reached SIX ways, five of which exist because the regime's rule table DECIDES no
//! conclusion of that shape. A test suite that only exercised the rule-table lane would
//! certify a binary that had none of the rest, so every mechanism the CLI can surface has a
//! case, and each names the mechanism rather than only the verdict:
//!
//! * **`strict-table`** — the rule table derives it, and (separately) the rule table
//!   REFUTES one: `not-entailed` is a proof, and `strict-table` is the only mechanism that
//!   can accompany one, because refuting needs the completeness half of a theorem.
//! * **`refutation`** — a negative fact (`owl:differentFrom`), which no rule concludes;
//!   the seventeen `false`-headed rules decide it instead.
//! * **`freeze`** — a schema axiom (`owl:TransitiveProperty` from a property chain), which
//!   no rule concludes either.
//! * **`composite`** — a conclusion GRAPH is a conjunction, so it can need a lane per half;
//!   the answer names `composite` and lists its constituents.
//! * **`--verify`** — the warrant re-decided without running a reasoner, and its
//!   `not-applicable` twin where there is no warrant to re-decide.
//! * **`--pattern`** — the certain answers of a basic graph pattern, the third service on
//!   the same boundary.
//! * **`--import`** — a premise whose `owl:imports` is intact, answered from the documents
//!   the operator supplied, and refused BY NAME when they are not supplied.
//!
//! ## The refusals
//!
//! `owl-direct` and `rif` are each defined by an input "premise, conclusion, regime" does
//! not carry, so the boundary refuses them naming the regime; a malformed `--import` pair
//! is a usage error rather than a skipped import; and two documents reading stdin is
//! refused rather than mis-read, because a process has one standard input.

use std::path::Path;
use std::process::{Command, Output, Stdio};

/// A `Command` for the built `purrdf` binary.
fn purrdf() -> Command {
    Command::new(env!("CARGO_BIN_EXE_purrdf"))
}

/// Run `purrdf` with `args`, returning the captured [`Output`].
fn run(args: &[&str]) -> Output {
    purrdf()
        .args(args)
        .output()
        .expect("spawn the built purrdf binary")
}

/// stdout of an [`Output`] as a `String`.
fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// stderr of an [`Output`] as a `String`.
fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Join a name onto `dir`, returning it as an owned `String` (the shape [`run`] wants).
fn path(dir: &Path, name: &str) -> String {
    dir.join(name)
        .to_str()
        .expect("temp path is valid UTF-8")
        .to_owned()
}

/// Write `contents` to `dir/name`, returning the path.
fn write_file(dir: &Path, name: &str, contents: &str) -> String {
    let p = path(dir, name);
    std::fs::write(&p, contents).expect("write fixture file");
    p
}

// ── Fixtures ────────────────────────────────────────────────────────────────────

/// `A ⊑ B`, `x : A` — enough for `cax-sco` to type `x` a `B`, and nothing else.
const SUBCLASS_PREMISE: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
    "ex:A rdfs:subClassOf ex:B .\n",
    "ex:x a ex:A .\n",
);

/// `x : B` — the conclusion `cax-sco` derives from [`SUBCLASS_PREMISE`].
const DERIVED_CONCLUSION: &str = "@prefix ex: <http://example.org/> .\nex:x a ex:B .\n";

/// `x : Never` — a conclusion nothing derives, which the complete table REFUTES.
const REFUTED_CONCLUSION: &str = "@prefix ex: <http://example.org/> .\nex:x a ex:Never .\n";

/// `Boy ⊓ Girl = ⊥`, `Stewie : Boy`, `Peter : Girl` — the refutation lane's premise.
const DISJOINT_PREMISE: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
    "ex:Boy a owl:Class .\n",
    "ex:Girl a owl:Class .\n",
    "ex:Boy owl:disjointWith ex:Girl .\n",
    "ex:Stewie a ex:Boy .\n",
    "ex:Peter a ex:Girl .\n",
);

/// `Stewie ≠ Peter` — a NEGATIVE FACT, which no head in Tables 4–9 has the shape of.
const DIFFERENT_CONCLUSION: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
    "ex:Stewie owl:differentFrom ex:Peter .\n",
);

/// A premise giving three lanes something to establish: `Boy ⊓ Girl = ⊥` with
/// `Stewie : Boy` for refutation, `p ∘ p ⊑ p` for freeze, `knows` reflexive for
/// reflexivity.
const THREE_LANE_PREMISE: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
    "ex:Boy a owl:Class .\n",
    "ex:Girl a owl:Class .\n",
    "ex:Boy owl:disjointWith ex:Girl .\n",
    "ex:Stewie a ex:Boy .\n",
    "ex:p a owl:ObjectProperty .\n",
    "ex:p owl:propertyChainAxiom ( ex:p ex:p ) .\n",
    "ex:knows a owl:ReflexiveProperty .\n",
);

/// `p : owl:TransitiveProperty` — a SCHEMA AXIOM, which no head in Tables 4–9 has the
/// shape of either.
const TRANSITIVE_CONCLUSION: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
    "ex:p a owl:TransitiveProperty .\n",
);

/// `Stewie : ¬Girl` ∧ `p : owl:TransitiveProperty` ∧ `Girl : owl:Class` — one conclusion
/// graph whose halves need refutation, freeze, and an ordinary match.
const COMPOSITE_CONCLUSION: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
    "_:c a owl:Class .\n",
    "_:c owl:complementOf ex:Girl .\n",
    "ex:Stewie a _:c .\n",
    "ex:p a owl:TransitiveProperty .\n",
    "ex:Girl a owl:Class .\n",
);

/// An ontology whose axioms are its own PLUS the document it imports.
const IMPORTING_PREMISE: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
    "ex:o owl:imports ex:schema .\n",
    "ex:tom a ex:Cat .\n",
);

/// The document `ex:schema` names.
const IMPORTED_SCHEMA: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
    "ex:Cat rdfs:subClassOf ex:Animal .\n",
);

/// `tom : Animal` — reachable only through the imported schema.
const IMPORTED_CONCLUSION: &str = "@prefix ex: <http://example.org/> .\nex:tom a ex:Animal .\n";

// ── The mechanisms ──────────────────────────────────────────────────────────────

/// THE RULE TABLE DERIVES IT: `mechanism strict-table`, `entailment entailed`.
///
/// The base case, and the one the other five are defined against: this is the
/// chase-and-graph-match procedure OWL 2 Profiles §4.3 states the entailment relation in
/// terms of, and it is the only lane the closure of `purrdf reason` would have shown.
#[test]
fn the_rule_table_derives_a_conclusion_and_says_so() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let premise = write_file(dir, "premise.ttl", SUBCLASS_PREMISE);
    let conclusion = write_file(dir, "conclusion.ttl", DERIVED_CONCLUSION);

    let o = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        &premise,
        "--conclusion",
        &conclusion,
    ]);
    assert!(o.status.success(), "entails failed: {}", stderr(&o));
    assert_eq!(
        stdout(&o),
        "mechanism strict-table\nentailment entailed\n",
        "the rule table's own lane must be named, not merely used"
    );
}

/// A CONCLUSION NOTHING DERIVES IS REFUTED, and `not-entailed` is a PROOF.
///
/// `strict-table` is the only mechanism a `not-entailed` can carry, because refuting needs
/// the completeness half of a theorem and only the table has one. The `miss` line names the
/// triple that was absent, so the operator learns WHICH half of a conjunction failed.
#[test]
fn a_conclusion_the_table_refutes_is_not_entailed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let premise = write_file(dir, "premise.ttl", SUBCLASS_PREMISE);
    let conclusion = write_file(dir, "never.ttl", REFUTED_CONCLUSION);

    let o = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        &premise,
        "--conclusion",
        &conclusion,
    ]);
    assert!(o.status.success(), "entails failed: {}", stderr(&o));
    let answer = stdout(&o);
    assert!(
        answer.starts_with("mechanism strict-table\nentailment not-entailed\n"),
        "{answer}"
    );
    assert!(
        answer.contains("\nmiss closure lacks <http://example.org/x> "),
        "the miss must name the triple that was absent: {answer}"
    );
    // NOT `undecided`: the procedure was complete for this premise, so the absence of a
    // mapping is the absence of an entailment.
    assert!(!answer.contains("entailment undecided"), "{answer}");
}

/// A NEGATIVE FACT IS REACHED BY REFUTATION, and the mechanism is named.
///
/// No head in Tables 4–9 is an `owl:differentFrom`, so a forward chase derives nothing to
/// match against. The seventeen `false`-concluding rules are the calculus that decides it.
/// Falsifiable against a CLI that only post-processed `purrdf reason` output: the closure of
/// this premise does not contain this triple, and the answer is nonetheless `entailed`.
#[test]
fn a_negative_fact_is_entailed_by_refutation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let premise = write_file(dir, "disjoint.ttl", DISJOINT_PREMISE);
    let conclusion = write_file(dir, "different.ttl", DIFFERENT_CONCLUSION);

    let o = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        &premise,
        "--conclusion",
        &conclusion,
    ]);
    assert!(o.status.success(), "entails failed: {}", stderr(&o));
    assert_eq!(
        stdout(&o),
        "mechanism refutation\nentailment entailed\n",
        "a negative fact is reached by refutation, and the answer must say so"
    );

    // The closure really does NOT carry it: `reason` over the same premise proves the two
    // subcommands answer different questions.
    let closure = path(dir, "closure.nt");
    let o = run(&["reason", "--regime", "owl-rl", &premise, &closure]);
    assert!(o.status.success(), "reason failed: {}", stderr(&o));
    let text = std::fs::read_to_string(&closure).expect("read closure");
    assert!(
        !text.contains("owl#differentFrom"),
        "the closure must NOT contain the conclusion — that is why refutation exists: {text}"
    );
}

/// A SCHEMA AXIOM IS REACHED BY FREEZING, and the mechanism is named.
///
/// `p rdf:type owl:TransitiveProperty` abbreviates a universally quantified implication,
/// and no head in Tables 4–9 is a property characteristic. The lane freezes the body over
/// constants the premise does not mention, re-runs the table, and reads the head.
#[test]
fn a_schema_axiom_is_entailed_by_freezing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let premise = write_file(dir, "three-lane.ttl", THREE_LANE_PREMISE);
    let conclusion = write_file(dir, "transitive.ttl", TRANSITIVE_CONCLUSION);

    let o = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        &premise,
        "--conclusion",
        &conclusion,
    ]);
    assert!(o.status.success(), "entails failed: {}", stderr(&o));
    assert_eq!(stdout(&o), "mechanism freeze\nentailment entailed\n");
}

/// A CONCLUSION GRAPH IS A CONJUNCTION, so it can need a lane per half.
///
/// `mechanism composite` is spelled that way rather than by any one constituent's name,
/// which would tell a reader that one mechanism sufficed; the `constituent` lines then name
/// which lanes did the work, in the fixed cost order the fold tries them.
#[test]
fn a_conjunction_folds_into_a_composite_and_names_its_constituents() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let premise = write_file(dir, "three-lane.ttl", THREE_LANE_PREMISE);
    let conclusion = write_file(dir, "composite.ttl", COMPOSITE_CONCLUSION);

    let o = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        &premise,
        "--conclusion",
        &conclusion,
    ]);
    assert!(o.status.success(), "entails failed: {}", stderr(&o));
    assert_eq!(
        stdout(&o),
        concat!(
            "mechanism composite\n",
            "entailment entailed\n",
            "constituent refutation\n",
            "constituent freeze\n",
        ),
        "a folded answer must name every lane that contributed"
    );
}

// ── `--verify`: the warrant re-decided ──────────────────────────────────────────

/// `--verify` RE-DECIDES THE WARRANT without running a reasoner, and reports `verified true`.
#[test]
fn verify_re_decides_the_warrant() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let premise = write_file(dir, "premise.ttl", SUBCLASS_PREMISE);
    let conclusion = write_file(dir, "conclusion.ttl", DERIVED_CONCLUSION);

    let o = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        &premise,
        "--conclusion",
        &conclusion,
        "--verify",
    ]);
    assert!(
        o.status.success(),
        "entails --verify failed: {}",
        stderr(&o)
    );
    let answer = stdout(&o);
    assert!(answer.contains("\nwarrant present\n"), "{answer}");
    assert!(
        answer.ends_with("verified true\n"),
        "the re-check must report its own result: {answer}"
    );
}

/// A verdict with NO warrant reports `not-applicable`, never `false`.
///
/// `verified false` would read as a failed check rather than as an absent one, and the
/// distinction is the whole reason the two lines are separate.
#[test]
fn verify_without_a_warrant_is_not_applicable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let premise = write_file(dir, "premise.ttl", SUBCLASS_PREMISE);
    let conclusion = write_file(dir, "never.ttl", REFUTED_CONCLUSION);

    let o = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        &premise,
        "--conclusion",
        &conclusion,
        "--verify",
    ]);
    assert!(
        o.status.success(),
        "entails --verify failed: {}",
        stderr(&o)
    );
    let answer = stdout(&o);
    assert!(answer.contains("\nwarrant absent\n"), "{answer}");
    assert!(answer.ends_with("verified not-applicable\n"), "{answer}");
}

// ── `--pattern`: the certain answers ────────────────────────────────────────────

/// `--pattern` ANSWERS A BASIC GRAPH PATTERN with its certain answers.
///
/// A row is a substitution the knowledge base ENTAILS the pattern under, so `?c` ranges over
/// the entailed types rather than the asserted one; and with no `limit` line the row set is
/// exhaustive, which is a claim rather than a silence.
#[test]
fn a_pattern_answers_with_its_certain_answers() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let premise = write_file(
        dir,
        "cats.ttl",
        concat!(
            "@prefix ex: <http://example.org/> .\n",
            "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
            "ex:Cat rdfs:subClassOf ex:Animal .\n",
            "ex:tom a ex:Cat .\n",
        ),
    );
    // A pattern is N-Triples with `?name` in a term position — not an RDF document, so its
    // bytes go to the boundary untranscoded and `--from` says nothing about it.
    let pattern = write_file(
        dir,
        "types.bgp",
        "<http://example.org/tom> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?c .\n",
    );

    let o = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        &premise,
        "--pattern",
        &pattern,
    ]);
    assert!(
        o.status.success(),
        "entails --pattern failed: {}",
        stderr(&o)
    );
    let answer = stdout(&o);
    assert!(
        answer.starts_with("mechanism strict-table\nvar c\n"),
        "{answer}"
    );
    assert!(
        answer.contains("\nrow <http://example.org/Animal>\n"),
        "`?c` must range over the ENTAILED types: {answer}"
    );
    assert!(
        !answer.contains("\nlimit "),
        "nothing beyond the rule table was needed, so the row set is exhaustive: {answer}"
    );
}

/// `--pattern` PROJECTS A VARIABLE IN PREDICATE POSITION, LIKE ANY OTHER.
///
/// Falsifiable against what this replaced: `?s ?p ?o` — the most ordinary basic graph pattern
/// there is — exited 1 with `the basic graph pattern is not N-Triples: … predicate must be
/// IRI`, a refusal naming a construct the operator had not written, while `?s <p> ?o` over the
/// same premise answered fine.
#[test]
fn a_pattern_projects_a_variable_in_predicate_position() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let premise = write_file(
        dir,
        "cats.nt",
        concat!(
            "<http://example.org/Cat> \
             <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/Animal> .\n",
            "<http://example.org/tom> \
             <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Cat> .\n",
        ),
    );

    // THE WHOLE CLOSURE, three columns wide. `simple` is the identity closure, so the two
    // rows are the premise's own two triples and the answer can be asserted whole.
    let open = write_file(dir, "open.bgp", "?s ?p ?o .\n");
    let o = run(&[
        "entails",
        "--regime",
        "simple",
        "--premise",
        &premise,
        "--pattern",
        &open,
    ]);
    assert!(o.status.success(), "`?s ?p ?o` failed: {}", stderr(&o));
    assert_eq!(
        stdout(&o),
        concat!(
            "mechanism strict-table\nvar s\nvar p\nvar o\n",
            "row <http://example.org/Cat> \
             <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/Animal>\n",
            "row <http://example.org/tom> \
             <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Cat>\n",
        )
    );

    // …AND THE PREDICATE COLUMN RANGES OVER WHAT THE CHASE ENTAILED. No triple of the premise
    // states `tom rdf:type Animal`, so `rdfs9` is the only reason this row exists — which the
    // same question under `simple` proves by answering with no row at all.
    let bridge = write_file(
        dir,
        "bridge.bgp",
        "<http://example.org/tom> ?p <http://example.org/Animal> .\n",
    );
    let o = run(&[
        "entails",
        "--regime",
        "rdfs",
        "--premise",
        &premise,
        "--pattern",
        &bridge,
    ]);
    assert!(o.status.success(), "predicate variable: {}", stderr(&o));
    let answer = stdout(&o);
    assert!(
        answer.starts_with("mechanism strict-table\nvar p\n"),
        "{answer}"
    );
    assert!(
        answer.contains("\nrow <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>\n"),
        "{answer}"
    );

    let o = run(&[
        "entails",
        "--regime",
        "simple",
        "--premise",
        &premise,
        "--pattern",
        &bridge,
    ]);
    assert!(o.status.success(), "{}", stderr(&o));
    assert_eq!(stdout(&o), "mechanism strict-table\nvar p\n");
}

/// AN OPEN PREDICATE UNDER `owl-rl` IS A `limit` LINE, NOT A SILENTLY SHORT ANSWER.
///
/// `p ∘ p ⊑ p` entails `p rdf:type owl:TransitiveProperty` — `--conclusion` proves it, by the
/// freeze mechanism — and no rule of the OWL 2 RL table puts a schema triple in the closure.
/// So `?s ?p ?o` cannot return that row, and the answer says why instead of looking complete.
#[test]
fn an_open_predicate_renders_the_limit_that_makes_the_answer_honest() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let premise = write_file(
        dir,
        "chain.nt",
        concat!(
            "<http://example.org/p> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
             <http://www.w3.org/2002/07/owl#ObjectProperty> .\n",
            "<http://example.org/p> <http://www.w3.org/2002/07/owl#propertyChainAxiom> _:l1 .\n",
            "_:l1 <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> <http://example.org/p> .\n",
            "_:l1 <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> _:l2 .\n",
            "_:l2 <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> <http://example.org/p> .\n",
            "_:l2 <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> \
             <http://www.w3.org/1999/02/22-rdf-syntax-ns#nil> .\n",
        ),
    );
    let transitive = write_file(
        dir,
        "transitive.nt",
        "<http://example.org/p> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
         <http://www.w3.org/2002/07/owl#TransitiveProperty> .\n",
    );
    let o = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        &premise,
        "--conclusion",
        &transitive,
    ]);
    assert!(o.status.success(), "{}", stderr(&o));
    assert_eq!(
        stdout(&o),
        "mechanism freeze\nentailment entailed\n",
        "the freeze lane proves it, and no rule of the table does"
    );

    let open = write_file(dir, "open.bgp", "?s ?p ?o .\n");
    let o = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        &premise,
        "--pattern",
        &open,
    ]);
    assert!(o.status.success(), "{}", stderr(&o));
    let answer = stdout(&o);
    assert!(
        !answer.contains("owl#TransitiveProperty"),
        "the closure does not hold it: {answer}"
    );
    let limits: Vec<&str> = answer
        .lines()
        .filter(|line| line.starts_with("limit "))
        .collect();
    assert_eq!(limits.len(), 1, "{answer}");
    assert!(
        limits[0].starts_with("limit the question leaves the predicate open in 1 triple"),
        "{limits:?}"
    );
}

// ── `--import`: the documents the premise says it is not all of ─────────────────

/// `--import` ANSWERS A PREMISE WHOSE `owl:imports` IS INTACT.
///
/// OWL 2 defines an ontology's imports closure to BE the ontology, so the conclusion is
/// entailed only once the imported schema is supplied. This is the pair of assertions: the
/// SAME premise and conclusion, refused without the pair and entailed with it.
#[test]
fn an_import_pair_answers_a_premise_that_imports() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let premise = write_file(dir, "importing.ttl", IMPORTING_PREMISE);
    let schema = write_file(dir, "schema.ttl", IMPORTED_SCHEMA);
    let conclusion = write_file(dir, "animal.ttl", IMPORTED_CONCLUSION);
    let pair = format!("http://example.org/schema={schema}");

    // Without the pair: refused BY NAME, never a silently truncated premise.
    let o = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        &premise,
        "--conclusion",
        &conclusion,
    ]);
    assert_eq!(
        o.status.code(),
        Some(1),
        "an unresolved import is a refusal: {}",
        stderr(&o)
    );
    assert!(
        stderr(&o).contains("owl:imports <http://example.org/schema>"),
        "the refusal must name the document to supply: {}",
        stderr(&o)
    );

    // With it: answered from the imports closure.
    let o = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        &premise,
        "--conclusion",
        &conclusion,
        "--import",
        &pair,
    ]);
    assert!(
        o.status.success(),
        "entails --import failed: {}",
        stderr(&o)
    );
    assert_eq!(stdout(&o), "mechanism strict-table\nentailment entailed\n");
}

/// THE REPORT SAYS WHICH OF THE TWO IMPORT SITUATIONS THE RUN WAS IN.
///
/// `owl:imports` used to render ONE `boundary ontology-import` line whose text said the
/// imported axioms were "premises this run did not have" — on `entails --import`, where the
/// documents had been merged in and the conclusion was reached THROUGH them, exactly as on
/// `reason`, where nothing had been resolved at all. One token, two meanings, and no
/// consumer able to tell them apart.
///
/// This drives both paths over the SAME premise through the shipped binary and asserts the
/// two tokens, because the split is only worth anything if it survives to the rendered line
/// every host shares.
#[test]
fn the_report_distinguishes_a_resolved_import_from_an_unresolved_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let premise = write_file(dir, "importing.ttl", IMPORTING_PREMISE);
    let schema = write_file(dir, "schema.ttl", IMPORTED_SCHEMA);
    let conclusion = write_file(dir, "animal.ttl", IMPORTED_CONCLUSION);
    let pair = format!("http://example.org/schema={schema}");

    // RESOLVED: the operator supplied the document, and the conclusion is reachable only
    // through it — so the verdict itself proves the run HAD the imported axioms.
    let resolved_report = path(dir, "resolved.report");
    let o = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        &premise,
        "--conclusion",
        &conclusion,
        "--import",
        &pair,
        &format!("--report={resolved_report}"),
    ]);
    assert!(o.status.success(), "entails --import: {}", stderr(&o));
    assert_eq!(stdout(&o), "mechanism strict-table\nentailment entailed\n");
    let resolved = std::fs::read_to_string(&resolved_report).expect("read the report");
    assert!(
        resolved.contains("\nboundary ontology-import-resolved "),
        "a merged import closure must render the RESOLVED token: {resolved}"
    );
    assert!(
        !resolved.contains("boundary ontology-import-unresolved"),
        "…and never the one that says the axioms were missing: {resolved}"
    );
    assert!(
        resolved.contains("THIS RUN HAD THAT CLOSURE"),
        "the reason must say what is true of THIS token: {resolved}"
    );

    // UNRESOLVED: the same premise, materialized, where no import map exists at all.
    let closure = path(dir, "closure.nt");
    let unresolved_report = path(dir, "unresolved.report");
    let o = run(&[
        "reason",
        "--regime",
        "owl-rl",
        &premise,
        &closure,
        &format!("--report={unresolved_report}"),
    ]);
    assert!(o.status.success(), "reason: {}", stderr(&o));
    let unresolved = std::fs::read_to_string(&unresolved_report).expect("read the report");
    assert!(
        unresolved.contains("\nboundary ontology-import-unresolved "),
        "a materialization resolved nothing and must say so: {unresolved}"
    );
    assert!(
        !unresolved.contains("boundary ontology-import-resolved"),
        "…and must not claim a merge it never made: {unresolved}"
    );
    assert!(
        unresolved.contains("NOTHING RESOLVED THOSE DOCUMENTS FOR THIS RUN"),
        "the reason must say what is true of THIS token: {unresolved}"
    );

    // RESOLVING THE IMPORTS DOES NOT NARROW THE ANSWER. An `owl-rl` chase always meets the
    // datatype value space, so `exact-within-boundaries` is what both runs say — and the
    // assertion is that the two AGREE rather than that either spells a particular word, so a
    // later change that let the import boundary decide completeness would fail here.
    let line = |report: &str| {
        report
            .lines()
            .find(|l| l.starts_with("completeness "))
            .expect("every report carries a completeness line")
            .to_owned()
    };
    assert_eq!(line(&resolved), line(&unresolved));
}

// ── The refusals ────────────────────────────────────────────────────────────────

/// THE TWO UNSERVED REGIMES ARE REFUSED BY NAME, never answered under a weaker one.
///
/// `owl-direct` is directed by a query's class expressions and `rif` entails under the
/// caller's rule document, and "premise, conclusion, regime" carries neither. The refusal
/// travels from the shared boundary with the regime in it, so the operator learns which one
/// they asked for — and `purrdf reason` still materializes both.
#[test]
fn an_unserved_regime_is_refused_and_named() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let premise = write_file(dir, "premise.ttl", SUBCLASS_PREMISE);
    let conclusion = write_file(dir, "conclusion.ttl", DERIVED_CONCLUSION);

    for regime in ["owl-direct", "rif"] {
        let o = run(&[
            "entails",
            "--regime",
            regime,
            "--premise",
            &premise,
            "--conclusion",
            &conclusion,
        ]);
        assert_eq!(
            o.status.code(),
            Some(1),
            "{regime} must be refused: {}",
            stderr(&o)
        );
        let err = stderr(&o);
        assert!(
            err.contains(&format!("entailment regime \"{regime}\"")),
            "the refusal must name the regime the operator wrote: {err}"
        );
        assert!(
            err.contains("is not total over"),
            "the refusal must say WHY: {err}"
        );
        // Nothing was answered: a refusal is not a verdict with a warning attached.
        assert!(stdout(&o).is_empty(), "{}", stdout(&o));
    }

    // …and the same two regimes still MATERIALIZE, which is the point of refusing here.
    let closure = path(dir, "closure.nt");
    let o = run(&["reason", "--regime", "owl-direct", &premise, &closure]);
    assert!(
        o.status.success(),
        "`reason --regime owl-direct` must still materialize: {}",
        stderr(&o)
    );
}

/// A MALFORMED `--import` PAIR IS A USAGE ERROR, never a silently skipped import.
#[test]
fn a_malformed_import_pair_is_a_usage_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let premise = write_file(dir, "premise.ttl", SUBCLASS_PREMISE);
    let conclusion = write_file(dir, "conclusion.ttl", DERIVED_CONCLUSION);

    for spec in ["http://example.org/schema", "=schema.ttl", "ex:schema="] {
        let o = run(&[
            "entails",
            "--regime",
            "owl-rl",
            "--premise",
            &premise,
            "--conclusion",
            &conclusion,
            "--import",
            spec,
        ]);
        assert_eq!(
            o.status.code(),
            Some(2),
            "`--import {spec}` must be a usage error: {}",
            stderr(&o)
        );
        assert!(
            stderr(&o).contains("IRI=FILE"),
            "the refusal must state the shape it wanted: {}",
            stderr(&o)
        );
        assert!(stdout(&o).is_empty(), "nothing was answered");
    }
}

/// TWO DOCUMENTS READING STDIN IS REFUSED, never mis-read as one.
///
/// A process has a single standard input, so `--premise - --conclusion -` would give each
/// document part of one stream. The refusal names both flags.
#[test]
fn two_stdin_documents_are_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let schema = write_file(dir, "schema.ttl", IMPORTED_SCHEMA);

    // premise + conclusion.
    let o = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        "-",
        "--conclusion",
        "-",
        "--from",
        "turtle",
    ]);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(stderr(&o).contains("--premise"), "{}", stderr(&o));
    assert!(stderr(&o).contains("--conclusion"), "{}", stderr(&o));

    // premise + an import document is the same incoherence, and is refused the same way.
    let o = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        "-",
        "--conclusion",
        &schema,
        "--import",
        "http://example.org/schema=-",
        "--from",
        "turtle",
    ]);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(
        stderr(&o).contains("--import http://example.org/schema=-"),
        "{}",
        stderr(&o)
    );
}

/// EXACTLY ONE QUESTION: `--conclusion` and `--pattern` conflict, and one is required.
#[test]
fn exactly_one_question_is_asked() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let premise = write_file(dir, "premise.ttl", SUBCLASS_PREMISE);
    let conclusion = write_file(dir, "conclusion.ttl", DERIVED_CONCLUSION);
    let pattern = write_file(dir, "p.bgp", "?s ?p ?o .\n");

    let neither = run(&["entails", "--regime", "owl-rl", "--premise", &premise]);
    assert_eq!(neither.status.code(), Some(2), "{}", stderr(&neither));

    let both = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        &premise,
        "--conclusion",
        &conclusion,
        "--pattern",
        &pattern,
    ]);
    assert_eq!(both.status.code(), Some(2), "{}", stderr(&both));

    // `--verify` re-decides a WARRANT, and a relation has none, so it conflicts too.
    let verified_pattern = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        &premise,
        "--pattern",
        &pattern,
        "--verify",
    ]);
    assert_eq!(
        verified_pattern.status.code(),
        Some(2),
        "{}",
        stderr(&verified_pattern)
    );
}

/// AN INCONSISTENT PREMISE IS REFUSED WITH ITS WITNESS, not answered `entailed` for
/// everything.
///
/// An inconsistent knowledge base entails every triple, so a membership test against its
/// closure would answer `yes` to literally anything, correctly and uselessly. The chase
/// refuses instead, and the refusal names the rule and the premise count.
#[test]
fn an_inconsistent_premise_is_refused_with_its_witness() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let premise = write_file(
        dir,
        "clash.ttl",
        concat!(
            "@prefix ex: <http://example.org/> .\n",
            "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
            "ex:A owl:disjointWith ex:B .\n",
            "ex:x a ex:A .\n",
            "ex:x a ex:B .\n",
        ),
    );
    let conclusion = write_file(dir, "anything.ttl", REFUTED_CONCLUSION);

    let o = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        &premise,
        "--conclusion",
        &conclusion,
    ]);
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    assert!(stderr(&o).contains("cax-dw"), "{}", stderr(&o));
    // The refusal carries the run's certificate, beginning at the report banner.
    assert!(
        stderr(&o).contains("purrdf-reasoning-report 4\n"),
        "{}",
        stderr(&o)
    );
    assert!(stdout(&o).is_empty(), "nothing was answered");
}

// ── Formats, the sink, and `--report` ───────────────────────────────────────────

/// `--from` REACHES THE BOUNDARY: RDF/XML in, the same verdict out.
///
/// The boundary parses one media type (N-Quads). The CLI's own format resolution runs in
/// front of it, so a caller hands `entails` any of the nine syntaxes or a verified pack,
/// exactly as they would `reason`. A pack premise proves the resolution is the shared one
/// rather than a text-only shortcut.
#[test]
fn every_input_syntax_reaches_the_boundary() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let turtle = write_file(dir, "premise.ttl", SUBCLASS_PREMISE);
    let conclusion = write_file(dir, "conclusion.ttl", DERIVED_CONCLUSION);
    let expected = "mechanism strict-table\nentailment entailed\n";

    // RDF/XML, through the extension.
    let rdfxml = path(dir, "premise.rdf");
    let o = run(&["convert", "--to", "rdfxml", &turtle, &rdfxml]);
    assert!(o.status.success(), "convert failed: {}", stderr(&o));
    let o = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        &rdfxml,
        "--conclusion",
        &conclusion,
    ]);
    assert!(o.status.success(), "rdfxml premise: {}", stderr(&o));
    assert_eq!(stdout(&o), expected);

    // A verified pack, through the same resolution.
    let pack = path(dir, "premise.purrpck");
    let o = run(&["convert", "--to", "pack", &turtle, &pack]);
    assert!(o.status.success(), "convert failed: {}", stderr(&o));
    let o = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        &pack,
        "--conclusion",
        &conclusion,
    ]);
    assert!(o.status.success(), "pack premise: {}", stderr(&o));
    assert_eq!(stdout(&o), expected);

    // An extensionless stdin premise REQUIRES `--from`, exactly as `convert`/`reason` do.
    let bare = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        "-",
        "--conclusion",
        &conclusion,
    ]);
    assert_eq!(bare.status.code(), Some(2), "{}", stderr(&bare));
    assert!(stderr(&bare).contains("--from"), "{}", stderr(&bare));
}

/// A PREMISE ON STDIN with `--from` is answered, and the verdict goes to stdout.
#[test]
fn a_stdin_premise_is_answered() {
    use std::io::Write as _;

    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let conclusion = write_file(dir, "conclusion.ttl", DERIVED_CONCLUSION);

    let mut child = purrdf()
        .args([
            "entails",
            "--regime",
            "owl-rl",
            "--premise",
            "-",
            "--from",
            "turtle",
            "--conclusion",
            &conclusion,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the built purrdf binary");
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(SUBCLASS_PREMISE.as_bytes())
        .expect("write the premise to stdin");
    let o = child.wait_with_output().expect("wait for purrdf");

    assert!(o.status.success(), "stdin premise: {}", stderr(&o));
    assert_eq!(stdout(&o), "mechanism strict-table\nentailment entailed\n");
}

/// THE ANSWER GOES TO `OUT` and the certificate to `--report`, and they never mix.
///
/// `--report` is the same tri-state `reason --report` decodes, so an operator learns which
/// rules fired, which constructs the run could not fully handle, what it cost, the contract
/// hash of the calculus, and — the line this subcommand exists for — which mechanism read
/// the answer off the run.
#[test]
fn the_answer_and_the_certificate_are_separate_outputs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let premise = write_file(dir, "disjoint.ttl", DISJOINT_PREMISE);
    let conclusion = write_file(dir, "different.ttl", DIFFERENT_CONCLUSION);
    let answer = path(dir, "answer.txt");
    let first = path(dir, "first.report");
    let second = path(dir, "second.report");

    // Bare `--report` goes to stderr; the answer goes to the named sink.
    let o = run(&[
        "entails",
        "--regime",
        "owl-rl",
        "--premise",
        &premise,
        "--conclusion",
        &conclusion,
        "--report",
        &answer,
    ]);
    assert!(o.status.success(), "entails --report: {}", stderr(&o));
    assert!(stdout(&o).is_empty(), "the data channel is untouched");
    let err = stderr(&o);
    assert!(err.starts_with("purrdf-reasoning-report 4\n"), "{err}");
    assert!(err.contains("\nregime owl-rl\n"), "{err}");
    assert!(err.contains("\ncontract-hash "), "{err}");
    // The mechanism line carries the semantic boundary beside the name.
    assert!(err.contains("\nmechanism refutation "), "{err}");
    assert_eq!(
        std::fs::read_to_string(&answer).expect("read the answer"),
        "mechanism refutation\nentailment entailed\n"
    );

    // `--report=PATH` writes the same bytes to a file, and two runs agree byte for byte.
    for target in [&first, &second] {
        let flag = format!("--report={target}");
        let o = run(&[
            "entails",
            "--regime",
            "owl-rl",
            "--premise",
            &premise,
            "--conclusion",
            &conclusion,
            &flag,
        ]);
        assert!(o.status.success(), "entails --report=PATH: {}", stderr(&o));
        assert!(stderr(&o).is_empty(), "{}", stderr(&o));
    }
    assert_eq!(
        std::fs::read(&first).expect("read first report"),
        std::fs::read(&second).expect("read second report"),
        "an entails run twice must be byte-identical"
    );
}

/// THE TWO GLOBAL DOCUMENT FLAGS ARE REFUSED, not silently ignored.
///
/// `--loss-ledger` records what a conversion dropped and `--jsonld-options` configures an
/// RDF serializer; `entails` writes a verdict, so neither has anything to do. A flag that
/// quietly did nothing is precisely the shape this repository refuses.
#[test]
fn the_document_flags_are_refused_rather_than_ignored() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let premise = write_file(dir, "premise.ttl", SUBCLASS_PREMISE);
    let conclusion = write_file(dir, "conclusion.ttl", DERIVED_CONCLUSION);
    let options = write_file(
        dir,
        "jsonld-options.json",
        r#"{"version":1,"mode":"context","prefixes":{"ex":"http://example.org/"}}"#,
    );

    for extra in [
        vec!["--loss-ledger"],
        vec!["--jsonld-options", options.as_str()],
    ] {
        let mut args = extra.clone();
        args.extend([
            "entails",
            "--regime",
            "owl-rl",
            "--premise",
            &premise,
            "--conclusion",
            &conclusion,
        ]);
        let o = run(&args);
        assert_eq!(
            o.status.code(),
            Some(2),
            "{extra:?} must be refused: {}",
            stderr(&o)
        );
        assert!(
            stderr(&o).contains(extra[0]),
            "the refusal must name the flag: {}",
            stderr(&o)
        );
    }
}

/// The five served regimes all answer, and the answer names the regime that ran.
///
/// A test that only exercised `owl-rl` would certify a subcommand that hard-coded it. The
/// conclusion is asserted in the premise, so it is entailed under every regime — Simple
/// included, which states no rule at all.
#[test]
fn every_served_regime_answers() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let premise = write_file(dir, "premise.ttl", SUBCLASS_PREMISE);
    let conclusion = write_file(
        dir,
        "asserted.ttl",
        "@prefix ex: <http://example.org/> .\nex:x a ex:A .\n",
    );

    for regime in ["simple", "rdf", "rdfs", "owl-rl", "d"] {
        let report = path(dir, &format!("{regime}.report"));
        let flag = format!("--report={report}");
        let o = run(&[
            "entails",
            "--regime",
            regime,
            "--premise",
            &premise,
            "--conclusion",
            &conclusion,
            &flag,
        ]);
        assert!(o.status.success(), "{regime}: {}", stderr(&o));
        assert_eq!(
            stdout(&o),
            "mechanism strict-table\nentailment entailed\n",
            "{regime}"
        );
        let written = std::fs::read_to_string(&report).expect("read the certificate");
        assert!(
            written.contains(&format!("\nregime {regime}\n")),
            "the certificate must name the regime that ran: {written}"
        );
    }
}
