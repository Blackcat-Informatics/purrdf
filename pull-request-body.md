## Summary

Prepare PurRDF 3.0.0 for coordinated Cargo, PyPI, npm and C publication. Publication and installed-artifact/attestation verification remain tracked in #375.

- Establish typed fallible dataset reads, pinned term guards, checked `u64` logical addresses and an opt-in segmented storage format. Resident datasets retain compact IDs and borrowed access; operational sessions share admission across storage, supported selective queries and streamed export.
- Expose exact JavaScript `bigint` identities, generations and asynchronous exchange IDs. Add structured diagnostics and align Python exports with the native canonical five-table schema.
- Complete recipient licensing/notices across all distributions and deployed/offline documentation. Preserve pack, columnar and GTS payload compatibility; acquire the pinned XML conformance corpus without redistributing its extracted payloads.
- Coordinate 3.0.0 versions, reciprocal Python requirements, locks, citation, generated C header and migration/release notes. Use hash-locked Python publishing tools and publish the exact audited npm tarball.

## Validation

All results below bind the reviewed head `9d60a8298e1ecd39bb4f9aaef2f8a7e7bbef1f98`.

- [CI](https://github.com/Blackcat-Informatics/purrdf/actions/runs/36961861726): all 42 jobs passed. All 11 uploaded artifacts were independently qualified, including 714 assembly cells and 29 conformance suites with 15,286 passes, 25 ledgered gaps and zero failures.
- [Docs](https://github.com/Blackcat-Informatics/purrdf/actions/runs/36961861543) and [CodeQL](https://github.com/Blackcat-Informatics/purrdf/actions/runs/36961859082) passed. The reported Markdown-prefix repetition was repaired with a bounded advancing scanner; its grammar and attack regressions pass.
- Fresh stable 1.98.1 Cargo (31 archives), Python (four archives and joint install), C (relocated linked smoke), and npm qualification passed. npm passed all 417 runtime tests, TypeScript, bigint checks and clean installation of the exact noticed tarball. All 37 candidates passed independent integrity, source-binding and recipient-notice audits.
- Fresh compilation reproduced all ten inputs of the separately retained actual Chromium qualification byte-for-byte: 100,100 resident rows and the million-row persistent scan, selective join and streamed export under a 256 MiB module cap. This evidence qualifies that browser environment. No physical ARM qualification or comparative performance improvement is claimed.
- Earlier full local `make check` receipts are retained and labeled for their exact candidate. The mandatory integrated-main release preflight runs before tags are cut.

## Checklist

- [x] Final committed candidate passes hosted CI, Docs and CodeQL.
- [x] No semantic Cargo features; all 31 publishable crates build for WASM.
- [x] Generated artifacts, deterministic outputs and frozen corpus pins are verified.
- [x] Migration guidance, README and release notes describe shipped behavior.
- [x] Final source-bound distribution candidates pass qualification.
- [x] Performance statements are limited to validated evidence.
