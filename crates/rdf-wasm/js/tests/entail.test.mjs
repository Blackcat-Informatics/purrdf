// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Node real-execution conformance for the entailment-REGIME surface reached through
// the PUBLIC package root (`../index.mjs`) — `entailMaterialize` / `entailRules` /
// `entailImplementedRules` / `entailExtensions` / `entailCheckGoldenVectors`. Not to be confused with
// `shaclEntail` (tests/shacl.test.mjs), which is SHACL-AF `sh:rule` entailment over a
// shapes graph; these close a document under a regime's own specification rule table.
//
// This file is where the tri-host claim is actually EXECUTED on wasm32. The repo has
// no `wasm-bindgen-test` harness (the crate carries no such dev-dependency, and adding
// one is out of scope), so the wasm leg of the assertion runs here instead: Node loads
// the real wasm-bindgen artifact `make wasm-pkg` produced and calls the SAME
// `check_regime_golden_vectors()` that the `purrdf-validate` and `purrdf-capi` Rust
// tests call, over the SAME committed artifact. wasm32 has a different pointer width,
// a different `usize`, different float behaviour and different map iteration; if any
// of those leaked into the chase, the canonical serializer or the report renderer,
// this is where it shows up.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

import {
  ready,
  entailCertainAnswers,
  entailCheckGoldenVectors,
  entailCheckInconsistentRefusal,
  entailClassify,
  entailConsistency,
  entailEntails,
  entailExplainConclusion,
  entailExtensions,
  entailExtractModule,
  entailGraphEntails,
  entailImplementedRules,
  entailInstances,
  entailJustify,
  entailMaterialize,
  entailProfile,
  entailRealize,
  entailRules,
  entailVerifyEntailment,
} from "../index.mjs";

await ready();

// `A ⊑ B ⊑ C`, and one typed instance — enough for rdfs9 to re-type it twice.
const SCHEMA = `<http://example.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/B> .
<http://example.org/B> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> .
<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .
`;

test("entailCheckGoldenVectors runs the committed tri-host artifact ON WASM", () => {
  // Throws with the case name and a diff of the two strings on the first byte that
  // differs from what the native build produced.
  entailCheckGoldenVectors();
});

test("an inconsistent input is refused WITH its certificate ON WASM", () => {
  // The path the golden artifact cannot cover: an inconsistent knowledge base has no
  // closure, so the only channel the evidence has is the thrown message. Same checker
  // as the C-ABI and `purrdf-validate` tests.
  entailCheckInconsistentRefusal();
  // …and a caller who reaches the boundary directly sees the same bytes: the witness
  // rule, the graph whose closure refused, and the three asserted triples.
  assert.throws(
    () =>
      entailMaterialize(
        [
          "<http://example.org/A> <http://www.w3.org/2002/07/owl#disjointWith> <http://example.org/B> .",
          "<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .",
          "<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/B> .",
          "",
        ].join("\n"),
        "owl-rl",
        "",
      ),
    /inconsistency-premise <http:\/\/example\.org\/A>/,
  );
});

// The same artifact, parsed here and driven case by case through the JS boundary — so
// the comparison covers the wasm→JS string marshalling too, not only the byte equality
// the checker establishes inside the module.
const VECTORS = fileURLToPath(
  new URL("../../../validate/tests/fixtures/regime-boundary.vectors", import.meta.url),
);

/** Parse the line-oriented `@case/@regime/@input/@program/@closure/@report/@end` artifact. */
function parseVectors(text) {
  const cases = [];
  let current = {};
  let open = null;
  for (const line of text.split("\n").slice(0, -1)) {
    if (!line.startsWith("@")) {
      if (open !== null) current[open] += `${line}\n`;
      continue;
    }
    const [keyword, ...rest] = line.slice(1).split(" ");
    open = null;
    if (keyword === "case") current.name = rest.join(" ").trim();
    else if (keyword === "regime") current.regime = rest.join(" ").trim();
    else if (
      keyword === "input" ||
      keyword === "program" ||
      keyword === "closure" ||
      keyword === "report"
    ) {
      current[keyword] = "";
      open = keyword;
    } else if (keyword === "end") {
      cases.push(current);
      current = {};
    }
  }
  return cases;
}

