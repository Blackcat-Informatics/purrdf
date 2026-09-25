// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! W3C SHACL 1.2 conformance harness over the vendored `vectors/shacl12/tests`
//! suite (w3c/data-shapes `shacl12-test-suite`), run through the LIBRARY API.
//!
//! Discovery is [`shacl_corpora::shacl12`]: every manifest root, every test type,
//! with the reachability guards that keep "discovered" and "on disk" the same
//! corpus. This file GRADES each discovered entry by its type:
//!
//! * **`sht:Validate`** — [`shacl_corpora::report_grading::run_validate_case`],
//!   the grader the SHACL 1.0 harness uses, so both suites mean the same thing by
//!   "the report agrees": `sh:conforms` plus the multiset of
//!   `(focusNode, resultPath, value, sourceConstraintComponent, severity)`
//!   tuples, blank nodes normalized, every `sh:resultMessage` the expected report
//!   mentions, and — where the expected report states `sh:conformanceDisallows`
//!   — validation under exactly that set, echoed back in the report. `sht:Failure` expects an error at load or
//!   validation. The only departure from the approved expectation is
//!   [`EXPECTATION_DEFECTS`].
//! * **`sht:EvalNodeExpr`** — the `sht:nodeExpr` node is parsed by the shapes
//!   parser itself ([`purrdf_shapes::shapes::from_dataset_with_node_expressions`],
//!   so custom functions and shape references bind exactly as they do in a
//!   shapes graph), and evaluated with `sht:focusNode` as the focus node and every
//!   `sht:scope-NAME` bound through the public [`Scope`]/[`Binding`] API. The
//!   output must equal the `mf:result` list TERM FOR TERM (RDF 1.2 term
//!   equality: lexical form, datatype, language and direction), in order unless
//!   the entry sets `sht:ignoreOrder true`, in which case as a multiset. The only
//!   departure from exact equality is [`NON_CANONICAL_EXPECTATIONS`].
//! * **`sht:Infer`** — the shapes graph's rules run through
//!   [`purrdf_shapes::apply_rules`] over the data graph, and the inferred graph
//!   must equal `data ⊎ expected` under RDF isomorphism (RDFC-1.0 canonical
//!   N-Quads, as the SHACL Rules harness compares). The expectation is the inline
//!   triple list, `rdf:nil`, or a result file; `sht:Failure` expects an error.
//! * **`srlt:*`** — the seven SPARQL 1.2 RL test types, routed to [`run_srl`].
//!
//! ## An entry with no `sht:focusNode`
//!
//! The suite's node-expression README makes `sht:focusNode` optional and says
//! nothing of what the focus node then is. The library's evaluator always takes
//! one, so such an entry is evaluated from [`ABSENT_FOCUS`], a blank node the
//! harness proves occurs nowhere in the test graph: it has no path values, is an
//! instance of nothing and conforms to nothing the graph could say about it.
//!
//! ## Ledger
//!
//! [`XFAIL`] names every entry the engine fails today, with the reason. The
//! harness asserts that every ledgered entry still fails (an `XPASS` is an error:
//! remove the entry), that every other entry passes, and the exact discovered
//! total. Run with `--nocapture` for the per-section and per-type scoreboard:
//! `cargo test -p purrdf-shapes --test w3c12_conformance -- --nocapture`

mod shacl_corpora;

use std::collections::BTreeMap;
use std::fs;
use std::sync::Arc;

use purrdf::{RdfDataset, RdfDatasetBuilder, RdfQuad, canonicalize};
use purrdf_shapes::data::{GraphFilter, ShaclData, native_quads};
use purrdf_shapes::expression::{
    Binding, NodeExpr, RecursionGuard, Scope, eval_node_expr_in_scope,
};
use purrdf_shapes::term::{Literal, NamedNode, Term};
use purrdf_shapes::{apply_rules, engine, shapes, sparql, text_ingest};

use shacl_corpora::report_grading::{grade, grade_against, no_panic, produce};
use shacl_corpora::shacl12::{
    Body, Case12, InferCase, InferExpected, NodeExprCase, SrlCase, W3C12_TOTAL_CASES, shacl12_cases,
};
use shacl_corpora::{Expected, Multiset, Tuple, W3cCase, file_iri, parse_turtle_file};

// ── Xfail ledger ──────────────────────────────────────────────────────────────

/// Why every SPARQL 1.2 RL entry fails today.
const NO_SRL: &str = "no SPARQL 1.2 RL implementation";

const R_SHAPE_CLASS_SUBJECT: &str = "a shape typed sh:ShapeClass is refused at load (implicit class targets are not \
     evaluated); the rule also omits sh:subject, which SHACL 1.2 Rules defaults to the focus \
     node";

const R_SHAPE_CLASS_RUN_ONCE: &str = "a shape typed sh:ShapeClass is refused at load (implicit class targets are not \
     evaluated); sh:runOnce / sh:tempTriple are not implemented either";

const R_SHAPE_CLASS_TEMPLATE: &str = "a shape typed sh:ShapeClass is refused at load (implicit class targets are not \
     evaluated); sh:SPARQLRuleTemplate is not implemented either";

const R_SHAPE_TARGET: &str = "sh:shape target declarations in the data graph are not read, so no focus node is \
     selected";

const R_SHAPE_CLASS: &str = "sh:ShapeClass implicit class targets are not evaluated, so a shape typed \
     sh:ShapeClass is refused at load";

const R_TARGET_WHERE: &str =
    "sh:targetWhere is not evaluated, so a shape using it is refused at load";

const R_TARGET_NODE_EXPR: &str = "a structured node-expression (sh:select) value of sh:targetNode is not \
     evaluated, so the shape is refused at load";

const R_VALUES: &str =
    "sh:values on a property shape is not evaluated, so the shape is refused at load";

const R_PREFIX_SCAN: &str = "prefix handling scans PREFIX lines out of the shapes document text, including one \
     inside a string literal, so the query is resolved against the wrong namespace";

const R_INSTANCES_OF_EXPR: &str = "shnex:instancesOf refuses a node-expression argument (shnex:arg) and requires an \
     IRI, so the function body is rejected at load";

const R_RULES_ENTAILMENT: &str = "sh:entailment sh:RulesEntailment is not supported, so the shapes graph is refused \
     at load, as SHACL requires for an entailment regime a processor does not support";

const R_TRIPLE_RULE_DEFAULT_SUBJECT: &str = "sh:TripleRule without sh:subject is refused; SHACL 1.2 Rules defaults sh:subject to \
     the focus node";

const R_GLOBAL_RULES: &str = "global rules (sh:SPARQLRule not attached to a shape, of a sh:ShapesGraph / \
     sh:RulesGraph) are not executed";

const R_LAYER: &str = "sh:layer is not implemented, so layered rules do not run in layer order";

const R_RUN_ONCE: &str = "sh:runOnce / sh:tempTriple are not implemented, so run-once rules and temporary \
     triples do not behave as specified";

const R_RULE_TEMPLATE: &str = "sh:SPARQLRuleTemplate is not implemented: template instances are not recognised as \
     rules and a missing template parameter is not refused";

const R_RULE_PROCESSOR: &str = "sh:ruleProcessor is not validated: an unknown processor at rule or rule-set level \
     runs instead of being refused";

const R_EXPECTED_PREDICATE: &str = "sh:values / sh:defaultValue on a property shape are not evaluated, so the shapes \
     graph the rule depends on is refused at load";

const R_UNBOUND_ARG: &str = "an unbound shnex:var argument makes the sparql: call yield nothing instead of \
     reaching BOUND/COALESCE as unbound";

const R_SEQUENCE_ORDER: &str = "node-expression output is set-shaped and canonically sorted: shnex:filterShape / \
     shnex:distinct do not keep input order (distinct must keep first occurrences)";

const R_EMPTY_SHAPE: &str = "shnex:findFirst / shnex:matchAll refuse the well-formed empty shape [] as an \
     undefined shape";

const R_ORDER_BY_UNBOUND: &str = "shnex:orderBy errors on a node whose sort key is unbound; the spec sorts such nodes \
     first";

