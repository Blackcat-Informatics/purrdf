# Issue #260: zh-Hans glossary gate: harden the anchored-substring residuals (reviewer R1–R4 leftovers)

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github
Labels: enhancement

## Body

PR #249 landed most of the second adversarial review's residuals, but the reviewer's list included cases worth a final pass now that real `msgstr`s are coming (issue on the pour is filed separately):

- Anchored substring collisions inside an anchored paragraph beyond the three lookarounds added (`输出处理` under provenance, `决定性能` under determinism were fixed; sweep for new ones as the glossary grows — the reviewer's method: scan every English paragraph containing an anchor for Chinese renderings that embed a rejected string).
- An anchor that appears only in a link URL of the `msgid` (should not anchor the row).
- `research 对象 (Research Object)` acceptance is house-typography-dependent; confirm the half-width-paren form matches policy or document the refusal.
- K-survival is whole-token since #249; add a self-test case per K row so a regression is caught row-by-row.

All are prevention — zero collisions existed in the book at review time. Small, self-contained, gate-only.

## Comments (0)