test("every golden case is byte-identical across the wasm/JS boundary", async () => {
  const cases = parseVectors(await readFile(VECTORS, "utf8"));
  assert.ok(cases.length > 0, "the artifact must hold cases");
  for (const vector of cases) {
    const closed = entailMaterialize(vector.input, vector.regime, vector.program ?? "");
    assert.equal(closed.nquads, vector.closure, `${vector.name}: closure`);
    assert.equal(closed.report, vector.report, `${vector.name}: report`);
  }
});

test("entailMaterialize closes under rdfs and always returns a report", () => {
  const closed = entailMaterialize(SCHEMA, "rdfs", "");
  assert.match(
    closed.nquads,
    /<http:\/\/example\.org\/x> <http:\/\/www\.w3\.org\/1999\/02\/22-rdf-syntax-ns#type> <http:\/\/example\.org\/C> \./,
  );
  assert.ok(closed.report.startsWith("purrdf-reasoning-report 4\n"));
  assert.ok(closed.report.includes("\nregime rdfs\n"));
  // The report says what the run could NOT do, rather than claiming completeness
  // it does not have. Asserted as the invariant, not as a `sound-incomplete <n>`
  // literal: the count moves as rules land, and a `boundary` line outlives a rule
  // table going complete.
  assert.ok(closed.report.includes("\ncompleteness "));
  assert.ok(closed.report.includes("\nboundary "));
  // The only observable rdfD1 / rdfD1a / rdfs14 / rdfs14a have: all four fire and none
  // of their conclusions can reach a `fired` line. It reached the command line only,
  // which left them invisible from exactly the hosts the report exists for.
  assert.ok(closed.report.includes("\nwithheld-surrogates "));
  assert.ok(closed.report.endsWith("inconsistency none\n"));
});

test("entailMaterialize under simple is the identity closure", () => {
  const closed = entailMaterialize(SCHEMA, "simple", "");
  assert.ok(
    !closed.nquads.includes(
      "<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/C> .",
    ),
  );
  assert.ok(closed.report.includes("\ncompleteness exact\n"));
});

test("entailMaterialize is byte-stable across repeated calls", () => {
  const first = entailMaterialize(SCHEMA, "owl-rl", "");
  for (let i = 0; i < 5; i += 1) {
    const again = entailMaterialize(SCHEMA, "owl-rl", "");
    assert.equal(again.nquads, first.nquads);
    assert.equal(again.report, first.report);
  }
});

test("entailMaterialize rejects an unknown regime, naming the accepted set", () => {
  assert.throws(() => entailMaterialize(SCHEMA, "rdfs-plus", ""), /accepted: simple, rdf, rdfs/);
  // The spellings are case-sensitive, exactly as the CLI writes them.
  assert.throws(() => entailMaterialize(SCHEMA, "RDFS", ""), /accepted:/);
});

// A normative RIF-in-XML rule document: `?x a ex:A` => `?x a ex:B`. `rif` is the
// one regime whose calculus is the CALLER's, so it is the one spelling whose
// `program` argument is a document rather than the empty string.
const RIF_PROGRAM =
  '<Document xmlns="http://www.w3.org/2007/rif#"><payload><Group><sentence><Forall><declare><Var>x</Var></declare><formula><Implies><if><Frame><object><Var>x</Var></object><slot><Const type="http://www.w3.org/2007/rif#iri">http://www.w3.org/1999/02/22-rdf-syntax-ns#type</Const><Const type="http://www.w3.org/2007/rif#iri">http://example.org/A</Const></slot></Frame></if><then><Frame><object><Var>x</Var></object><slot><Const type="http://www.w3.org/2007/rif#iri">http://www.w3.org/1999/02/22-rdf-syntax-ns#type</Const><Const type="http://www.w3.org/2007/rif#iri">http://example.org/B</Const></slot></Frame></then></Implies></formula></Forall></sentence></Group></payload></Document>';