/// Entries the engine currently fails, with the reason: `(test id, reason)`,
/// where the id is the entry's IRI relative to `vectors/shacl12/tests` (see
/// `shacl12::entry_id` for the SPARQL 1.2 RL evaluation entries). Grouped by
/// feature; each reason names what the engine does today, not a plan.
///
/// A ledgered entry MUST fail; when engine work fixes it the harness errors with
/// `XPASS` and the entry must be removed.
const XFAIL: &[(&str, &str)] = &[
    // ── Targets ──
    ("core/targets/shape-001", R_SHAPE_TARGET),
    ("core/targets/targetClassImplicit-002", R_SHAPE_CLASS),
    ("core/targets/targetWhere-001", R_TARGET_WHERE),
    ("sparql/targets/targetNode-select-001", R_TARGET_NODE_EXPR),
    // ── SPARQL surface ──
    ("sparql/property/property-select-001", R_VALUES),
    ("sparql/property/property-sparqlExpr-001", R_VALUES),
    ("sparql/node/prefixes-002", R_PREFIX_SCAN),
    (
        "sparql/functions/instanceCount-example",
        R_INSTANCES_OF_EXPR,
    ),
    // ── SHACL rules (sht:Infer and the rules entailment regime) ──
    (
        "inference-rules/rules-entailment-validation",
        R_RULES_ENTAILMENT,
    ),
    (
        "inference-rules/TripleRule-example-childCount",
        R_TRIPLE_RULE_DEFAULT_SUBJECT,
    ),
    (
        "inference-rules/TripleRule-example-squares",
        R_SHAPE_CLASS_SUBJECT,
    ),
    ("inference-rules/global-symmetric", R_GLOBAL_RULES),
    ("inference-rules/same-order", R_GLOBAL_RULES),
    ("inference-rules/rdfs/rdfs-domain-1", R_GLOBAL_RULES),
    ("inference-rules/rdfs/rdfs-domain-2", R_GLOBAL_RULES),
    ("inference-rules/rdfs/rdfs-range-1", R_GLOBAL_RULES),
    ("inference-rules/rdfs/rdfs-range-2", R_GLOBAL_RULES),
    ("inference-rules/rdfs/rdfs-subclass-1", R_GLOBAL_RULES),
    ("inference-rules/rdfs/rdfs-subproperty-1", R_GLOBAL_RULES),
    ("inference-rules/layers-example", R_LAYER),
    ("inference-rules/run-once-example", R_SHAPE_CLASS_RUN_ONCE),
    ("inference-rules/run-once-blank-node-feed", R_RUN_ONCE),
    (
        "inference-rules/temp-triples-example",
        R_SHAPE_CLASS_RUN_ONCE,
    ),
    (
        "inference-rules/SPARQLRuleTemplate-example-Multiply",
        R_SHAPE_CLASS_TEMPLATE,
    ),
    (
        "inference-rules/SPARQLRuleTemplate-example-SymmetricProperty",
        R_RULE_TEMPLATE,
    ),
    (
        "inference-rules/SPARQLRuleTemplate-missing-param",
        R_RULE_TEMPLATE,
    ),
    (
        "inference-rules/ruleProcessor-unknown-at-rule",
        R_RULE_PROCESSOR,
    ),
    (
        "inference-rules/ruleProcessor-unknown-at-ruleset",
        R_RULE_PROCESSOR,
    ),
    (
        "inference-rules/expectedPredicate-example",
        R_EXPECTED_PREDICATE,
    ),
    // ── Node expressions ──
    ("node-expr/shnex-sparql/bound-example", R_UNBOUND_ARG),
    ("node-expr/shnex-sparql/coalesce-example", R_UNBOUND_ARG),
    ("node-expr/shnex/filterShape-integers", R_SEQUENCE_ORDER),
    (
        "node-expr/shnex/filterShape-unconstrained",
        R_SEQUENCE_ORDER,
    ),
    ("node-expr/shnex/distinct-list", R_SEQUENCE_ORDER),
    ("node-expr/shnex/distinct-termEquality", R_SEQUENCE_ORDER),
    ("node-expr/shnex/findFirst-empty", R_EMPTY_SHAPE),
    ("node-expr/shnex/matchAll-empty", R_EMPTY_SHAPE),
    ("node-expr/shnex/orderBy-height", R_ORDER_BY_UNBOUND),
    // ── SPARQL 1.2 RL (every srlt: test type) ──
    ("sparql-rl/eval/eval-basic-01", NO_SRL),
    ("sparql-rl/eval/eval-basic-02", NO_SRL),
    ("sparql-rl/eval/eval-basic-bnodes-pattern-1", NO_SRL),
    ("sparql-rl/eval/eval-basic-bnodes-pattern-2", NO_SRL),
    ("sparql-rl/eval/eval-basic-bnodes-pattern-3", NO_SRL),
    ("sparql-rl/eval/eval-basic-bnodes-pattern-4", NO_SRL),
    ("sparql-rl/eval/eval-data-01", NO_SRL),
    ("sparql-rl/eval/eval-data-02", NO_SRL),
    ("sparql-rl/eval/eval-bnodes-01", NO_SRL),
    ("sparql-rl/eval/eval-bnodes-02", NO_SRL),
    ("sparql-rl/eval/eval-bnodes-03", NO_SRL),
    ("sparql-rl/eval/eval-neg-01", NO_SRL),
    ("sparql-rl/eval/eval-rdfs-subclass-1", NO_SRL),
    ("sparql-rl/eval/eval-rdfs-subproperty-1", NO_SRL),
    ("sparql-rl/eval/eval-rdfs-domain-1", NO_SRL),
    ("sparql-rl/eval/eval-rdfs-domain-2", NO_SRL),
    ("sparql-rl/eval/eval-rdfs-range-1", NO_SRL),
    ("sparql-rl/eval/eval-rdfs-range-2", NO_SRL),
    ("sparql-rl/eval/eval-filter-01", NO_SRL),
    ("sparql-rl/eval/eval-filter-error-1", NO_SRL),
    ("sparql-rl/eval/eval-neg-02a", NO_SRL),
    ("sparql-rl/eval/eval-neg-02b", NO_SRL),
    ("sparql-rl/eval/eval-assign-01", NO_SRL),
    ("sparql-rl/eval/eval-assign-02", NO_SRL),
    ("sparql-rl/eval/eval-assign-03", NO_SRL),
    ("sparql-rl/eval/eval-assign-error-1", NO_SRL),
    ("sparql-rl/eval/eval-neg-data-01", NO_SRL),
    ("sparql-rl/eval/eval-neg-data-02", NO_SRL),
    ("sparql-rl/eval/eval-neg-data-03", NO_SRL),
    ("sparql-rl/eval/eval-neg-data-04", NO_SRL),
    ("sparql-rl/eval/eval-neg-data-05", NO_SRL),
    ("sparql-rl/eval/eval-neg-data-06", NO_SRL),
    ("sparql-rl/eval/eval-where-data-01", NO_SRL),
    ("sparql-rl/eval/eval-where-data-02", NO_SRL),
    ("sparql-rl/eval/eval-where-data-03", NO_SRL),
    ("sparql-rl/eval2/link-path-1", NO_SRL),
    ("sparql-rl/eval2/link-path-2", NO_SRL),
    ("sparql-rl/eval2/eval-dft-value-neg-01", NO_SRL),
    ("sparql-rl/eval2/eval-dft-value-neg-02", NO_SRL),
    ("sparql-rl/eval2/eval-dft-value-where-01", NO_SRL),
    ("sparql-rl/eval2/eval-dft-value-where-02", NO_SRL),
    ("sparql-rl/examples/example-1", NO_SRL),
    ("sparql-rl/examples/example-2", NO_SRL),
    ("sparql-rl/examples/example-3", NO_SRL),
    ("sparql-rl/examples/example-4", NO_SRL),
    ("sparql-rl/examples/example-5", NO_SRL),
    ("sparql-rl/stratification/manifest#test_1", NO_SRL),
    ("sparql-rl/stratification/manifest#test_2", NO_SRL),
    ("sparql-rl/stratification/manifest#test_3", NO_SRL),
    ("sparql-rl/stratification/manifest#test_4", NO_SRL),
    ("sparql-rl/stratification/manifest#test_5", NO_SRL),
    ("sparql-rl/stratification/manifest#test_6", NO_SRL),
    ("sparql-rl/stratification/manifest#test_7", NO_SRL),
    ("sparql-rl/stratification/manifest#test_8", NO_SRL),
    ("sparql-rl/stratification/manifest#test_9", NO_SRL),
    ("sparql-rl/stratification/manifest#test_10", NO_SRL),
    ("sparql-rl/syntax/manifest#test_1", NO_SRL),
    ("sparql-rl/syntax/manifest#test_2", NO_SRL),
    ("sparql-rl/syntax/manifest#test_3", NO_SRL),
    ("sparql-rl/syntax/manifest#test_4", NO_SRL),
    ("sparql-rl/syntax/manifest#test_5", NO_SRL),
    ("sparql-rl/syntax/manifest#test_6", NO_SRL),
    ("sparql-rl/syntax/manifest#test_7", NO_SRL),
    ("sparql-rl/syntax/manifest#test_8", NO_SRL),
    ("sparql-rl/syntax/manifest#test_9", NO_SRL),
    ("sparql-rl/syntax/manifest#test_10", NO_SRL),
    ("sparql-rl/syntax/manifest#test_11", NO_SRL),
    ("sparql-rl/syntax/manifest#test_12", NO_SRL),
    ("sparql-rl/syntax/manifest#test_13", NO_SRL),
    ("sparql-rl/syntax/manifest#test_14", NO_SRL),
    ("sparql-rl/syntax/manifest#test_15", NO_SRL),
    ("sparql-rl/syntax/manifest#test_16", NO_SRL),
    ("sparql-rl/syntax/manifest#test_17", NO_SRL),
    ("sparql-rl/syntax/manifest#test_18", NO_SRL),
    ("sparql-rl/syntax/manifest#test_19", NO_SRL),
    ("sparql-rl/syntax/manifest#test_20", NO_SRL),
    ("sparql-rl/syntax/manifest#test_21", NO_SRL),
    ("sparql-rl/syntax/manifest#test_22", NO_SRL),
    ("sparql-rl/syntax/manifest#test_23", NO_SRL),
    ("sparql-rl/syntax/manifest#test_24", NO_SRL),
    ("sparql-rl/syntax/manifest#test_25", NO_SRL),
    ("sparql-rl/syntax/manifest#test_26", NO_SRL),
    ("sparql-rl/syntax/manifest#test_27", NO_SRL),
    ("sparql-rl/syntax/manifest#test_28", NO_SRL),
    ("sparql-rl/syntax/manifest#test_29", NO_SRL),
    ("sparql-rl/syntax/manifest#test_30", NO_SRL),
    ("sparql-rl/syntax/manifest#test_31", NO_SRL),
    ("sparql-rl/syntax/manifest#test_32", NO_SRL),
    ("sparql-rl/syntax/manifest#test_33", NO_SRL),
    ("sparql-rl/syntax/manifest#test_34", NO_SRL),
    ("sparql-rl/syntax/manifest#test_35", NO_SRL),
    ("sparql-rl/syntax/manifest#test_36", NO_SRL),
    ("sparql-rl/syntax/manifest#test_37", NO_SRL),
    ("sparql-rl/syntax/manifest#test_38", NO_SRL),
    ("sparql-rl/syntax/manifest#test_39", NO_SRL),
    ("sparql-rl/syntax/manifest#test_40", NO_SRL),
    ("sparql-rl/syntax/manifest#test_41", NO_SRL),
    ("sparql-rl/syntax/manifest#test_42", NO_SRL),
    ("sparql-rl/syntax/manifest#test_43", NO_SRL),
    ("sparql-rl/syntax/manifest#test_44", NO_SRL),
    ("sparql-rl/syntax/manifest#test_45", NO_SRL),
    ("sparql-rl/syntax/manifest#test_46", NO_SRL),
    ("sparql-rl/syntax/manifest#test_47", NO_SRL),
    ("sparql-rl/syntax/manifest#test_48", NO_SRL),
    ("sparql-rl/syntax/manifest#test_49", NO_SRL),
    ("sparql-rl/syntax/manifest#test_50", NO_SRL),
    ("sparql-rl/syntax/manifest#test_51", NO_SRL),
    ("sparql-rl/syntax/manifest#test_52", NO_SRL),
    ("sparql-rl/syntax/manifest#test_53", NO_SRL),
    ("sparql-rl/syntax/manifest#test_54", NO_SRL),
    ("sparql-rl/syntax/manifest#test_55", NO_SRL),
    ("sparql-rl/syntax/manifest#test_56", NO_SRL),
    ("sparql-rl/syntax/manifest#test_57", NO_SRL),
    ("sparql-rl/syntax/manifest#test_58", NO_SRL),
    ("sparql-rl/syntax/manifest#test_59", NO_SRL),
    ("sparql-rl/syntax/manifest#test_60", NO_SRL),
    ("sparql-rl/syntax/manifest#test_61", NO_SRL),
    ("sparql-rl/syntax/manifest#test_62", NO_SRL),
    ("sparql-rl/syntax/manifest#test_63", NO_SRL),
    ("sparql-rl/syntax/manifest#test_64", NO_SRL),
    ("sparql-rl/syntax/manifest#test_65", NO_SRL),
    ("sparql-rl/syntax/manifest#test_66", NO_SRL),
    ("sparql-rl/syntax/manifest#test_67", NO_SRL),
    ("sparql-rl/syntax/manifest#test_68", NO_SRL),
    ("sparql-rl/syntax/manifest#test_69", NO_SRL),
    ("sparql-rl/syntax/manifest#test_70", NO_SRL),
    ("sparql-rl/syntax/manifest#test_71", NO_SRL),
    ("sparql-rl/syntax/manifest#test_72", NO_SRL),
    ("sparql-rl/syntax/manifest#test_73", NO_SRL),
    ("sparql-rl/syntax/manifest#test_74", NO_SRL),
    ("sparql-rl/syntax/manifest#test_75", NO_SRL),
    ("sparql-rl/syntax/manifest#test_76", NO_SRL),
    ("sparql-rl/syntax/manifest#test_77", NO_SRL),
    ("sparql-rl/syntax/manifest#test_78", NO_SRL),
    ("sparql-rl/syntax/manifest#test_79", NO_SRL),
    ("sparql-rl/syntax/manifest#test_80", NO_SRL),
    ("sparql-rl/syntax/manifest#test_81", NO_SRL),
    ("sparql-rl/syntax/manifest#test_82", NO_SRL),
    ("sparql-rl/syntax/manifest#test_83", NO_SRL),
    ("sparql-rl/syntax/manifest#test_84", NO_SRL),
    ("sparql-rl/syntax/manifest#test_85", NO_SRL),
    ("sparql-rl/syntax/manifest#test_86", NO_SRL),
    ("sparql-rl/syntax/manifest#test_87", NO_SRL),
    ("sparql-rl/syntax/manifest#test_88", NO_SRL),
    ("sparql-rl/syntax/manifest#test_89", NO_SRL),
    ("sparql-rl/syntax/manifest#test_90", NO_SRL),
    ("sparql-rl/syntax/manifest#test_91", NO_SRL),
    ("sparql-rl/syntax/manifest#test_92", NO_SRL),
    ("sparql-rl/syntax/manifest#test_93", NO_SRL),
    ("sparql-rl/syntax/manifest#test_94", NO_SRL),
    ("sparql-rl/syntax/manifest#test_95", NO_SRL),
    ("sparql-rl/syntax/manifest#test_96", NO_SRL),
    ("sparql-rl/syntax/manifest#test_97", NO_SRL),
    ("sparql-rl/syntax/manifest#test_98", NO_SRL),
    ("sparql-rl/syntax/manifest#test_99", NO_SRL),
    ("sparql-rl/syntax/manifest#test_100", NO_SRL),
    ("sparql-rl/syntax/manifest#test_101", NO_SRL),
    ("sparql-rl/syntax/manifest#test_102", NO_SRL),
    ("sparql-rl/syntax/manifest#test_103", NO_SRL),
    ("sparql-rl/syntax/manifest#test_104", NO_SRL),
    ("sparql-rl/syntax/manifest#test_105", NO_SRL),
    ("sparql-rl/syntax/manifest#test_106", NO_SRL),
    ("sparql-rl/syntax/manifest#test_107", NO_SRL),
    ("sparql-rl/syntax/manifest#test_108", NO_SRL),
    ("sparql-rl/syntax/manifest#test_109", NO_SRL),
    ("sparql-rl/syntax/manifest#test_110", NO_SRL),
    ("sparql-rl/syntax/manifest#test_111", NO_SRL),
    ("sparql-rl/syntax/manifest#test_112", NO_SRL),
    ("sparql-rl/syntax/manifest#test_113", NO_SRL),
    ("sparql-rl/syntax/manifest#test_114", NO_SRL),
    ("sparql-rl/syntax/manifest#test_115", NO_SRL),
    ("sparql-rl/syntax/manifest#test_116", NO_SRL),
    ("sparql-rl/syntax/manifest#test_117", NO_SRL),
    ("sparql-rl/syntax/manifest#test_118", NO_SRL),
    ("sparql-rl/syntax/manifest#test_119", NO_SRL),
    ("sparql-rl/syntax/manifest#test_120", NO_SRL),
    ("sparql-rl/syntax/manifest#test_121", NO_SRL),
    ("sparql-rl/syntax/manifest#test_122", NO_SRL),
    ("sparql-rl/syntax/manifest#test_123", NO_SRL),
    ("sparql-rl/syntax/manifest#test_124", NO_SRL),
    ("sparql-rl/syntax/manifest#test_125", NO_SRL),
    ("sparql-rl/syntax/manifest#test_126", NO_SRL),
    ("sparql-rl/syntax/manifest#test_127", NO_SRL),
    ("sparql-rl/syntax/manifest#test_128", NO_SRL),
    ("sparql-rl/syntax/manifest#test_129", NO_SRL),
    ("sparql-rl/syntax/manifest#test_130", NO_SRL),
    ("sparql-rl/syntax/manifest#test_131", NO_SRL),
    ("sparql-rl/syntax/manifest#test_132", NO_SRL),
    ("sparql-rl/syntax/manifest#test_133", NO_SRL),
    ("sparql-rl/syntax/manifest#test_134", NO_SRL),
    ("sparql-rl/syntax/manifest#test_135", NO_SRL),
    ("sparql-rl/syntax/manifest#test_136", NO_SRL),
    ("sparql-rl/syntax/manifest#test_137", NO_SRL),
    ("sparql-rl/syntax/manifest#test_138", NO_SRL),
    ("sparql-rl/syntax/manifest#test_139", NO_SRL),
    ("sparql-rl/wellformed/manifest#test_1", NO_SRL),
    ("sparql-rl/wellformed/manifest#test_2", NO_SRL),
    ("sparql-rl/wellformed/manifest#test_3", NO_SRL),
    ("sparql-rl/wellformed/manifest#test_4", NO_SRL),
    ("sparql-rl/wellformed/manifest#test_5", NO_SRL),
    ("sparql-rl/wellformed/manifest#test_6", NO_SRL),
    ("sparql-rl/wellformed/manifest#test_7", NO_SRL),
    ("sparql-rl/wellformed/manifest#test_8", NO_SRL),
];

