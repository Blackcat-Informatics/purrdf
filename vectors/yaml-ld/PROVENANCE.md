<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Pinned W3C YAML-LD test corpus

- Upstream: <https://github.com/w3c/yaml-ld>
- Revision: `fafcf7ae69d5e5cb7402e71dfd5cd2c03c18f29f`
- Cutoff: the latest repository revision at the end of 24 October 2026, the
  publication date of the pinned YAML-LD 1.0 Working Draft.
- Source archive: <https://codeload.github.com/w3c/yaml-ld/tar.gz/fafcf7ae69d5e5cb7402e71dfd5cd2c03c18f29f>
- Imported paths: every regular file under `tests/cases/`, plus
  `tests/manifest.jsonld`, `tests/context.jsonld`, `tests/LICENSE.md` and
  `tests/README.md`, at their original paths.

The 123 upstream files contain 73,243 bytes: 119 case files and four supporting
files. All have Git mode `100644`. The manifest directly references 118 distinct
input, expected, context and frame files; the remaining case file is the
transitive context `tests/cases/yaml-schema/context.yamlld`. Empty files,
original line endings, BOMs and the UTF-16 refusal fixture are retained verbatim.
The shared-blank-node input and expected result are present at this full-repository
revision. The older manifest-only snapshot omits those referenced files.

The files were extracted from the exact pinned source archive. Each extracted
file's size and Git blob identity were checked against the pinned repository tree.
No upstream generator was executed. `tests/generate_index.js` and rendered
`tests/manifest.html` are intentionally omitted. Git attributes disable text
conversion and text diff whitespace policy for the upstream payload only.

## License

The upstream `tests/LICENSE.md` notice and `tests/README.md` are unmodified.
The notice adopts the W3C test-suite dual-licensing policy. Its existing local
texts are [the policy](../../LICENSES/W3C-test-suites-policy.html),
[the test-suite license](../../LICENSES/W3C-test-suite-license.html) and
[the three-clause BSD alternative](../../LICENSES/W3C-test-suite-BSD.html).

## Inventory and native selection

| Manifest family | Entries | Normative entries selected here |
| --- | ---: | ---: |
| Expand | 80 | 0 |
| ToRDF | 13 | 11 |
| Compact | 2 | 0 |
| Flatten | 1 | 0 |
| Frame | 1 | 0 |
| Total | 97 | 11 |

Five entries explicitly declare `extendedYAML: true` and `normative: false`:
three Expand entries and the two ToRDF exclusions below. The native
`pinned_w3c_normative_rdf_construction` case in
`crates/rdf/tests/yaml_mapping_keys.rs` requires this complete normative ToRDF
inventory, reads the actual `extractAllScripts` option, and compares the native
construction with upstream N-Quads by RDF dataset isomorphism:

- `#aa-information-1-positive`
- `#shared-blank-node-from-stream`
- `#empty-document-from-stream-rdf`
- `#empty-document-rdf`
- `#empty-document-rdf-all`
- `#zero-documents-rdf`
- `#zero-documents-rdf-all`
- `#all-empty-rdf`
- `#all-empty-rdf-all`
- `#leading-empty-rdf`
- `#trailing-empty-rdf`

The explicit ToRDF exclusions are `#cir-scalar-core-2-positive` and
`#cir-scalar-other-2-positive`, both informative Extended YAML cases.

Vendoring the whole inventory does not qualify its unselected families. The
other 81 normative cases, including expansion and HTML processing, compaction,
flattening and framing, remain unqualified here. The five informative cases
remain outside this Basic selection. This family does not establish complete
pinned Basic conformance, other required upstream-suite qualification or the
remaining deterministic semantic-output acceptance.