test("entailMaterialize materializes every regime spelling", () => {
  // Falsifiable against the old behavior: `owl-direct` and `rif` threw here with a
  // message naming the five spellings that were not refused.
  for (const [regime, program] of [
    ["simple", ""],
    ["rdf", ""],
    ["rdfs", ""],
    ["owl-rl", ""],
    ["owl-direct", ""],
    ["rif", RIF_PROGRAM],
    ["d", ""],
  ]) {
    const closed = entailMaterialize(SCHEMA, regime, program);
    assert.match(closed.report, /^purrdf-reasoning-report 4\n/);
    assert.ok(closed.report.includes(`\nregime ${regime}\n`), regime);
    assert.ok(closed.report.includes("\nwithheld-surrogates "), regime);
    assert.ok(closed.report.endsWith("inconsistency none\n"), regime);
  }
});

test("a rule document belongs to rif alone and is refused elsewhere", () => {
  assert.throws(
    () => entailMaterialize(SCHEMA, "rdfs", RIF_PROGRAM),
    /takes no rule document/,
  );
});

test("entailMaterialize rejects a malformed document (never a silent empty closure)", () => {
  assert.throws(() => entailMaterialize("this is not n-quads\n", "rdfs"));
});

test("the rule inventories are the specification tables, and the gap is measurable", () => {
  // OWL 2 Profiles §4.3 Tables 4-9; RDF 1.2 Semantics §8.1.1 and §9.2.1.
  assert.equal(entailRules("owl-rl").length, 78);
  assert.equal(entailRules("rdfs").length, 18);
  assert.equal(entailRules("rdf").length, 3);
  // A regime with no rule table is an empty array, not `[""]`.
  assert.deepEqual(entailRules("simple"), []);
  assert.deepEqual(entailImplementedRules("simple"), []);

  // The implemented set is a subsequence of the defined set — no additions, and
  // the gap is legitimately empty for a regime whose table is fully implemented…
  for (const regime of ["rdf", "rdfs", "owl-rl", "d"]) {
    const defined = entailRules(regime);
    const fired = entailImplementedRules(regime);
    assert.ok(fired.length <= defined.length, `${regime}: no rule is invented`);
    for (const rule of fired) assert.ok(defined.includes(rule), `${regime}: ${rule}`);
  }

  // …and the difference is exactly what the report's `missing` lines name.
  const missing = entailMaterialize(SCHEMA, "rdfs", "")
    .report.split("\n")
    .filter((line) => line.startsWith("missing "))
    .map((line) => line.slice("missing ".length));
  const defined = entailRules("rdfs");
  const fired = entailImplementedRules("rdfs");
  assert.deepEqual(
    missing,
    defined.filter((rule) => !fired.includes(rule)),
  );
});

test("the rule inventories reject an unknown regime, naming the accepted set", () => {
  assert.throws(() => entailRules("rdfs-plus"), /accepted: simple, rdf, rdfs/);
  assert.throws(() => entailImplementedRules("rdfs-plus"), /accepted: simple, rdf, rdfs/);
  assert.throws(() => entailExtensions("rdfs-plus"), /accepted: simple, rdf, rdfs/);
});

test("entailExtensions names what this build adds beyond the specification table", () => {
  // Asking is not the same as materializing: a caller learns what the build adds
  // without closing a dataset first, which is the whole reason this is bound.
  assert.deepEqual(entailExtensions("owl-rl"), ["ext-eq-diff-sym"]);

  // Extending a lane is a decision taken per lane, and only one has been taken.
  for (const regime of ["simple", "rdf", "rdfs", "owl-direct", "rif", "d"]) {
    assert.deepEqual(entailExtensions(regime), [], regime);
  }

  // The load-bearing invariant: an extension is in NEITHER normative inventory,
  // for every regime. The 78 stays 78 because a sound rule the table omits does
  // not change what the table says.
  for (const regime of ["simple", "rdf", "rdfs", "owl-rl", "owl-direct", "rif", "d"]) {
    const defined = entailRules(regime);
    const fired = entailImplementedRules(regime);
    for (const rule of entailExtensions(regime)) {
      assert.ok(!defined.includes(rule), `${regime}: ${rule} not in rules()`);
      assert.ok(!fired.includes(rule), `${regime}: ${rule} not in implemented()`);
    }
  }
  assert.equal(entailRules("owl-rl").length, 78);
  assert.equal(entailImplementedRules("owl-rl").length, 78);

  // And the report's `extension` line names the same rules the inventory does,
  // so the two disclosures cannot drift apart.
  const reported = entailMaterialize(SCHEMA, "owl-rl", "")
    .report.split("\n")
    .filter((line) => line.startsWith("extension "))
    .map((line) => line.slice("extension ".length));
  assert.deepEqual(reported, entailExtensions("owl-rl"));
});

