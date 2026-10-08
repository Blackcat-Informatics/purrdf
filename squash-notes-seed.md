Implement native visible-text glossary enforcement

<!-- stagectl: replace the TODO lines before merging -->

Closes #260

## What changed

TODO: what this does and why, in prose. The commit subjects below are
      the WHAT; this section is the WHY, and it is the part a reader
      six months from now cannot reconstruct from the diff.

## Commits squashed (4)

- `cdc43aaa6` Reject ambiguous keep-English anchors and preserve restoration failures
- `b1833aeed` glossary: use exact link and fence whitespace boundaries
- `4db9f7947` refactor(i18n): route glossary checks through the native gate
- `97b05f758` Implement native visible-text glossary enforcement

## Files

```
.githooks/pre-commit                               |    9 +-
 .github/workflows/ci.yaml                          |    2 +-
 .github/workflows/docs.yaml                        |    1 +
 Cargo.lock                                         |    1 +
 Makefile                                           |    4 +-
 crates/helper-census/Cargo.toml                    |    9 +
 .../helper-census/examples/glossary_hook_probe.rs  |  224 +++
 crates/helper-census/src/glossary/mod.rs           | 1480 ++++++++++++++++++++
 crates/helper-census/src/glossary/pattern.rs       |  324 +++++
 crates/helper-census/src/glossary/po.rs            |  186 +++
 crates/helper-census/src/glossary/surface.rs       |  329 +++++
 crates/helper-census/src/main.rs                   |   13 +
 crates/helper-census/tests/glossary_callers.rs     |  172 +++
 docs/book/po/glossary-zh-Hans.md                   |   56 +-
 docs/book/po/zh-Hans.po                            |    2 +-
 layers.toml                                        |    2 +-
 scripts/check-gate-parity.py                       |    2 +-
 scripts/check-i18n-glossary.py                     |  717 ----------
 scripts/po_catalog.py                              |    4 +-
 19 files changed, 2808 insertions(+), 729 deletions(-)
```

## Conflicts

TODO: none, or how each was resolved and why.

---

Defect-Class: TODO
# One of the repo's named classes, or `none` for new work. This becomes
# a GhpRsq-Defect-Class trailer, and recurrence is COUNTED from those
# trailers - `prior-art.md` reports 'uncountable' for a repo that has
# none, which is the current state here. A class written once is worth
# more than a paragraph explaining the same thing three merges later.

