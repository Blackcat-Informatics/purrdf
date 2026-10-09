# Issue #401: Update transfers preserve RDF 1.2 identity: per-document LOAD blank scoping and typed record transfer

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github
Labels: enhancement

## Body

## Problem
1. `LOAD` merges blank nodes across independent documents. Parsers put every blank node in the default blank scope, and `load()` inserts the values without remapping, so two LOADs that both contain `_:b0` share one node. RDF semantics requires each document's blank nodes to be kept apart, including those inside nested triple terms.
2. ADD/COPY/MOVE move records through a projection that flattens the three RDF 1.2 statement surfaces (ordinary, reifier, annotation) and then reclassifies them on insert, so a transfer can lose or change a record's role.

## Proposed solution
- Give each loaded document its own blank-node remapping, reusing the existing blank-node counter and handling nested triple terms and CDT-embedded blank nodes.
- Make transfers typed: add record kinds/values so a reifier or annotation record stays a reifier or annotation. Public API changes must be purely additive.

## Acceptance
Failing-first Rust tests: two LOADs of `_:b0` stay distinct; nested-term blank nodes stay consistent within a document and separate across documents; reifier and annotation roles survive ADD, COPY and MOVE in both graph-existence modes. `cargo semver-checks`, `make check` and `make wasm` pass.


## Comments (0)