// ── The Description-Logic reasoning services, driven through the PACKAGE ROOT ──
//
// Not deep imports of `./pkg/purrdf_wasm.js` (the `exports` map in package.json
// refuses that with ERR_PACKAGE_PATH_NOT_EXPORTED) — every call below goes through
// `../index.mjs`, the same public entry point `import "@blackcatinformatics/purrdf"`
// resolves to. Each service is actually CALLED and its `.answer`/`.certificate`
// getters are inspected; a `typeof fn === "function"` check would pass even if the
// service silently returned garbage, which is exactly what nine wasm-bindgen
// exports sitting compiled-but-unreachable would have looked like from here.

// `A ⊑ B ⊑ C`, `D ⊑ C`, and one instance of `A` — entails `x : B`, `x : C`, and the
// unasserted axiom `A ⊑ C` without ever asserting either.
const TAXONOMY = `<http://example.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/B> .
<http://example.org/B> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> .
<http://example.org/D> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> .
<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .
`;

// `A ⊑ C` — entailed by the chain, asserted nowhere in TAXONOMY.
const CHAIN_AXIOM =
  "<http://example.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> .\n";

test("entailConsistency decides consistency ON WASM, with its certificate", () => {
  const decided = entailConsistency(TAXONOMY, 0);
  assert.equal(decided.answer, "consistency true\n");
  // Never optional. `completeness decided` is reported only because the boundary
  // list beside it is, in fact, empty — the DL certificate's own honesty gate.
  assert.ok(decided.certificate.startsWith("purrdf-dl-certificate 1\n"));
  assert.ok(decided.certificate.includes("\nservice consistency\n"));
  assert.ok(decided.certificate.includes("\ncompleteness decided\n"));
  assert.ok(!decided.certificate.includes("\nboundary "));
});

test("entailClassify reaches the full subsumption hierarchy ON WASM", () => {
  const classified = entailClassify(TAXONOMY, 0);
  // Direct: only the asserted edges. Transitive: also the chain A -> C.
  assert.ok(
    classified.answer.includes(
      "direct <http://example.org/A> <http://example.org/B>\n",
    ),
  );
  assert.ok(
    classified.answer.includes(
      "subclass <http://example.org/A> <http://example.org/C>\n",
    ),
    classified.answer,
  );
  assert.ok(
    !classified.answer.includes(
      "direct <http://example.org/A> <http://example.org/C>\n",
    ),
    "A -> C is transitive, not direct",
  );
  assert.ok(classified.certificate.startsWith("purrdf-dl-certificate 1\n"));
  assert.ok(classified.certificate.includes("\nservice classify\n"));
});

test("entailRealize reaches the individuals' entailed types ON WASM", () => {
  const realized = entailRealize(TAXONOMY, 0);
  assert.ok(
    realized.answer.includes(
      "type <http://example.org/x> <http://example.org/C>\n",
    ),
    realized.answer,
  );
  assert.ok(
    realized.answer.includes(
      "direct-type <http://example.org/x> <http://example.org/A>\n",
    ),
    realized.answer,
  );
  assert.ok(realized.certificate.includes("\nservice realize\n"));
});

