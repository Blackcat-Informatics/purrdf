<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Pinned W3C CSVW conformance corpus

This directory vendors the transitive input and expected-result closure used by
`manifest-rdf.jsonld` and `manifest-validation.jsonld` from the W3C CSVW test
suite.

- Upstream: <https://github.com/w3c/csvw>
- Revision: `b3f461db0e86a68c019bc1f912e86f3555907e34`
- Upstream path: `tests/`
- Imported: 2026-07-16
- Upstream terms: W3C Document License, reproduced in `LICENSE.md`

The fixture files are copied byte-for-byte. The harness asserts the exact
manifest classification and approval counts so an upstream refresh cannot add,
remove, or silently promote cases without a reviewed source update.
