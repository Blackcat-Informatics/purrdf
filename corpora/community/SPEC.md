<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Portable corpus contract, version 1

## Authority and applicability

The dated specifications govern each expectation. This document governs corpus
acquisition, evidence and comparison. Passing one case establishes that case's
declared proposition on the named surface; it does not establish support for every
construct in a specification.

| Profile identity | Controlling language contract |
| --- | --- |
| `shacl-20170720` | [SHACL Recommendation, 20 July 2017](https://www.w3.org/TR/2017/REC-shacl-20170720/), including its prebinding restrictions and report mapping |
| `shacl12-20260918` | [SHACL 1.2 SPARQL Extensions Working Draft, 18 September 2026](https://www.w3.org/TR/2026/WD-shacl12-sparql-20260918/), with explicitly declared processor policies |

SHACL-AF cases additionally name the [8 June 2017 SHACL-AF Note](https://www.w3.org/TR/2017/NOTE-shacl-af-20170608/).
Neither dated SHACL identity switches the entire underlying RDF/SPARQL kernel to
another grammar. It selects SHACL admission, role prebinding and report behavior.
In particular, Recommendation restrictions on all VALUES clauses and nested
prebound projections cannot be silently imposed on the draft. Draft SERVICE
refusal is an explicitly identified processor policy; an external implementation
that allows it has a different policy, recorded as such.

## Acquisition

The catalog references each authoritative inventory once and identifies the
manifest root. Acquisition follows RDF `mf:include` and `mf:entries`
structurally, preserves document bases, and refuses repeated includes, escaped
corpus paths and malformed lists. Every inventory case must have exactly one
reachable manifest entry, and every reachable entry must have an inventory case.
Entry type, shapes and data files and the expected result's form must agree
before execution begins.

Files use the stable corpus base `http://example.org/community/` plus their
root-relative path. Relative RDF IRIs therefore keep the same identity when the
corpus is copied outside a checkout. Independently parsed source documents have
separate blank identity; identical spelling or identical bytes do not join them.

Review admission binds each case's shapes, data, manifest and expected payloads,
the reachable suite manifest and the derivation document by BLAKE3. Every case
has one accepted review with a named reviewer and derivation; the reviewers are
automated agents, which each record states. Catalog, inventory and registry
bytes are excluded from their own payload digest sets to avoid circular
identities. Their schema, identity associations and applicability are admitted
independently.

## SHACL reports

Comparison observes the dedicated complete report graph under the selected dated
test-report policy. It includes conformance, every result, focus, value, severity,
source shape, source constraint component, mandatory source constraint, nested
details and complete path topology. A sequence/inverse path is an RDF structure,
not a path-label string. Stated messages and draft annotations/details are checked
under their declared policy; optional unstated messages cannot introduce an
ordering requirement.

Data and shapes source contexts anchor reported blank focus/value and
source-shape/source-constraint identities. Actual correspondence comes from the
report producer and its actual source objects. Expected correspondence is frozen
from the reviewed input graphs. Report-only isomorphism cannot repair a wrong
source identity. Domains distinguish the separately acquired data and shapes
sources.

Community reports require their normative sourceConstraint. The narrow compatibility
policy for old frozen W3C expectations that omit it cannot be applied here. Rule
cases compare the specified complete inference delta, and the manifest's inline
triple list must equal the expected graph; an empty delta cannot stand in for an
unavailable rules operation.

## Outcomes and provenance

An observation records the case, requested contract, actual surface, the reviewed
expectation, observed kind, exact reason where applicable and verdict. The
distinct observed kinds are comparison, intended rejection, semantic failure,
unsupported, inapplicable, unexecuted, resource exhaustion, malformed, crash and
mismatch. An arbitrary exception cannot pass a negative test. Typed production
refusals must identify the intended rule by its stable reason. Unknown exceptions
remain failures.

A required applicable case must execute and pass. Unsupported or inapplicable rows
do not increment a passing conformance total. An expected rejection that starts
succeeding is a failure requiring a reviewed applicability or oracle change; it
cannot silently remain on an expected-failure ledger. Corrections to a source or
oracle retain the earlier review and apply prospectively.