test("entailInstances retrieves instances for a class the schema never asserts them of ON WASM", () => {
  const instances = entailInstances(TAXONOMY, "<http://example.org/C>", 0);
  assert.ok(
    instances.answer.includes("instance <http://example.org/x>\n"),
    instances.answer,
  );
  assert.ok(instances.certificate.includes("\nservice instances\n"));
});

test("entailEntails decides an axiom entailed nowhere by assertion ON WASM", () => {
  const decided = entailEntails(TAXONOMY, CHAIN_AXIOM, 0);
  assert.ok(decided.answer.startsWith("entails true\n"), decided.answer);
  assert.ok(decided.answer.includes("\naxiom SubClassOf\n"));
  assert.ok(decided.answer.includes("\nterm <http://example.org/A>\n"));
  assert.ok(decided.answer.includes("\nterm <http://example.org/C>\n"));
  assert.ok(decided.certificate.includes("\nservice entails\n"));
});

test("entailEntails answers unknown (never false) on an exhausted budget ON WASM", () => {
  const starved = entailEntails(TAXONOMY, CHAIN_AXIOM, 1);
  assert.equal(starved.answer.split("\n")[0], "entails unknown");
  assert.ok(starved.certificate.includes("\ncompleteness budget-exhausted\n"));
});

test("entailProfile certifies the most restrictive profile first ON WASM", () => {
  const certified = entailProfile(TAXONOMY);
  assert.equal(certified.answer.split("\n")[0], "certified EL");
  assert.ok(certified.certificate.startsWith("purrdf-owl-profile-certificate 1\n"));
  assert.ok(certified.certificate.includes("\nservice profile\n"));
  // A certification proves membership; a violation never proves exclusion.
  assert.ok(certified.certificate.endsWith("one-directional true\n"));
});

test("entailExtractModule extracts a locality module as canonical N-Quads ON WASM", () => {
  const extracted = entailExtractModule(TAXONOMY, "<http://example.org/A>\n", "star");
  assert.ok(
    extracted.answer.includes(
      "<http://example.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/B> .",
    ),
    extracted.answer,
  );
  assert.ok(extracted.certificate.startsWith("purrdf-module-extraction 1\n"));
  assert.ok(extracted.certificate.includes("\nservice extract-module\n"));
  assert.ok(extracted.certificate.includes("\nmethod STAR\n"));
});

test("entailExtractModule rejects an unknown method, naming the accepted set", () => {
  assert.throws(
    () => entailExtractModule(TAXONOMY, "<http://example.org/A>\n", "nested"),
    /bot, top, star/,
  );
});

test("entailJustify finds a minimal entailing subset ON WASM", () => {
  const why = entailJustify(TAXONOMY, CHAIN_AXIOM);
  // The chain (A ⊑ B ⊑ C) — and NOT the irrelevant D ⊑ C axiom.
  assert.ok(
    why.answer.includes(
      "<http://example.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/B> .",
    ),
    why.answer,
  );
  assert.ok(!why.answer.includes("<http://example.org/D>"), why.answer);
  assert.ok(why.certificate.startsWith("purrdf-justification 1\n"));
  assert.ok(why.certificate.includes("\nservice justify\n"));
  assert.ok(why.certificate.includes("\naxiom SubClassOf\n"));
  assert.ok(why.certificate.includes("\nsufficient true\n"));
  assert.ok(why.certificate.endsWith("minimal true\n"));
});

test("entailExplainConclusion derives a chase conclusion never asserted ON WASM", () => {
  const proof = entailExplainConclusion(
    TAXONOMY,
    "owl-rl",
    "<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/C> .\n",
  );
  assert.ok(proof.answer.startsWith("asserted false\n"), proof.answer);
  assert.ok(proof.answer.includes("\nrule "), proof.answer);
  assert.ok(proof.certificate.startsWith("purrdf-chase-proof 1\n"));
  assert.ok(proof.certificate.includes("\nservice explain-conclusion\n"));
  // `checked` is the RE-DERIVED verdict, the terminal line of a chase-proof
  // certificate: the checker walked the premises to the head independently of
  // what the proof claims.
  assert.ok(proof.certificate.endsWith("checked true\n"));
});