// ── Canonical-form expectations ───────────────────────────────────────────────

/// Node-expression entries whose W3C expected literal is NOT in the canonical
/// lexical form XSD 1.1 Part 2 assigns its value: `(test id, expected lexical,
/// XSD 1.1 canonical lexical, canonical-mapping clause)`.
///
/// RDF 1.2 literal equality compares lexical forms, and PurRDF emits canonical
/// forms, so an expectation spelled non-canonically can never be met by a
/// canonical engine — and it is not relaxed by comparing VALUES instead, which
/// would equally accept a genuinely non-canonical engine output. Each entry
/// instead swaps the one expected lexical form for its canonical form and grades
/// the output against THAT, exactly. The table is checked in both directions:
/// [`non_canonical_expectations_are_really_non_canonical`] proves each expected
/// form is non-canonical and that the stated canonical form is what the XSD 1.1
/// canonical mapping produces, and the harness proves the engine's output equals
/// the canonical form term for term.
/// The clause every current entry cites. XSD 1.1 Part 2 §3.3.3.1 states that
/// "for integers, the decimal point and fractional part are prohibited" in the
/// canonical representation and that the mapping "is given formally in
/// decimalCanonicalMap", which §E.1 defines: "If d is an integer, then return
/// noDecimalPtCanonicalMap(d)". SPARQL's CEIL, FLOOR, ROUND, numeric division,
/// SECONDS and SUM over decimals all yield `xsd:decimal`.
const DECIMAL_CANONICAL_MAP: &str = "XSD 1.1 Part 2 §3.3.3.1 + §E.1 decimalCanonicalMap: \
     an integer-valued xsd:decimal maps through noDecimalPtCanonicalMap (no decimal point)";

