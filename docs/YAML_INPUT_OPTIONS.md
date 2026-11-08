<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# YAML-LD input options

The shared `purrdf_validate::rdf_input` boundary accepts original RDF bytes and
optional JSON configuration. Without configuration, YAML-LD constructs the first
document. Explicit configuration requires YAML-LD input, including when `yaml_ld`
is empty. It selects Basic only. Profiles, extensions, unknown fields and
serialization controls are refused.

```json
{
  "version": 1,
  "yaml_ld": {
    "extract_all_scripts": false,
    "syntax": {
      "max_input_bytes": 16777216,
      "max_allocation_bytes": 134217728,
      "max_occurrences": 1048576,
      "max_depth": 128,
      "aliases": true,
      "max_documents": 1048576
    },
    "projection": {
      "max_allocation_bytes": 134217728,
      "max_nodes": 1048576,
      "max_depth": 128,
      "max_numeric_work": 67108864,
      "max_numeric_bytes": 33554432
    }
  }
}
```

`version` and the `yaml_ld` object are required. Every Basic control is optional;
the example states all native Basic defaults. `extract_all_scripts: true` constructs
all documents through the same JSON-LD engine. Syntax limits cover the complete
stream; projection limits cover retained syntax and selected construction.
The YAML phase accounts remain those described in [conformance](CONFORMANCE.md).

Limits use exact unsigned integer JSON lexemes. Fractional or exponent notation,
negative numbers, null control records, duplicate fields and unsupported versions fail
before RDF parsing. `max_occurrences`, `max_nodes` and `max_numeric_work` accept
`0..=18446744073709551615`. All other numeric limits accept `0..=4294967295` on
every host. Zero is an actual limit.

Options decoding has separate bounds derived from the native context envelope:
10 MiB of UTF-8, 64 simultaneously open JSON containers and 262,144 value
occurrences. A separate 128 MiB construction account covers JSON/record decoding,
the retained initial value, pending registry text and its vector. Native registry
construction of shared byte storage and the lookup map is outside that account,
under its separate fixed native ceilings.
This replaces the earlier flat-control envelope of 16 KiB, four containers,
64 values and 128 KiB of construction storage. Diagnostic transport and
the subsequent YAML syntax and projection accounts are separate. These controls
do not claim a combined process-memory or end-to-end allocation ceiling.
`max_serialization_bytes` is refused because direct RDF input does not serialize.

The `yaml_ld` object also accepts independently optional `context` and `registry`:

```json
{
  "version": 1,
  "yaml_ld": {
    "context": {"label": "https://example.org/label"},
    "registry": [
      {
        "iri": "https://example.org/context.jsonld",
        "format": "json",
        "document": "{\"@context\":{\"p\":\"https://example.org/p\"}}"
      },
      {
        "iri": "https://example.org/context.yamlld",
        "format": "yaml",
        "document": "'@context': {q: 'https://example.org/q'}\n",
        "yaml_ld": {}
      }
    ]
  }
}
```

An absent initial context preserves default initialization; an explicit
`context: null` supplies a JSON-LD context reset. A registry without an initial
context supports references in the input document. Each registry record requires
an absolute `iri`, explicit `json` or `yaml` format and original document text.
YAML records require their own `yaml_ld` object, with the same Basic controls;
JSON records refuse that field. Duplicate IRIs, null registries, unsupported
formats and unknown fields are errors. Document text is decoded directly with
the selected native decoder, without format inference or serialization.

Compilation uses the effective caller base, including the CLI's retrieval IRI
when no explicit base is supplied. There is no separate options document-IRI
override. Registered IRIs supply the bases for relative context, import and
scoped-context references; no network or filesystem context loading occurs.
Native context limits still apply: 1 MiB per context document, 8 MiB aggregate
registered bytes and 128 registered documents, plus compiler work and term
ceilings. Context wrapper and import restrictions remain unchanged.

Native `ParseOptions::yaml_ld_context` borrows a compiled context and uses default
Basic controls when `yaml_ld` is absent. The context's original compilation
document IRI must match the caller parse base; its active base may differ through
an explicit `@base`. The shared parse outcome preserves Caller base provenance
when unchanged, Enclosing provenance when a context changes it, and no base when
explicitly cleared. Foreign input signatures and serialization context handles
retain their existing form. These controls do not establish full upstream
conformance.

Python module `parse`, `Store.load`, `Store.bulk_load` and `MutableDataset.load`
accept keyword-only `input_options` JSON text. Store loads retain their existing
per-load blank-node scope and prefix results. JavaScript uses
`Dataset.parse(input, format, base, rememberEmptyGraphs, inputOptions)`. Its string
interface measures UTF-8 bytes and cannot transport invalid UTF-8 bytes.
Native semantic and resource errors retain their code and original location:
Python exposes `code` and `diagnostic_json`; JavaScript exposes `code` and
`diagnostic` on the thrown `Error`.