test("the existential refusal is per conclusion, not per regime ON WASM", () => {
  // `rdfs` derives the chain axiom through plain Datalog rules, so it explains —
  // even though four of its eighteen rules are existential. `rdf`, whose
  // three-rule table cannot reach the same conclusion beside two existential
  // rules that might, refuses by name.
  const proof = entailExplainConclusion(TAXONOMY, "rdfs", CHAIN_AXIOM);
  assert.ok(proof.certificate.endsWith("checked true\n"));
  assert.throws(
    () => entailExplainConclusion(TAXONOMY, "rdf", CHAIN_AXIOM),
    /existential/,
  );
});

test("every DL reasoning service is reachable from the package root, not only ./pkg/", () => {
  // Falsifiable against the dark-feature defect: these nine names were compiled
  // into the shipped wasm binary (the size budget already paid for them) but
  // `../index.mjs` re-exported none of them, and the npm `exports` map refuses a
  // deep `./pkg/` import — so no consumer of the published package could reach any
  // of them at all.
  for (const fn of [
    entailConsistency,
    entailClassify,
    entailRealize,
    entailInstances,
    entailEntails,
    entailProfile,
    entailExtractModule,
    entailJustify,
    entailExplainConclusion,
  ]) {
    assert.equal(typeof fn, "function");
  }
});

test("every conclusion-directed entailment service is reachable from the package root", () => {
  // The same dark-feature argument as the nine above, made for the three services of
  // the CHASE lane: compiled in, budgeted for, and worth nothing if `../index.mjs`
  // does not re-export them, because the npm `exports` map refuses a deep `./pkg/`
  // import. `scripts/check-entailment-surface.py` gates the re-export structurally;
  // this executes it on real wasm.
  for (const fn of [entailCertainAnswers, entailGraphEntails, entailVerifyEntailment]) {
    assert.equal(typeof fn, "function");
  }

  const conclusion =
    "<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/C> .\n";
  const pattern =
    "<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?c .\n";

  // `?c` ranges over the ENTAILED types, so `C` is a row and it is asserted nowhere.
  const answers = entailCertainAnswers("owl-rl", SCHEMA, pattern);
  assert.ok(answers.answer.startsWith("mechanism strict-table\nvar c\n"), answers.answer);
  assert.ok(answers.answer.includes("\nrow <http://example.org/C>\n"), answers.answer);

  const decided = entailGraphEntails("owl-rl", SCHEMA, conclusion);
  assert.equal(decided.answer, "mechanism strict-table\nentailment entailed\n");

  const checked = entailVerifyEntailment("owl-rl", SCHEMA, conclusion);
  assert.ok(checked.answer.endsWith("warrant present\nverified true\n"), checked.answer);

  // All three carry the run that answered, on the materialization lane's own banner,
  // naming the mechanism. The mechanism crosses this boundary as its canonical
  // spelling and never as an enum ordinal, so a seventh mechanism cannot renumber a
  // JS consumer's reading of an old one.
  for (const produced of [answers, decided, checked]) {
    assert.match(produced.certificate, /^purrdf-reasoning-report 4\n/);
    assert.ok(produced.certificate.includes("\nmechanism strict-table "), produced.certificate);
  }
});

test("a conclusion nothing derives has no warrant, and says so", () => {
  const never =
    "<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Never> .\n";
  const checked = entailVerifyEntailment("owl-rl", SCHEMA, never);
  assert.ok(checked.answer.includes("\nentailment not-entailed\n"), checked.answer);
  // `not-applicable`, never `false`: there is no evidence to re-decide, and a `false`
  // would read as a check that ran and failed.
  assert.ok(
    checked.answer.endsWith("warrant absent\nverified not-applicable\n"),
    checked.answer,
  );
});

test("the two regimes defined by a missing input are refused by name", () => {
  const conclusion =
    "<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/C> .\n";
  for (const regime of ["owl-direct", "rif"]) {
    assert.throws(() => entailGraphEntails(regime, SCHEMA, conclusion), new RegExp(regime));
  }
});