const NON_CANONICAL_EXPECTATIONS: &[(&str, &str, &str, &str)] = &[
    (
        "node-expr/shnex-sparql/ceil-example",
        "4.0",
        "4",
        DECIMAL_CANONICAL_MAP,
    ),
    (
        "node-expr/shnex-sparql/floor-example",
        "3.0",
        "3",
        DECIMAL_CANONICAL_MAP,
    ),
    (
        "node-expr/shnex-sparql/round-example",
        "4.0",
        "4",
        DECIMAL_CANONICAL_MAP,
    ),
    (
        "node-expr/shnex-sparql/divide-example",
        "42.0",
        "42",
        DECIMAL_CANONICAL_MAP,
    ),
    (
        "node-expr/shnex-sparql/seconds-example",
        "00",
        "0",
        DECIMAL_CANONICAL_MAP,
    ),
    (
        "node-expr/shnex/sum-totalRevenue",
        "42.0",
        "42",
        DECIMAL_CANONICAL_MAP,
    ),
];

/// [`NON_CANONICAL_EXPECTATIONS`] pinned by count, so an entry cannot be added or
/// dropped without this number moving with it.
const NON_CANONICAL_EXPECTATIONS_COUNT: usize = 6;

/// Replace each expected literal an entry of [`NON_CANONICAL_EXPECTATIONS`]
/// names with its canonical form. An entry that matches no expected literal of
/// its test is a stale entry and an error.
fn canonical_expectation(id: &str, expected: &[Term]) -> Result<Vec<Term>, String> {
    let Some((_, lexical, canonical, _)) = NON_CANONICAL_EXPECTATIONS
        .iter()
        .find(|(entry, ..)| *entry == id)
    else {
        return Ok(expected.to_vec());
    };
    let mut replaced = 0usize;
    let out = expected
        .iter()
        .map(|term| match term {
            Term::Literal(l) if l.value() == *lexical && l.language().is_none() => {
                replaced += 1;
                Term::Literal(Literal::new_typed_literal(*canonical, l.datatype()))
            }
            other => other.clone(),
        })
        .collect();
    if replaced == 0 {
        return Err(format!(
            "NON_CANONICAL_EXPECTATIONS names {id} with expected lexical {lexical:?}, which \
             is not among its expected results — stale entry"
        ));
    }
    Ok(out)
}

// ── Expectation defects ───────────────────────────────────────────────────────

/// One compared result tuple, spelled as the comparison spells it (see
/// `shacl_corpora::norm`): an IRI in angle brackets, a literal or triple term in
/// N-Triples form, any blank node as `_:`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ResultTuple {
    focus: &'static str,
    path: Option<&'static str>,
    value: Option<&'static str>,
    component: &'static str,
    severity: &'static str,
}

impl ResultTuple {
    /// This tuple as a multiset key.
    fn key(self) -> Tuple {
        (
            self.focus.to_owned(),
            self.path.map(ToOwned::to_owned),
            self.value.map(ToOwned::to_owned),
            self.component.to_owned(),
            self.severity.to_owned(),
        )
    }
}

/// One amendment of one approved expected result: the approved report's
/// `approved` result is graded as `graded` instead. Every field the two do not
/// share IS the delta; every field they share is held.
#[derive(Clone, Copy, Debug)]
struct Amendment {
    approved: ResultTuple,
    graded: ResultTuple,
}

const REIFIER_SHAPE_COMPONENT: &str =
    "<http://www.w3.org/ns/shacl#ReifierShapeConstraintComponent>";
const VIOLATION: &str = "<http://www.w3.org/ns/shacl#Violation>";
const INVALID_RESOURCE_1: &str = "<http://example.com/ns#InvalidResource1>";
const PROPERTY_A: &str = "<http://example.com/ns#propertyA>";

/// The SHACL 1.2 Core §7.8.5 sentences the `sh:reifierShape` entries grade
/// against, quoted from the Working Draft and the editor's draft, which agree
/// word for word.
const REIFIER_SHAPE_CLAUSE: &str = "SHACL 1.2 Core §7.8.5 sh:reifierShape: \"Let t be the triple \
     term (focus node, $path, value node). […] For each reifier t that does not conform to \
     $reifierShape, there is a validation result with t as sh:value.\"";
const REIFICATION_REQUIRED_CLAUSE: &str = "SHACL 1.2 Core §7.8.5 sh:reificationRequired: \"If \
     $reificationRequired is set to true and there is no reified statement for the triple term \
     t in the data graph, there is a validation result with t as sh:value.\"";

