<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Vendored W3C XML Conformance Test Suite

Frozen copy of the W3C XML Conformance Test Suite, version 2013-09-23
(`xmlts20130923`), vendored for `crates/lex/tests/xmlconf.rs`, which grades
`purrdf_lex::xml` against it. **Do not hand-edit**: the freeze is enforced by
`scripts/check-corpus-frozen.py` against
`scripts/conformance-frozen/vectors-xmlconf.sha256`, and the tree is
regenerated only by `python3 scripts/vendor-xmlconf.py`.

## Source

- Upstream: <https://www.w3.org/XML/Test/>
- Retrieval: `https://www.w3.org/XML/Test/xmlts20130923.tar.gz`
- Tarball SHA-256: `9b61db9f5dbffa545f4b8d78422167083a8568c59bd1129f94138f936cf6fc1f`
- The tarball's `xmlconf/` tree is written verbatim (3386 files; the tarball's
  own layout, file bytes and names are unchanged; only the archive's owner and
  permission bits are dropped).
- Licence: the suite is a collection of contributed sub-suites (James Clark's
  XMLTEST, Sun Microsystems, OASIS/NIST, IBM, Fuji Xerox, the University of
  Edinburgh), each under the terms stated in its own directory (for example
  `xmltest/readme.html`) and the W3C test-suite licence at
  <https://www.w3.org/Consortium/Legal/2008/04-testsuite-copyright.html>. The
  files are redistributed here unmodified and are test data only: no crate
  compiles any of it in.

## Contents

`xmlconf.xml` is the master manifest (`TESTSUITE`, one `TESTCASES` per
sub-suite, each `TEST` naming its document by `URI` with the attributes
`TYPE`, `VERSION`, `EDITION`, `ENTITIES`, `NAMESPACE`, `RECOMMENDATION` and
`OUTPUT`). `testcases.dtd` describes those attributes.
