<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# SHACL

[`purrdf-shapes`](https://docs.rs/purrdf-shapes) (re-exported as
`purrdf::shapes`) is PurRDF's native SHACL validator: the **complete SHACL
Core feature set** — all constraint components, full property paths,
qualified value shapes, property pairs — plus **SHACL-SPARQL** constraints
and targets and the **SHACL-AF** surface, running entirely on PurRDF's own
interned IR and native SPARQL engine (no oxigraph, no PyO3).

It validates an RDF 1.2 data graph against a SHACL shapes graph with **no
inference** (parity with pySHACL `inference="none"`); combine with
[Entailment](../entailment.md) if you want to validate a materialized closure.

## What it covers

- **SHACL Core** — every constraint component, full property paths, qualified
  value shapes, property pairs. The W3C `data-shapes` suite passes clean
  (126/126, zero ledgered gaps at the time of writing — the live number is in
  [`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md)).
- **SHACL-SPARQL** — SPARQL-based constraints and targets, custom constraint
  components with pre-binding semantics, user-defined `sh:SPARQLFunction`
  calls, and `sh:SPARQLTargetType`, evaluated on the native SPARQL engine.
- **SHACL-AF** — node expressions (including
  `sh:ExpressionConstraintComponent`) and **SHACL Rules** (`sh:TripleRule` and
  `sh:SPARQLRule`, with `sh:condition`, `sh:order`, `sh:deactivated`): rules
  fire in an iterative fixpoint and the derivation is materialized as a new
  dataset (`base ⊎ derived`), leaving the input graph untouched. Some
  node-expression conveniences (`sh:if`, aggregations, ordering wrappers) are
  DASH/TopBraid conventions with no normative RDF definition; PurRDF documents
  its adopted reading and pins it with a frozen corpus — see the
  [SHACL-AF section of docs/CONFORMANCE.md](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md#shacl-af-node-expressions-normative-surface-vs-owned-extensions).

## The SHACL 1.2 reifier-shape draft scope

The crate implements a **scoped** SHACL 1.2 Working Draft feature:
`sh:reifierShape` and `sh:reificationRequired` for direct IRI property paths,
so shapes can constrain the RDF 1.2 reifier metadata attached to statements
(see [RDF 1.2 Features](../concepts/rdf12.md)). The relevant SHACL 1.2 Core
draft is dated 2026-06-02. **This is not a claim of full SHACL 1.2
conformance** — it is one draft feature, explicitly scoped and tested.

## From Python

```python
from purrdf_native import shacl

report = shacl.validate(shapes_ttl="...", data_nt="...")
print(report["conforms"])  # True / False
print(report["results"])   # list of violation dicts
```

Each result dict keeps the stable keys `focus`, `path`, `value`, `severity`,
`component`, `source_shape`, and `message`.

## SARIF output

Validation reports stay structured in the engine; the SARIF 2.1.0 boundary is
the separate [`purrdf-validate`](https://docs.rs/purrdf-validate) crate
(`purrdf::validate`), which renders a report — or parser diagnostics — as a
source-traced, byte-deterministic SARIF log for editors, CI, and
code-scanning dashboards:

```rust,ignore
use purrdf::validate::{validate_to_sarif_string, SarifOptions};

let shapes = r#"
    @prefix sh:  <http://www.w3.org/ns/shacl#> .
    @prefix ex:  <http://example.org/> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
    ex:PersonShape a sh:NodeShape ;
      sh:targetClass ex:Person ;
      sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .
"#;
let data = r#"<http://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .
<http://example.org/alice> <http://example.org/age> "nope" .
"#;

let sarif = validate_to_sarif_string(shapes, data, &SarifOptions::default())
    .expect("sarif produced");
assert!(sarif.contains("\"version\": \"2.1.0\""));
```

Lower-level entry points (`build_report_sarif`, `build_diagnostics_sarif`)
build a `SarifLog` value instead of a string, so a host can merge runs before
serializing.

## Conformance

The validator is gated by the vendored W3C `data-shapes` suite, a vendored
DASH SHACL-AF/rules corpus, and a first-party frozen corpus of 69 cases with
byte-frozen expected reports; SHACL Rules output is compared to expected
inferred graphs by RDFC-1.0 isomorphism. See
[Conformance & Testing](../project/conformance.md).
