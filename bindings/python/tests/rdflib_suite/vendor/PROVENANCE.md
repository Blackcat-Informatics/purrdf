<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
This PROVENANCE file is authored by the purrdf project. The vendored `test_*.py`
files and `LICENSE` in this directory are rdflib's own and retain their license.
-->

# Vendored rdflib conformance tests — provenance

This directory holds a **curated, verbatim** subset of [RDFLib](https://github.com/RDFLib/rdflib)'s
own test suite, run **unmodified** against `purrdf.compat.rdflib` through the
top-level `rdflib` shadow. It is the source corpus for the "rdflib LSP
conformance gate".

## Upstream source

| Field | Value |
| --- | --- |
| Package | `rdflib` |
| Version | **7.6.0** |
| Distribution | sdist (`rdflib-7.6.0.tar.gz` — the wheel omits `test/`) |
| sdist SHA-256 | `6c831288d5e4a5a7ece85d0ccde9877d512a3d0f02d7c06455d00d6d0ea379df` |
| Fetch | `pip download rdflib==7.6.0 --no-binary :all: --no-deps` |
| Upstream test root | `rdflib-7.6.0/test/` |
| License | **BSD-3-Clause** (see `LICENSE`, copied verbatim from the sdist) |

The files below are byte-for-byte copies of the upstream files; only their
directory location is flattened into `vendor/`. No source edits were made — the
gate's job is to run rdflib's real assertions against the shim.

## Vendored files (curated subset)

Chosen to target rdflib's **public** Graph / term / namespace / SPARQL /
collection / list API, with no dependency on rdflib's private `test.utils` /
`test.data` harness (see exclusions). Upstream path → vendored name:

| Upstream path | Vendored as |
| --- | --- |
| `test/test_misc/test_collection.py` | `test_collection.py` |
| `test/test_misc/test_prefix_types.py` | `test_prefix_types.py` |
| `test/test_misc/test_conventions.py` | `test_conventions.py` |
| `test/test_misc/test_bnode_ncname.py` | `test_bnode_ncname.py` |
| `test/test_misc/test_b64_binary.py` | `test_b64_binary.py` |
| `test/test_graph/test_graph_items.py` | `test_graph_items.py` |
| `test/test_graph/test_graph_operator.py` | `test_graph_operator.py` |
| `test/test_graph/test_batch_add.py` | `test_batch_add.py` |
| `test/test_literal/test_normalized_string.py` | `test_normalized_string.py` |
| `test/test_literal/test_tokendatatype.py` | `test_tokendatatype.py` |
| `test/test_literal/test_hex_binary.py` | `test_hex_binary.py` |
| `test/test_namespace/test_definednamespace_dir.py` | `test_definednamespace_dir.py` |
| `test/test_path.py` | `test_path.py` |
| `test/test_having.py` | `test_having.py` |
| `test/test_sparql/test_construct_bindings.py` | `test_construct_bindings.py` |
| `test/test_sparql/test_evaluate_bind.py` | `test_evaluate_bind.py` |
| `test/test_sparql/test_optional.py` | `test_optional.py` |
| `test/test_sparql/test_subselect.py` | `test_subselect.py` |
| `test/test_sparql/test_agg_distinct.py` | `test_agg_distinct.py` |
| `test/test_sparql/test_agg_undef.py` | `test_agg_undef.py` |
| `test/test_sparql/test_nested_filters.py` | `test_nested_filters.py` |
| `test/test_conjunctivegraph/test_conjunctive_graph.py` | `test_conjunctive_graph.py` |

## Scoreboard (rdflib 7.6.0 vs purrdf shim, live)

**62 passed / 24 xfailed** (0 xpassed, 0 failed, 0 collection errors; ledger
24/24 applied, 0 stale).

Every one of the 24 xfails has a concrete, self-describing reason in
`../xfail_ledger.toml`, applied as a **strict** xfail (an XPASS or stale key
fails the gate → the ledger only shrinks). Themes: Graph subclass identity
through set operators (3), rdf:List parse + Collection mutation (3),
BatchAddGraph context handling (1), DefinedNamespace `__dir__` (1), SPARQL
`Result.bindings` / `SELECT *` subselect projection (8), SPARQL prefix
forwarding + `VALUES` (2), SPARQL aggregate/nested-FILTER evaluation (3),
ConjunctiveGraph legacy semantics (3).

(The XSD whitespace-facet and binary-datatype-coercion themes cited in earlier
revisions have been fully resolved and pruned — the shrink-only mechanism raised
the pass count from 50 to 62.)

## Deliberately EXCLUDED — explicit, not silent

Nothing here is silently skipped. Three exclusion classes, each with the reason
it is out of scope for a **public-API** drop-in conformance gate:

### 1. Requires rdflib's private test harness (`test.utils` / `test.data`)

rdflib's richer Graph/term/namespace tests import `from test.utils import
GraphHelper`, `from test.data import …`, `from test.utils.outcome import …`,
etc. That harness package **itself does not import against the shim** — e.g.
`test/utils/__init__.py` does `from rdflib.term import IdentifiedNode`, and the
shim's `term` module does not yet re-export `IdentifiedNode` (plus a cascade of
other private/edge names). Vendoring the harness would mean either patching the
shim's surface or forking rdflib's harness — both out of scope for this gate.
Excluded on this basis (non-exhaustive): `test_graph/test_graph.py`,
`test_graph/test_graph_context.py`, `test_graph/test_slice.py`,
`test_graph/test_diff.py`, `test_graph/test_canonicalization.py`,
`test_literal/test_literal.py`, `test_literal/test_term.py`,
`test_namespace/test_namespace.py`, `test_namespace/test_namespacemanager.py`.
Tracked as a follow-up (broaden the shim's term/namespace surface,
then vendor these).

### 2. Import-level shim gaps (whole module fails to collect)

These target public API but their **module-level imports** reference a shim
submodule/name that does not yet exist, so pytest cannot collect them into
per-test items (an all-or-nothing collection error, not a ledgerable per-test
xfail). Excluded with the missing symbol noted; each is a real, tracked gap:

| Upstream file | Missing shim symbol |
| --- | --- |
| `test_graph/test_container.py` | `rdflib.container` module |
| `test_graph/test_graph_formula.py` | `rdflib.graph.QuotedGraph` |
| `test_literal/test_term.py` | `rdflib.graph.QuotedGraph` |
| `test_literal/test_datetime.py`, `test_literal/test_duration.py` | `rdflib.xsd_datetime` module |
| `test_sparql/test_operators.py`, `test_sparql/test_functions.py` | `rdflib.plugins.sparql.operators` submodule |
| `test_sparql/test_prepare.py` | `rdflib.plugins.sparql.prepareUpdate` |
| `test_sparql/test_sparql_parser.py` | `rdflib.plugins.sparql.processor` submodule |
| `test_skolem_genid.py` | `rdflib.term.Genid` |
| `test_aggregate_graphs.py` | `rdflib.logger` |
| `test_parsers/test_parser.py`, `test_literal/test_uriref_literal_comparison.py` | `rdflib.plugins.parsers.rdfxml` submodule |

### 3. rdflib-internal implementation detail / infrastructure

Out of scope for a public-API drop-in gate (they test rdflib internals, specific
store backends, plugin/registry internals, C-extension paths, or the network):
the `test_store/` tree (BerkeleyDB / Memory / triple-store backends),
`test_graph/test_graph_http.py` and `test_graph/test_graph_redirect.py`
(network), `test_graphdb/` and `test_rdf4j/` (live remote HTTP servers),
`test_misc/test_plugins.py` / `test_misc/test_security.py` (plugin registry &
audit internals), `test_extras/` (infixowl / external graph libs),
`test_w3c_spec/` and `test_roundtrip.py` (W3C manifest runners that pull rdflib's
data corpus + `test.utils.manifest`), and `jsonld/` (JSON-LD codec, not part of
the native format set).

## Refresh procedure

To re-vendor against a newer rdflib:

```sh
pip download rdflib==<VER> --no-binary :all: --no-deps -d /tmp/rdflibsrc
tar -C /tmp/rdflibsrc -xzf /tmp/rdflibsrc/rdflib-<VER>.tar.gz
# re-copy the curated file list above from rdflib-<VER>/test/, update the
# version + sha here, then re-run `python tests/rdflib_suite/runner.py` and
# reconcile ../xfail_ledger.toml (strict xfail forces the ledger to stay exact).
```
