<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# W3C SPARQL 1.0 data-r2 corpus

This tree preserves every payload file from `w3c/rdf-tests`,
`sparql/sparql10/`, at commit `426c7df4b5d5d292e3ba09dc22e622ea301f230a`.
The upstream corpus is the DAWG data-r2 suite retained by SPARQL 1.1.
Every payload has a `LicenseRef-W3C-Test-Suite` sidecar; the upstream
`LICENSE` is preserved verbatim. Files are never hand-edited.

The only filename mapping is upstream `manifest.ttl` to `manifest-all.ttl`.
Its bytes and relative include targets are unchanged. The native runner
discovers the 29 leaf group manifests and the sort `extended-manifest.ttl`;
the root aggregator is loaded only by the native inventory test to prove
closure coverage without executing its children twice.

Upstream: <https://github.com/w3c/rdf-tests/tree/426c7df4b5d5d292e3ba09dc22e622ea301f230a/sparql/sparql10>.
The W3C Test Suite License is recorded at
<https://www.w3.org/Consortium/Legal/2008/04-testsuite-license.html>.
