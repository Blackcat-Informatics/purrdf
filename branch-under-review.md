# Branch under review — issue #458

Branch `paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (9)

- `241061e7c` fix(crypto): clear owned sponge and sampling storage  _2026-10-06_
- `ad0d8e3eb` perf(gts): skip unused provenance on nonstreamable reads  _2026-10-06_
- `52988974f` fix(gts): preserve authorship across invalid rewrite timestamps  _2026-10-06_
- `b25603862` feat(gts): preserve composite authorship through certified compaction  _2026-10-06_
- `39f0d4dce` feat(gts): wire hedged composite authoring and typed key resolution  _2026-10-06_
- `31498279c` feat(gts): authenticate composite ML-DSA-65 and Ed25519 signatures  _2026-10-06_
- `bf7d5d14f` docs(gts): normalize NIST notice whitespace  _2026-10-06_
- `34e7cfdde` feat(gts): implement native ML-DSA-65 signatures  _2026-10-06_
- `74bf968ce` feat(hash): add streaming SHAKE128 and SHAKE256  _2026-10-06_

## Files changed

```
AGENTS.md                                          |   4 +
 Cargo.lock                                         |   1 +
 bindings/python/src/py_gts.rs                      |   3 +-
 crates/ed25519/Cargo.toml                          |   4 +
 crates/ed25519/src/ct.rs                           |  42 +-
 crates/ed25519/src/lib.rs                          |  10 +
 crates/ed25519/tests/rfc8032.rs                    |  24 +-
 crates/gts/Cargo.toml                              |  17 +
 crates/gts/README.md                               |  37 +
 crates/gts/src/compact.rs                          | 586 ++++++++++---
 crates/gts/src/cose.rs                             | 133 +--
 crates/gts/src/cose/composite.rs                   | 238 +++++
 crates/gts/src/cose/sign1.rs                       | 501 +++++++++++
 crates/gts/src/cose/tests.rs                       |   1 +
 crates/gts/src/fixture.rs                          |  10 +
 crates/gts/src/lib.rs                              |   1 +
 crates/gts/src/mldsa65/codec.rs                    |  89 ++
 crates/gts/src/mldsa65/math.rs                     | 182 ++++
 crates/gts/src/mldsa65/mod.rs                      | 593 +++++++++++++
 crates/gts/src/mldsa65/sampling.rs                 | 227 +++++
 crates/gts/src/model.rs                            |  15 +
 crates/gts/src/policy.rs                           |  14 +-
 crates/gts/src/reader.rs                           |  28 +
 crates/gts/src/verify.rs                           | 163 +++-
 crates/gts/src/writer.rs                           | 271 +++++-
 crates/gts/tests/compaction_signatures.rs          |  40 +-
 crates/gts/tests/composite/IETF-NOTICE.txt         |  39 +
 crates/gts/tests/composite/PROVENANCE.md           |  48 +
 crates/gts/tests/composite/ietf-cose-example.txt   |   8 +
 crates/gts/tests/composite_support/mod.rs          |   6 +
 crates/gts/tests/cose_composite.rs                 | 869 ++++++++++++++++++
 crates/gts/tests/hedged_writer.rs                  | 695 +++++++++++++++
 crates/gts/tests/mldsa65.rs                        | 277 ++++++
 crates/gts/tests/mldsa65/NIST-NOTICE.txt           |   5 +
 crates/gts/tests/mldsa65/PROVENANCE.md             |  56 ++
 crates/gts/tests/mldsa65/keyGen.txt                |  29 +
 crates/gts/tests/mldsa65/sigGen.txt                |  34 +
 crates/gts/tests/mldsa65/sigVer.txt                |  19 +
 crates/hash-conformance/Cargo.toml                 |   6 +
 crates/hash-conformance/tests/shake.rs             | 155 ++++
 .../tests/vectors/shake-PROVENANCE.md              |  39 +
 .../tests/vectors/shake_boundary_vectors.txt       |  21 +
 .../tests/vectors/shake_nist_vectors.txt           |   9 +
 crates/hash/PROVENANCE.md                          |  18 +-
 crates/hash/README.md                              |  51 +-
 crates/hash/src/block.rs                           |  70 +-
 crates/hash/src/lib.rs                             |  20 +-
 crates/hash/src/secret.rs                          | 157 ++++
 crates/hash/src/sha3.rs                            | 364 +++++++-
 crates/rdf/Cargo.toml                              |   5 +
 crates/rdf/src/gts_certify.rs                      | 315 +++----
 crates/rdf/src/gts_write.rs                        |   2 +-
 crates/rdf/tests/gts_certify.rs                    |  16 +-
 crates/rdf/tests/gts_composite_compaction.rs       | 967 +++++++++++++++++++++
 helpers-ledger.toml                                |  74 +-
 scripts/check-issue-refs.py                        |   7 -
 56 files changed, 6996 insertions(+), 619 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

