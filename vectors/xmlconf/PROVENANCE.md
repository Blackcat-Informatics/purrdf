<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Acquired XML conformance suite

Tests use the unmodified XML conformance collection `xmlts20130923`, acquired
by `python3 scripts/vendor-xmlconf.py` into `target/conformance/xmlconf`.
The source repository and release archives contain no extracted suite payload.

The upstream archive is https://www.w3.org/XML/Test/xmlts20130923.tar.gz with
SHA-256 `9b61db9f5dbffa545f4b8d78422167083a8568c59bd1129f94138f936cf6fc1f`.
Every one of its 3386 files retains its exact bytes, checked against the
unchanged `scripts/conformance-frozen/vectors-xmlconf.sha256`. Both the freeze
gate and the XML test harness verify acquisition; a valid cache works offline.
The reader scoreboard and all test attributes remain unchanged.

Each contributed sub-suite keeps the terms stated by its original authors.
In particular, James Clark's XMLTEST readme restricts redistribution to the
unmodified original `xmltest.zip`; the W3C collection contains later changes
and is not that original archive. We acquire the W3C collection for local
verification rather than redistributing its extracted XMLTEST files.
The [W3C test-suite policy](https://www.w3.org/copyright/test-suites-licenses/)
does not retroactively replace the licenses of older suites.