/// Approved W3C SHACL 1.2 expectations that contradict normative SHACL 1.2 Core
/// text: `(test id, the normative sentence quoted with its section, the exact
/// delta)`.
///
/// An entry is not an expected failure. The harness grades the engine's report
/// against the approved expected report WITH the delta applied, and against
/// nothing else: a report that lacks the delta, carries a different one, or
/// differs anywhere else fails exactly as an unamended comparison would. Each
/// delta must apply to a result the approved report really contains, or the
/// entry is stale and the harness says so. [`expectation_defects_are_exact`]
/// proves both directions on the engine's real report, and pins the table by
/// count.
///
/// * `core/node/xone-003` — the result is produced by
///   `shsh:xoneSubjectsShapeXonePropertyShape`, a property shape whose `sh:path`
///   is `sh:xone`, through `sh:minListLength`, whose definition states no
///   exception to §6.7.2.2. The approved report omits `sh:resultPath`; the
///   quoted sentence makes it the shape's `sh:path`.
/// * `core/property/reifierShape-001` — `ex:InvalidResource1`'s statement has one
///   reifier (the annotation's blank node), and it fails `ex:ReifyShape`. The
///   approved report gives the VALUE NODE, `sh:value "invalid"`; the quoted
///   sentence's `t` is bound by "for each reifier t", so `sh:value` is the
///   reifier, a blank node (`_:` in the comparison).
/// * `core/property/reifierShape-002` — `ex:InvalidResource1`'s statement has no
///   reifier and `sh:reificationRequired true`. The approved report gives
///   `sh:value "invalid"`; the quoted sentence's `t` is the triple term
///   `<<( ex:InvalidResource1 ex:propertyA "invalid" )>>`, so that is `sh:value`.
const EXPECTATION_DEFECTS: &[(&str, &str, &[Amendment])] = &[
    (
        "core/node/xone-003",
        "SHACL 1.2 Core §6.7.2.2: \"For results produced by a property shape, this SHACL \
         property path is equivalent to the value of sh:path of the shape, unless stated \
         otherwise.\"",
        &[Amendment {
            approved: ResultTuple {
                focus: "<http://example.com/ns#TestXoneUnsatisfiableShape>",
                path: None,
                value: Some("<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>"),
                component: "<http://www.w3.org/ns/shacl#MinListLengthConstraintComponent>",
                severity: "<http://www.w3.org/ns/shacl#Warning>",
            },
            graded: ResultTuple {
                focus: "<http://example.com/ns#TestXoneUnsatisfiableShape>",
                path: Some("<http://www.w3.org/ns/shacl#xone>"),
                value: Some("<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>"),
                component: "<http://www.w3.org/ns/shacl#MinListLengthConstraintComponent>",
                severity: "<http://www.w3.org/ns/shacl#Warning>",
            },
        }],
    ),
    (
        "core/property/reifierShape-001",
        REIFIER_SHAPE_CLAUSE,
        &[Amendment {
            approved: ResultTuple {
                focus: INVALID_RESOURCE_1,
                path: Some(PROPERTY_A),
                value: Some("\"invalid\""),
                component: REIFIER_SHAPE_COMPONENT,
                severity: VIOLATION,
            },
            graded: ResultTuple {
                focus: INVALID_RESOURCE_1,
                path: Some(PROPERTY_A),
                value: Some("_:"),
                component: REIFIER_SHAPE_COMPONENT,
                severity: VIOLATION,
            },
        }],
    ),
    (
        "core/property/reifierShape-002",
        REIFICATION_REQUIRED_CLAUSE,
        &[Amendment {
            approved: ResultTuple {
                focus: INVALID_RESOURCE_1,
                path: Some(PROPERTY_A),
                value: Some("\"invalid\""),
                component: REIFIER_SHAPE_COMPONENT,
                severity: VIOLATION,
            },
            graded: ResultTuple {
                focus: INVALID_RESOURCE_1,
                path: Some(PROPERTY_A),
                value: Some(
                    "<<( <http://example.com/ns#InvalidResource1> \
                     <http://example.com/ns#propertyA> \"invalid\" )>>",
                ),
                component: REIFIER_SHAPE_COMPONENT,
                severity: VIOLATION,
            },
        }],
    ),
];

/// [`EXPECTATION_DEFECTS`] pinned by count, so an entry cannot be added or
/// dropped without this number moving with it.
const EXPECTATION_DEFECTS_COUNT: usize = 3;

/// The approved expected results of `id` with `deltas` applied. A delta whose
/// approved result the approved report does not contain is a stale entry and an
/// error, so an entry can never amend a result into existence.
fn amend(id: &str, results: &Multiset, deltas: &[Amendment]) -> Result<Multiset, String> {
    let mut amended = results.clone();
    for delta in deltas {
        let approved = delta.approved.key();
        match amended.get_mut(&approved) {
            Some(count) if *count > 1 => *count -= 1,
            Some(_) => {
                amended.remove(&approved);
            }
            None => {
                return Err(format!(
                    "EXPECTATION_DEFECTS amends {id} at {approved:?}, which is not among its \
                     approved expected results — stale entry"
                ));
            }
        }
        *amended.entry(delta.graded.key()).or_insert(0) += 1;
    }
    Ok(amended)
}

/// The expectation a validation case is graded against: the approved one, or the
/// approved one amended by its [`EXPECTATION_DEFECTS`] entry.
fn graded_expectation(id: &str, tc: &W3cCase) -> Result<Expected, String> {
    let Some((_, _, deltas)) = EXPECTATION_DEFECTS.iter().find(|(entry, ..)| *entry == id) else {
        return Ok(match &tc.expected {
            Expected::Failure => Expected::Failure,
            Expected::Report { conforms, results } => Expected::Report {
                conforms: *conforms,
                results: results.clone(),
            },
        });
    };
    let Expected::Report { conforms, results } = &tc.expected else {
        return Err(format!(
            "EXPECTATION_DEFECTS names {id}, whose approved result is sht:Failure — a result \
             delta cannot amend it"
        ));
    };
    Ok(Expected::Report {
        conforms: *conforms,
        results: amend(id, results, deltas)?,
    })
}

/// Grade one `sht:Validate` entry against its graded expectation.
fn run_validate(id: &str, tc: &W3cCase) -> Result<(), String> {
    grade_against(tc, &graded_expectation(id, tc)?)
}

// ── Term-level comparison ─────────────────────────────────────────────────────

/// Compare produced node-expression output against the expected list: exact RDF
/// 1.2 term equality, in order, or as a multiset under `ignore_order`.
fn compare_outputs(produced: &[Term], expected: &[Term], ignore_order: bool) -> Result<(), String> {
    let agree = if ignore_order {
        let mut p: Vec<&Term> = produced.iter().collect();
        let mut e: Vec<&Term> = expected.iter().collect();
        p.sort_by_key(ToString::to_string);
        e.sort_by_key(ToString::to_string);
        p == e
    } else {
        produced == expected
    };
    if agree {
        return Ok(());
    }
    let render = |terms: &[Term]| {
        terms
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    };
    Err(format!(
        "node-expression output mismatch{}:\n  produced ( {} )\n  expected ( {} )",
        if ignore_order { " (order ignored)" } else { "" },
        render(produced),
        render(expected)
    ))
}

// ── sht:EvalNodeExpr ──────────────────────────────────────────────────────────

/// The focus node of an entry that gives no `sht:focusNode` (see module docs).
const ABSENT_FOCUS: &str = "shacl12-harness-absent-focus-node";

/// Evaluate `expr` with `bindings` pushed as scope frames on top of `scope`.
fn eval_in_scope(
    data: &ShaclData,
    focus: &Term,
    expr: &NodeExpr,
    guard: &mut RecursionGuard,
    bindings: &[(String, Term)],
    scope: Scope<'_>,
) -> Result<Vec<Term>, String> {
    match bindings.split_first() {
        None => eval_node_expr_in_scope(data, focus, expr, guard, scope),
        Some(((name, value), rest)) => {
            let frame = Binding::new(name, value, scope);
            eval_in_scope(data, focus, expr, guard, rest, Scope::bound(&frame))
        }
    }
}

