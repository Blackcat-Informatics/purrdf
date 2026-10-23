<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Vendored W3C SHACL 2017 test inventory (`alltests.ttl`)

Byte-frozen copy of the W3C Data Shapes Working Group's inventory of the
SHACL 1.0 (2017) test suite, vendored so the `purrdf-shapes` harness can check
that the EARL implementation report it writes covers exactly the tests the
Working Group lists. **Do not hand-edit.**

- Upstream: <https://github.com/w3c/data-shapes>
  (`data-shapes-test-suite/reports/alltests.ttl`)
- Pinned commit: `4cd469867a1cc6d2fff80e8a40723b26c695d67a`
- License: W3C Software and Document License
  (<http://www.w3.org/Consortium/Legal/copyright-software>), per the upstream
  repository's `LICENSE.md`, vendored at `vectors/shacl12/LICENSE.md` from the
  same commit.
- Copied with:

  ```sh
  git -C <data-shapes clone> show \
    4cd469867a1cc6d2fff80e8a40723b26c695d67a:data-shapes-test-suite/reports/alltests.ttl \
    > vectors/shacl-reports/alltests.ttl
  ```

  The copy was verified byte-equal to that blob with `cmp`.

The file is byte-frozen: `scripts/check-corpus-frozen.py` (run by `make check`)
SHA-256-verifies it against `scripts/conformance-frozen/vectors-shacl-reports.sha256`.
A deliberate re-vendor regenerates that manifest with
`python3 scripts/check-corpus-frozen.py --update`.

## Consumer

`crates/shapes/tests/w3c_conformance.rs`, test
`w3c_2017_earl_matches_wg_inventory`, parses this file and requires the
`urn:x-shacl-test:` subjects whose `mf:status` is `sht:approved` (120) to be
exactly the tests of the EARL report the harness renders, each once, with the
`sht:proposed` subject absent.