The CLI accepts `convert --yamlld-input-options PATH`. The flag applies to every
conversion input, including canonicalization and entailment. `update` and `query`
accept the same flag for their sole `--data` document. Query applies it in ordinary,
governed, explain and entailment execution, using its existing filename-based
input selection. Query premise imports retain their default input policy.
`validate` accepts the flag for primary `IN`, on both whole-graph and incremental
validation. Prepared products retain their existing input policy. The flag is also valid with
`--shapes-product`, because the primary data is still parsed.
Validation accepts independent `--changes-yamlld-input-options PATH` and
`--changes-removed-yamlld-input-options PATH` for added and removed documents.
Each requires its matching change document and is also valid with a shapes
product. Both roles and formats are admitted before either change options file is read;
all requested controls decode before RDF or product acquisition and shapes-base
filesystem resolution. A configured role requires YAML-LD, while the other may
use another format. The existing `--changes-from` override applies to both;
otherwise each format is inferred independently. Unconfigured roles keep native
defaults, independently of primary, shapes, import and neighboring controls.
`describe` and `project` accept the flag for primary `IN`. Description selectors
retain their existing base resolution. Projection configuration and assets keep
their existing input policy; YAML controls are admitted before either is read.
`lift` reads a carrier archive and does not accept YAML input controls.
`reason` and `consistency` accept the flag for primary `IN` only. Imports retain
their existing controls and `--from` scoping. Reason admits the options before RIF
rule acquisition; consistency retains its lossless N-Quads crossing and existing
proof/check-proof behavior.
`entails --yamlld-input-options PATH` configures only `--premise`;
`--conclusion-yamlld-input-options PATH` independently configures only
`--conclusion` and requires that graph question, conflicting with `--pattern`.
Premise controls also apply in pattern mode. Both requested configurations are
decoded before any RDF source is read. Each configured role must resolve to
YAML-LD; the other role may use a different inferred format. Imports retain their
existing input policy and global `--from` scoping.
`shex --yamlld-input-options PATH` configures only `--data`, with configuration
decoding before data, schema or schema-import acquisition. Schema syntax, imports,
shape-map selectors and base policy retain their existing behavior.
`rules` and `node-expr` accept the flag for primary `IN` only. Configuration
admission precedes data, rule, shapes, import and expression document acquisition.
Both SHACL and SRL rules use the same configured data loader. `rules --check`
reads no data and refuses the flag. SRL, expression and import documents
retain their existing input policy; coordinates, selectors, bases, governors,
serialization and reports retain their existing behavior.
`validate`, `rules` and `node-expr` also accept independent
`--shapes-yamlld-input-options PATH` for their root `--shapes` document. Each
configured role must be YAML-LD; data and shapes may otherwise use different
formats. Both configurations are admitted before either RDF role or any import,
change, rule or expression document is acquired. Shapes controls require
`--shapes` and are refused with validation products or SRL/check-only rules.
`shapes lint --yamlld-input-options PATH` configures its root `FILE`;
`shacl pack --yamlld-input-options PATH` configures its root `--shapes` document.
Pack retains its Turtle default, so YAML-LD requires `--shapes-from yamlld`.
The native shapes parse retains its final declared base and prefix metadata for
import resolution.
The five shapes-document commands accept repeatable
`--import-yamlld-input-options IRI=PATH` for individual RDF imports. Each absolute
IRI must name one actual `--import` pair. Duplicate or unknown control IRIs,
empty halves and controls targeting non-YAML-LD imports are usage errors. Other
imports may retain any supported format. Every mapping and target is checked
before opening an options file; every requested options file is decoded before
any RDF source is acquired. Each import receives only its own controls; an
unmapped import keeps native defaults. A valid imported-input limit applies when
that document is parsed. Product validation and SRL/check-only rules refuse the
flag. The same per-IRI flag is available for premise RDF imports on `reason`, `entails`,
`consistency`, `convert --entailment` and `query --entailment`. Controls require
entailment on convert/query and are refused on ordinary, canonical-only or
explain paths. Each mapped import receives only its own controls, independently
of primary, conclusion and root shapes configuration. Mapping admission and
configuration decoding precede RDF and RIF acquisition. Reason, convert, entails
and consistency retain global `--from` import selection; query infers each
import's format from its filename. Premise imports keep the caller's `--base` and
permit the sole imported stdin when an explicit format resolves it. SHACL imports
keep ontology-IRI bases and native document metadata. SRL and ShEx schema imports
keep their existing policy.
Every selected input must be YAML-LD; pack, GTS and other native formats are
refused before data acquisition. The bounded named options file is validated
before primary data, premise imports or rule documents are read; it does not
consume standard input. Other commands reject the flag. Existing output options
are independent.
For validation, configuration decoding also precedes change documents and shapes
document/product/import acquisition.

```sh
purrdf update --data input.yamlld --yamlld-input-options options.json --to nquads \
  'INSERT { ?s <urn:seen> true } WHERE { ?s <urn:value> ?o }'
```

Without the flag, the existing first-document behavior is preserved. A configured
input refusal writes no updated dataset and leaves an existing output intact.
Query input refusals emit no answers or explanation.
Validation input refusals publish no report and preserve an existing output.

The C ABI takes nullable `input_options` bytes and `input_options_len` after
`source_name` in `purrdf_parse`. Null with length zero keeps default parsing;
non-null input must contain one complete options document. The returned handle
owns the frozen dataset. Errors expose the original diagnostic record through
`purrdf_error_presentation_json` until the error handle is freed.
