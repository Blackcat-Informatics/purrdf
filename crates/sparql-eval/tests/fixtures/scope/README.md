<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Portable binding-scope cases

`inventory.json` is the executed inventory. Each evaluation entry supplies its query, RDF input files, independently enumerated SPARQL Results JSON bag, semantic categories and specification rationale. `manifest.ttl` exposes the SPARQL 1.1 cases in the W3C query-test manifest vocabulary. The RDF 1.2 triple-term case is separately labelled in the inventory.

The default data has two `p` witnesses and one `q` witness from `s`; each reaches `o` through `r`. The unrelated `wrong` edge reaches `other` and makes a lost witness connection observable. Expected bags count derivations by hand and retain duplicate and zero-column rows. Named-graph data is loaded into its declared graph, never unioned into the default graph.

These are SPARQL query cases, not SHACL-SPARQL constraints. In particular MINUS, VALUES and the unprojected focus-name subquery are not evidence of SHACL admission. SHACL restrictions are exercised through the shapes loader. Property functions and concrete blank prebindings are tested separately as PurRDF host invariants. No official corpus is changed by this fixture set.

A checked carrier with no visible source variables has one fresh transport unit column whose cell is the integer `1`. The harness verifies that column and every cell explicitly, then restores the original zero-column schema while retaining every row; the evaluator-only SERVICE cases also verify the adapter performs that restoration. Reparsing can canonicalize the integer spelling before subsequent renderings stabilize. Nonempty source schemas are compared exactly before and after carrier execution.
