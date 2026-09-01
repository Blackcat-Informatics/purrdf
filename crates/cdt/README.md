<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# purrdf-cdt

Native SPARQL **composite datatypes** (SEP-0009: `cdt:List` and `cdt:Map`) for
the PurRDF RDF 1.2 query stack.

`purrdf-cdt` is a **closed leaf**: its only runtime dependencies are
`purrdf-iri` (absolute-IRI validation) and `purrdf-xsd` (the XSD value space).
It has no dependency on `purrdf-core` in either direction, which is precisely
what lets the kernel depend on it later without a cycle. It is `#![no_std]`
plus `alloc`, so it builds for `wasm32-unknown-unknown` like every other
release crate in the workspace.

What it gives you:

* `parse_list` / `parse_map` / `parse_cdt` — the SEP-0009 lexical grammar,
  scanned **iteratively** under three published resource bounds (nesting depth,
  element count, total bytes). No recursive descent anywhere, so a hostile
  lexical form yields a typed error carrying a byte offset instead of an
  uncatchable stack-overflow abort.
* `CdtValue::canonical_lexical` — a byte-deterministic canonical spelling that
  PurRDF chooses for the values it computes (SEP-0009 defines none).
* `list_equal` / `map_equal` / `list_less_than` / `map_less_than` — the
  spec-defined, partial, error-propagating comparisons.
* `total_value_cmp` — a provably transitive *syntactic* total order for
  `ORDER BY`, map key order and render order.
* `parse_literal` — one closed `LiteralValue` enum over the XSD and CDT value
  spaces, preserving the parsed / not-this-value-space / ill-typed tri-state.

Two documented PurRDF supersets of the SEP-0009 lexical space (no IRI is
minted; the datatype stays `cdt:List` / `cdt:Map`): RDF 1.2 **triple terms**
`<<( s p o )>>` as elements, and **directional** language-tagged literals
`"lex"@lang--ltr` / `"lex"@lang--rtl`. Both are emitted only when such a term
is actually present, i.e. only for values SEP-0009 cannot express at all.

Licensed under MIT OR Apache-2.0.
