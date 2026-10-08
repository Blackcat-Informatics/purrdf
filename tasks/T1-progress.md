Task1 completed and pushed in97b05f758: original Rust visible-text glossary gate,
bounded existing regex reuse, compatible table/PO parsing, all K-row controls,
URL/title masking and actual catalogue sweep. Independent review caught multiline
inline URL leakage; corrected paragraph handling passed independent re-review.

Focused checks PASS:10 native tests, strict all-target clippy,1,716 production
controls,4,181 actual scan units and41 real-paragraph poison refusals, layer/fmt
and the still-wired legacy caller. Normal configured commit hooks passed.

Production Make/CI/staged-hook migration and obsolete Python retirement are next
in Task2. Full i18n/local qualification, hosted gates, PR and merge remain pending.