/// Parse and evaluate an entry's node expression through the library.
fn eval_node_expr_case(tc: &NodeExprCase) -> Result<Vec<Term>, String> {
    let text = fs::read_to_string(&tc.file)
        .map_err(|e| format!("cannot read {}: {e}", tc.file.display()))?;
    let doc_prefixes = text_ingest::extract_prefixes(&text);
    let (shapes, mut exprs) = shapes::from_dataset_with_node_expressions(
        &tc.dataset,
        &doc_prefixes,
        None,
        std::slice::from_ref(&tc.expr),
    )
    .map_err(|e| format!("shapes/node-expression parse error: {e}"))?;
    let expr = exprs
        .pop()
        .ok_or("the parser returned no expression for the one root it was given")?;

    let projected = engine::project_dataset(tc.dataset.as_ref())?;
    let data = ShaclData::new(Arc::clone(&projected), projected, None);
    let focus = match &tc.focus {
        Some(focus) => focus.clone(),
        None => {
            let absent = Term::blank(ABSENT_FOCUS);
            let mentioned = !native_quads(
                tc.dataset.as_ref(),
                Some(&absent),
                None,
                None,
                GraphFilter::AnyGraph,
            )
            .is_empty()
                || !native_quads(
                    tc.dataset.as_ref(),
                    None,
                    None,
                    Some(&absent),
                    GraphFilter::AnyGraph,
                )
                .is_empty();
            if mentioned {
                return Err(format!("the test graph mentions the harness's {absent}"));
            }
            absent
        }
    };

    let _function_scope =
        sparql::enter_function_scope(sparql::bind_in_current_env(&shapes.functions)?);
    let _aggregate_scope = sparql::enter_aggregate_scope(Arc::clone(&shapes.aggregates));
    let mut guard = RecursionGuard::new();
    eval_in_scope(&data, &focus, &expr, &mut guard, &tc.scope, Scope::EMPTY)
}

/// Grade one `sht:EvalNodeExpr` entry.
fn run_node_expr(id: &str, tc: &NodeExprCase) -> Result<(), String> {
    let produced = no_panic(|| eval_node_expr_case(tc))?;
    let expected = canonical_expectation(id, &tc.expected)?;
    compare_outputs(&produced, &expected, tc.ignore_order)
}

// ── sht:Infer ─────────────────────────────────────────────────────────────────

/// Run the rules; `Ok((inferred graph, projected base))`.
fn infer(tc: &InferCase) -> Result<(Arc<RdfDataset>, Arc<RdfDataset>), String> {
    let shapes_text = fs::read_to_string(&tc.shapes_path)
        .map_err(|e| format!("cannot read shapes {}: {e}", tc.shapes_path.display()))?;
    let shapes_dataset = purrdf::parse_dataset(
        shapes_text.as_bytes(),
        "text/turtle",
        Some(&file_iri(&tc.shapes_path)),
    )
    .map_err(|e| format!("shapes graph parse error: {e}"))?;
    let doc_prefixes = text_ingest::extract_prefixes(&shapes_text);
    let shapes = shapes::from_dataset_with_config_and_graph(
        &shapes_dataset,
        &doc_prefixes,
        None,
        Some(tc.shapes_graph_iri.clone()),
    )
    .map_err(|e| format!("shapes parse error: {e}"))?;
    let data_dataset = if tc.data_path == tc.shapes_path {
        shapes_dataset
    } else {
        parse_turtle_file(&tc.data_path).map_err(|e| format!("data graph parse error: {e}"))?
    };
    let projected = engine::project_dataset(data_dataset.as_ref())?;
    let holder = ShaclData::new(Arc::clone(&projected), Arc::clone(&projected), None);
    let inferred = apply_rules(&holder, &shapes).map_err(|e| format!("apply_rules failed: {e}"))?;
    Ok((inferred, projected))
}

/// `base ⊎ derived`, blank labels standardized apart per source.
fn merge(base: &RdfDataset, derived: &RdfDataset) -> Result<Arc<RdfDataset>, String> {
    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(base);
    builder.push_dataset(derived);
    builder.freeze().map_err(|e| e.to_string())
}

/// The expected inferred triples as a dataset of their own.
fn expected_triples(expected: &InferExpected) -> Result<Arc<RdfDataset>, String> {
    match expected {
        InferExpected::Failure => Err("an sht:Failure entry has no expected triples".to_owned()),
        InferExpected::File(path) => parse_turtle_file(path),
        InferExpected::Triples(triples) => {
            let mut builder = RdfDatasetBuilder::new();
            for [s, p, o] in triples {
                let Term::NamedNode(p) = p else {
                    return Err(format!("expected triple has a non-IRI predicate {p}"));
                };
                builder.push_owned_quad(&RdfQuad::new(
                    s.to_rdf_term(),
                    p.as_str(),
                    o.to_rdf_term(),
                ));
            }
            builder.freeze().map_err(|e| e.to_string())
        }
    }
}

/// Grade one `sht:Infer` entry.
fn run_infer(tc: &InferCase) -> Result<(), String> {
    let outcome = no_panic(|| infer(tc));
    if matches!(tc.expected, InferExpected::Failure) {
        return match outcome {
            Err(_) => Ok(()),
            Ok(_) => Err("suite expects sht:Failure but the rules ran successfully".to_owned()),
        };
    }
    let (inferred, base) = outcome?;
    let expected_full = merge(base.as_ref(), expected_triples(&tc.expected)?.as_ref())?;
    let produced = canonicalize(inferred.as_ref()).nquads;
    let expected = canonicalize(expected_full.as_ref()).nquads;
    if produced == expected {
        return Ok(());
    }
    let produced_lines: Vec<&str> = produced.lines().collect();
    let expected_lines: Vec<&str> = expected.lines().collect();
    let missing: Vec<&str> = expected_lines
        .iter()
        .filter(|l| !produced_lines.contains(l))
        .copied()
        .collect();
    let extra: Vec<&str> = produced_lines
        .iter()
        .filter(|l| !expected_lines.contains(l))
        .copied()
        .collect();
    Err(format!(
        "inferred graph differs from data ⊎ expected (RDFC-1.0):\n  missing: {missing:?}\n  \
         extra: {extra:?}"
    ))
}

// ── srlt:* ────────────────────────────────────────────────────────────────────

/// Grade one SPARQL 1.2 RL entry.
///
/// PurRDF has no SPARQL 1.2 RL implementation, so there is no parser to accept
/// or reject a rule set and no evaluator to infer a graph: every entry — the
/// NEGATIVE syntax, well-formedness and stratification tests included — is a
/// failure. A negative test must not pass merely because nothing accepted its
/// input; it passes when an implementation REJECTS it.
fn run_srl(tc: &SrlCase) -> Result<(), String> {
    let ruleset = fs::read_to_string(&tc.ruleset)
        .map_err(|e| format!("cannot read {}: {e}", tc.ruleset.display()))?;
    Err(format!(
        "{NO_SRL}: the {}-byte rule set {} ({}) is not {}",
        ruleset.len(),
        tc.ruleset.display(),
        tc.kind.local_name(),
        if tc.data.is_some() || tc.result.is_some() {
            "evaluated"
        } else {
            "parsed"
        }
    ))
}

// ── The harness ───────────────────────────────────────────────────────────────

/// Grade one entry.
fn run(case: &Case12) -> Result<(), String> {
    match &case.body {
        Body::Validate(tc) => run_validate(&case.id, tc),
        Body::NodeExpr(tc) => run_node_expr(&case.id, tc),
        Body::Infer(tc) => run_infer(tc),
        Body::Srl(tc) => run_srl(tc),
    }
}

