<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Portable community conformance corpus

These are **community-proposed SHACL tests**. Their expectations were derived
from the dated specifications and reviewed by automated agent reviewers before
any engine output was observed; they have no W3C approval status and no
independent human review. `catalog.json` identifies the versioned contract,
profile applicability, the authoritative inventory and the RDF manifest root.
The standard manifests, shapes and inline expected reports follow the
[W3C Data Shapes test format](https://w3c.github.io/data-shapes/data-shapes-test-suite/)
and can be used without PurRDF. The JSON catalog and inventory add acquisition
and review provenance; they do not replace the RDF manifests.

Read [the corpus contract](SPEC.md) before interpreting a score. The review
records are in [the review registry](reviews/index.json) and
[the SHACL review](reviews/shacl.md). Fixture payloads use
`MIT OR Apache-2.0 OR MulanPSL-2.0`; documentation carries its own CC-BY-4.0
header, and [the licensing guidance](LICENSING.md) lists the full texts. All
application IRIs are examples. The W3C test and language vocabularies keep their
standard namespace identities.

From a PurRDF source checkout, run the shared grader over the whole corpus:

```sh
cargo test -p purrdf-sparql-conformance --test community_conformance -- --nocapture
cargo run -p purrdf-sparql-conformance --bin community-conformance -- \
  corpora/community /tmp/community-native-evidence
```

The binary's output directory must be new. It receives one graded record per
public route (`records.json`), the same verdicts as EARL assertions (`earl.nt`)
and the scoreboard. Every applicable case runs under each profile it names:
the SHACL Recommendation of 20 July 2017 (`shacl-20170720`) and the SHACL 1.2
SPARQL Extensions Working Draft of 18 September 2026 (`shacl12-20260918`). A
catalog profile that no built-in dated law implements is counted as
unsupported; it is never a pass and never folded into the failures.
