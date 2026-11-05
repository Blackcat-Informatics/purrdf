<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# YAML-LD Basic input options

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

`version` and the `yaml_ld` object are required. Every nested field is optional;
the example states all native defaults. `extract_all_scripts: true` constructs
all documents through the same JSON-LD engine. Syntax limits cover the complete
stream; projection limits cover retained syntax and selected construction.
The YAML phase accounts remain those described in [conformance](CONFORMANCE.md).

Limits use exact unsigned integer JSON lexemes. Fractional or exponent notation,
negative numbers, null records, duplicate fields and unsupported versions fail
before RDF parsing. `max_occurrences`, `max_nodes` and `max_numeric_work` accept
`0..=18446744073709551615`. All other numeric limits accept `0..=4294967295` on
every host. Zero is an actual limit.

Options decoding has separate bounds: 16,384 UTF-8 bytes, four simultaneously open
JSON containers, 64 value occurrences, and 131,072 admitted native bytes for the
JSON tree, parser buffers and strict-record metadata. Diagnostic transport and
the subsequent YAML syntax and projection accounts are separate. These controls
do not claim a combined process-memory or end-to-end allocation ceiling.
`max_serialization_bytes` is refused because direct RDF input does not serialize.

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
validation. Shapes documents, prepared products, imports and added/removed change
documents retain their existing input policy. The flag is also valid with
`--shapes-product`, because the primary data is still parsed.
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
reads no data and refuses the flag. Shapes, SRL, expression and import documents
retain their existing input policy; coordinates, selectors, bases, governors,
serialization and reports retain their existing behavior.
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