#[test]
fn w3c_shacl12_conformance() {
    let cases = shacl12_cases();
    assert_eq!(
        cases.len(),
        W3C12_TOTAL_CASES,
        "discovered SHACL 1.2 test count"
    );

    let xfail: BTreeMap<&str, &str> = XFAIL.iter().copied().collect();
    assert_eq!(
        xfail.len(),
        XFAIL.len(),
        "duplicate entries in the XFAIL ledger"
    );
    for (id, _) in XFAIL {
        assert!(
            cases.iter().any(|c| c.id == *id),
            "XFAIL ledger names unknown test {id} — stale entry?"
        );
    }

    let mut errors: Vec<String> = Vec::new();
    // (passed, xfailed), keyed by section and by test type.
    let mut sections: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut types: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut total_passed = 0usize;
    let mut total_xfailed = 0usize;

    // Engine panics are caught by `no_panic` and graded as failures; the default
    // hook's backtraces would only drown the scoreboard.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    for case in &cases {
        let section = sections.entry(case.section.as_str()).or_insert((0, 0));
        let ty = types.entry(case.type_label()).or_insert((0, 0));
        match (run(case), xfail.get(case.id.as_str())) {
            (Ok(()), None) => {
                section.0 += 1;
                ty.0 += 1;
                total_passed += 1;
            }
            (Err(_), Some(_)) => {
                section.1 += 1;
                ty.1 += 1;
                total_xfailed += 1;
            }
            (Ok(()), Some(reason)) => errors.push(format!(
                "XPASS [{id}]: now passes — remove it from the XFAIL ledger (reason was: {reason})",
                id = case.id
            )),
            (Err(e), None) => errors.push(format!("FAIL [{id}]: {e}", id = case.id)),
        }
    }

    std::panic::set_hook(default_hook);

    println!(
        "W3C SHACL 1.2 conformance scoreboard ({} tests):",
        cases.len()
    );
    for (section, (passed, xfailed)) in &sections {
        println!("  {section:<36} passed {passed:>3}  xfailed {xfailed:>3}");
    }
    println!("  by test type:");
    for (ty, (passed, xfailed)) in &types {
        println!("  {ty:<36} passed {passed:>3}  xfailed {xfailed:>3}");
    }
    println!(
        "  W3C12 TOTAL: passed {total_passed}, xfailed {total_xfailed}, ledger {}",
        XFAIL.len()
    );

    assert!(
        errors.is_empty(),
        "w3c_shacl12_conformance: {} error(s):\n{}",
        errors.len(),
        errors.join("\n\n")
    );
    assert_eq!(
        total_xfailed,
        XFAIL.len(),
        "xfail count must match the ledger exactly"
    );
    assert_eq!(
        total_passed + total_xfailed,
        W3C12_TOTAL_CASES,
        "every discovered test must be a pass or a ledgered xfail"
    );
}

// ── The canonical-form table ──────────────────────────────────────────────────

