<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# W3C SPARQL 1.0 data-r2 corpus

This tree preserves every payload file from `w3c/rdf-tests`,
`sparql/sparql10/`, at commit `426c7df4b5d5d292e3ba09dc22e622ea301f230a`.
The upstream corpus is the DAWG data-r2 suite retained by SPARQL 1.1.
Every payload has a `LicenseRef-W3C-Test-Suite` sidecar; the upstream
`LICENSE` is preserved verbatim. Files are never hand-edited.

Every file keeps its upstream name and path. The native runner discovers
the 29 group manifests and runs each as its own case. The root `manifest.ttl`
only includes the 29 groups, so the runner treats it as an index: it checks
that every member is discovered and never runs the members through it, which
would run them twice. The native inventory test loads its closure to prove it
equals the 29 groups exactly. The one case of the upstream extended root,
`extended-manifest-evaluation.ttl`, is `dawg-sort-11`, whose frozen order
RDF 1.2 makes unreachable; it is graded in its own conformance row against
the SPARQL 1.2 order.

Upstream: <https://github.com/w3c/rdf-tests/tree/426c7df4b5d5d292e3ba09dc22e622ea301f230a/sparql/sparql10>.
The W3C Test Suite License is recorded at
<https://www.w3.org/Consortium/Legal/2008/04-testsuite-license.html>.