/// Both directions of [`NON_CANONICAL_EXPECTATIONS`], plus its count pin.
///
/// For every entry: the named test exists and is a node-expression test; its
/// expected results contain a literal with the stated lexical form; the XSD 1.1
/// canonical mapping of that literal's value (`purrdf_xsd`) is EXACTLY the stated
/// canonical form; and that canonical form DIFFERS from the expected one — an
/// expectation that was already canonical has no business in the table. The
/// engine side (its output equals the canonical form, term for term) is graded
/// by the harness itself.
#[test]
fn non_canonical_expectations_are_really_non_canonical() {
    assert_eq!(
        NON_CANONICAL_EXPECTATIONS.len(),
        NON_CANONICAL_EXPECTATIONS_COUNT,
        "NON_CANONICAL_EXPECTATIONS count pin"
    );
    let cases = shacl12_cases();
    for (id, lexical, canonical, clause) in NON_CANONICAL_EXPECTATIONS {
        assert!(
            clause.starts_with("XSD 1.1 Part 2 §"),
            "{id}: the clause must cite XSD 1.1 Part 2, got {clause:?}"
        );
        assert_ne!(lexical, canonical, "{id}: an entry must change the form");
        let case = cases
            .iter()
            .find(|c| c.id == *id)
            .unwrap_or_else(|| panic!("{id}: no such test"));
        let Body::NodeExpr(tc) = &case.body else {
            panic!("{id}: not an sht:EvalNodeExpr test");
        };
        let literal = tc
            .expected
            .iter()
            .find_map(|t| match t {
                Term::Literal(l) if l.value() == *lexical => Some(l),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{id}: no expected literal spelled {lexical:?}"));
        let value = purrdf_xsd::parse_by_iri(literal.value(), literal.datatype_str())
            .unwrap_or_else(|e| panic!("{id}: {lexical:?} is not a valid lexical form: {e}"))
            .unwrap_or_else(|| panic!("{id}: <{}> is not an XSD datatype", literal.datatype_str()));
        assert_eq!(
            value.canonical_lexical(),
            *canonical,
            "{id}: the XSD 1.1 canonical form of {lexical:?}^^<{}>",
            literal.datatype_str()
        );
    }
}

// ── The expectation-defect table ──────────────────────────────────────────────

/// Both directions of [`EXPECTATION_DEFECTS`], plus its count pin.
///
/// For every entry: the test exists, is an `sht:Validate` entry with an
/// approved expected report, is not also ledgered in [`XFAIL`], and the clause
/// quotes SHACL 1.2 Core. Then, on the ENGINE'S REAL REPORT:
///
/// * it agrees with the amended expectation (the delta is exactly what the
///   engine does differently);
/// * it disagrees with the approved expectation (the delta is not vacuous);
///
/// and, as controls over the comparison itself, with the engine's `sh:conforms`
/// held fixed:
///
/// * a report MISSING the delta — the approved results verbatim — fails;
/// * a report carrying a DIFFERENT delta — the same result, every field the
///   delta changes given another value — fails;
/// * a report carrying the delta plus a difference in any field the delta holds
///   (focus node, path, value, component or severity, one at a time) fails;
/// * a report carrying the delta PLUS an extra, different amendment of another
///   result fails.
#[test]
fn expectation_defects_are_exact() {
    assert_eq!(
        EXPECTATION_DEFECTS.len(),
        EXPECTATION_DEFECTS_COUNT,
        "EXPECTATION_DEFECTS count pin"
    );
    let cases = shacl12_cases();
    for (id, clause, deltas) in EXPECTATION_DEFECTS {
        assert!(
            clause.starts_with("SHACL 1.2 Core §") && clause.contains('"'),
            "{id}: the clause must quote SHACL 1.2 Core, got {clause:?}"
        );
        assert!(!deltas.is_empty(), "{id}: an entry must carry a delta");
        assert!(
            !XFAIL.iter().any(|(entry, _)| entry == id),
            "{id}: an expectation defect is graded, never also an expected failure"
        );
        let case = cases
            .iter()
            .find(|c| c.id == *id)
            .unwrap_or_else(|| panic!("{id}: no such test"));
        let Body::Validate(tc) = &case.body else {
            panic!("{id}: not an sht:Validate test");
        };
        let Expected::Report { conforms, results } = &tc.expected else {
            panic!("{id}: the approved result is sht:Failure");
        };
        let amended = amend(id, results, deltas).unwrap_or_else(|e| panic!("{e}"));
        assert_ne!(
            &amended, results,
            "{id}: the delta must change the expectation"
        );

        let (produced_conforms, produced) =
            produce(tc).unwrap_or_else(|e| panic!("{id}: the engine must validate: {e}"));
        let amended_expectation = Expected::Report {
            conforms: *conforms,
            results: amended.clone(),
        };
        let approved_expectation = Expected::Report {
            conforms: *conforms,
            results: results.clone(),
        };
        grade(
            &amended_expectation,
            Ok((produced_conforms, produced.clone())),
        )
        .unwrap_or_else(|e| panic!("{id}: the engine must equal the amended report: {e}"));
        assert!(
            grade(&approved_expectation, Ok((produced_conforms, produced))).is_err(),
            "{id}: the engine must differ from the approved report, or the entry is vacuous"
        );

        // A report missing the delta.
        assert!(
            grade(
                &amended_expectation,
                Ok((produced_conforms, results.clone()))
            )
            .is_err(),
            "{id}: a report missing the delta must fail"
        );

        // A report with a different delta in its place: every field the delta
        // changes carries another value instead.
        let differently: Vec<Amendment> = deltas
            .iter()
            .map(|delta| Amendment {
                graded: other_in_delta_fields(delta),
                ..*delta
            })
            .collect();
        let differently_amended = amend(id, results, &differently).expect("the same result");
        assert!(
            grade(
                &amended_expectation,
                Ok((produced_conforms, differently_amended))
            )
            .is_err(),
            "{id}: a report carrying a different delta must fail"
        );

        // A report with the delta AND a difference in a field the delta holds:
        // the entry licenses its delta and nothing else about that result.
        for (field, held) in deltas.iter().flat_map(|delta| {
            held_field_variants(delta)
                .into_iter()
                .map(move |(field, graded)| (field, Amendment { graded, ..*delta }))
        }) {
            let widened = amend(id, results, &[held]).expect("the same result");
            assert!(
                grade(&amended_expectation, Ok((produced_conforms, widened))).is_err(),
                "{id}: a report carrying the delta plus a different {field} must fail"
            );
        }

        // A report with the delta plus an extra, different result.
        let mut extra = amended;
        *extra
            .entry((
                "<http://example.org/ns#extra-focus>".to_owned(),
                Some("<http://example.org/ns#extra-path>".to_owned()),
                None,
                "<http://www.w3.org/ns/shacl#MinCountConstraintComponent>".to_owned(),
                "<http://www.w3.org/ns/shacl#Violation>".to_owned(),
            ))
            .or_insert(0) += 1;
        assert!(
            grade(&amended_expectation, Ok((produced_conforms, extra))).is_err(),
            "{id}: a report carrying the delta plus another difference must fail"
        );
    }
}

/// A value no vendored report carries, standing in for "some other term".
const ELSEWHERE: &str = "<http://example.org/ns#elsewhere>";

/// `delta`'s graded tuple with every field the delta CHANGES replaced by
/// [`ELSEWHERE`]: the same result, amended differently.
fn other_in_delta_fields(delta: &Amendment) -> ResultTuple {
    let (approved, graded) = (delta.approved, delta.graded);
    ResultTuple {
        focus: if approved.focus == graded.focus {
            graded.focus
        } else {
            ELSEWHERE
        },
        path: if approved.path == graded.path {
            graded.path
        } else {
            Some(ELSEWHERE)
        },
        value: if approved.value == graded.value {
            graded.value
        } else {
            Some(ELSEWHERE)
        },
        component: if approved.component == graded.component {
            graded.component
        } else {
            ELSEWHERE
        },
        severity: if approved.severity == graded.severity {
            graded.severity
        } else {
            ELSEWHERE
        },
    }
}

/// One variant of `delta`'s graded tuple per field the delta HOLDS, that field
/// replaced by [`ELSEWHERE`], named for the assertion message.
fn held_field_variants(delta: &Amendment) -> Vec<(&'static str, ResultTuple)> {
    let (approved, graded) = (delta.approved, delta.graded);
    let mut variants = Vec::new();
    if approved.focus == graded.focus {
        variants.push((
            "focus node",
            ResultTuple {
                focus: ELSEWHERE,
                ..graded
            },
        ));
    }
    if approved.path == graded.path {
        variants.push((
            "result path",
            ResultTuple {
                path: Some(ELSEWHERE),
                ..graded
            },
        ));
    }
    if approved.value == graded.value {
        variants.push((
            "value",
            ResultTuple {
                value: Some(ELSEWHERE),
                ..graded
            },
        ));
    }
    if approved.component == graded.component {
        variants.push((
            "component",
            ResultTuple {
                component: ELSEWHERE,
                ..graded
            },
        ));
    }
    if approved.severity == graded.severity {
        variants.push((
            "severity",
            ResultTuple {
                severity: ELSEWHERE,
                ..graded
            },
        ));
    }
    variants
}

/// A delta naming a result the approved report does not hold is refused as a
/// stale entry rather than amending a result into existence; the neighbouring
/// delta that names a real result applies.
#[test]
fn a_stale_expectation_defect_is_refused() {
    let focus = "<http://example.org/ns#focus>";
    let component = "<http://www.w3.org/ns/shacl#MinListLengthConstraintComponent>";
    let severity = "<http://www.w3.org/ns/shacl#Violation>";
    let approved = ResultTuple {
        focus,
        path: None,
        value: None,
        component,
        severity,
    };
    let mut results = Multiset::new();
    results.insert(approved.key(), 1);
    let real = Amendment {
        approved,
        graded: ResultTuple {
            path: Some("<http://example.org/ns#p>"),
            ..approved
        },
    };
    let stale = Amendment {
        approved: ResultTuple {
            focus: ELSEWHERE,
            ..approved
        },
        ..real
    };
    let amended = amend("control", &results, &[real]).expect("a real result is amended");
    assert_eq!(
        amended.keys().next().and_then(|tuple| tuple.1.as_deref()),
        Some("<http://example.org/ns#p>")
    );
    let error = amend("control", &results, &[stale]).expect_err("a stale delta is refused");
    assert!(error.contains("stale entry"), "{error}");
}

// ── Comparator controls ───────────────────────────────────────────────────────

fn xsd(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("http://www.w3.org/2001/XMLSchema#{local}"))
}

fn typed(lexical: &str, datatype: &str) -> Term {
    Term::Literal(Literal::new_typed_literal(lexical, xsd(datatype)))
}

/// The comparator is RDF 1.2 term equality: the same term agrees, a different
/// value disagrees, the same lexical form under another datatype disagrees, and
/// the same value under another lexical form disagrees.
#[test]
fn comparator_is_exact_term_equality() {
    let four = typed("4", "decimal");
    assert!(
        compare_outputs(
            std::slice::from_ref(&four),
            std::slice::from_ref(&four),
            false
        )
        .is_ok()
    );
    assert!(
        compare_outputs(&[typed("5", "decimal")], std::slice::from_ref(&four), false).is_err(),
        "a different value must not agree"
    );
    assert!(
        compare_outputs(&[typed("4", "integer")], std::slice::from_ref(&four), false).is_err(),
        "a different datatype must not agree"
    );
    assert!(
        compare_outputs(&[typed("4.0", "decimal")], &[four], false).is_err(),
        "a different lexical form of the same value must not agree"
    );
}

/// Order matters unless the entry says otherwise, and cardinality always does.
#[test]
fn comparator_order_and_cardinality() {
    let (a, b) = (typed("1", "integer"), typed("2", "integer"));
    assert!(compare_outputs(&[b.clone(), a.clone()], &[a.clone(), b.clone()], false).is_err());
    assert!(compare_outputs(&[b.clone(), a.clone()], &[a.clone(), b.clone()], true).is_ok());
    assert!(
        compare_outputs(&[a.clone(), a.clone(), b.clone()], &[a, b], true).is_err(),
        "ignoring order must not ignore cardinality"
    );
}

/// The canonical-form substitution touches only a test the table names: every
/// other test keeps its expectation exactly as the suite spells it.
#[test]
fn canonical_substitution_is_scoped_to_the_table() {
    let spelled = [typed("4.0", "decimal")];
    let kept = canonical_expectation("node-expr/not-in-the-table", &spelled)
        .expect("an unlisted test passes through");
    assert_eq!(kept, spelled);
    for (id, lexical, canonical, _) in NON_CANONICAL_EXPECTATIONS {
        let expected = [typed(lexical, "decimal"), typed("7", "integer")];
        let swapped = canonical_expectation(id, &expected).expect("a listed test substitutes");
        assert_eq!(
            swapped,
            [typed(canonical, "decimal"), typed("7", "integer")]
        );
        assert!(
            canonical_expectation(id, &[typed("7", "integer")]).is_err(),
            "{id}: an entry matching no expected literal is stale"
        );
    }
}

/// PurRDF's own numeric output is the XSD 1.1 canonical lexical form: the
/// `xsd:decimal` value four is spelled `"4"`, not `"4.0"` (XSD 1.1 Part 2
/// §3.3.3.1 and §E.1 `decimalCanonicalMap`). Evaluated through the same library
/// path the harness grades with, on an `example.org` fixture.
#[test]
fn engine_emits_the_canonical_decimal_lexical_form() {
    let ttl = r"
        @prefix ex: <http://example.org/ns#> .
        @prefix sparql: <http://www.w3.org/ns/sparql#> .
        ex:root ex:expr [ sparql:ceil ( 3.2 ) ] .
    ";
    let dataset = purrdf::parse_dataset(ttl.as_bytes(), "text/turtle", None).expect("parse");
    let root = native_quads(
        dataset.as_ref(),
        None,
        Some(&Term::NamedNode(NamedNode::new_unchecked(
            "http://example.org/ns#expr",
        ))),
        None,
        GraphFilter::AnyGraph,
    )
    .into_iter()
    .map(|(_, _, o)| o)
    .next()
    .expect("the fixture's expression node");
    let (shapes, exprs) = shapes::from_dataset_with_node_expressions(
        &dataset,
        &[],
        None,
        std::slice::from_ref(&root),
    )
    .expect("the expression parses");
    let projected = engine::project_dataset(dataset.as_ref()).expect("projection");
    let data = ShaclData::new(Arc::clone(&projected), projected, None);
    let _function_scope =
        sparql::enter_function_scope(sparql::bind_in_current_env(&shapes.functions).expect("bind"));
    let mut guard = RecursionGuard::new();
    let out = eval_node_expr_in_scope(
        &data,
        &Term::blank(ABSENT_FOCUS),
        &exprs[0],
        &mut guard,
        Scope::EMPTY,
    )
    .expect("evaluates");
    assert_eq!(out, [typed("4", "decimal")]);
}
