# Independent scoped correction review — 2026-10-08

VERDICT: PASS for the two-file correction and its focused qualification.
Corrected full qualification/rendering/completion remain NOT MET by this review.
No build, test, shipping-source edit, index or forge action was performed here.

Read tasks/T3-correction.md, the exact source diff and actual retained logs.
Reviewed source hashes:

- glossary/surface.rs SHA256
  `7294741f9485508114c81444bf51955f7d6bdaa7532fcfe4700f6e33c3827ce1`.
- tests/glossary_callers.rs SHA256
  `61b02a4786f234ee9404a63de170ef7cba66b6d60b5a0da9e5a1cfeb95a6926e`.

The whitespace correction is justified by the actual clauses, not a global
CommonMark whitespace label. [CommonMark 0.31.2 §6.3](https://spec.commonmark.org/0.31.2/#links)
limits inline destination/title separation to space/tab and up to one line ending;
form feed is excluded. [§4.5](https://spec.commonmark.org/0.31.2/#fenced-code-blocks)
limits closing-fence trailing content to space/tab. The projector retains the
physical CR/LF terminator in its line slice, so the existing shared four-byte
`purrdf_lex::terminals::is_ws` matches this particular suffix check. It does not
use the broader Unicode whitespace class, add a duplicate predicate or introduce
a census exemption. Structural link/fence handling remains the bounded surface
projection; this review does not claim a general CommonMark parser.

The new regression distinguishes FF and ordinary space before quoted title text:
the parentheses inside the quote are balanced only when it is a legitimate title
separator. The fence regression keeps following forbidden prose masked after an
FF pseudo-closer, then exposes prose after the actual closer. Existing exact-run,
shorter-fence, tilde and multiline/offset neighbors remain exercised. Source
length/line-preservation assertions and caller/parity assertions are retained.

The local fixture `write` wrapper duplicated directory/file writing. Fixed
fixture directories are now created once and actual std::fs::write calls preserve
the same content and mutating/parity controls. No alternate helper, relaxed gate,
test skip or lost external-input/renamed-Markdown assertion was introduced.

Actual qualification receipts inspected:

- Session 44571 exited 0: T3-correction-focused-settled.log contains eleven passing
  glossary tests and one filtered passing native production parity test.
- Session 86961 exited 0, confirmed by root's qualifier terminal: both external
  input/renamed-Markdown and parity caller tests passed 2/2 in
  T3-correction-callers.log; strict all-target helper clippy passed in
  T3-correction-clippy.log; live helper census passed 81 jobs, 23 variants,
  91 distinct groups and 1,879 files in T3-correction-helpers.log.

The earlier marker-less private-build attempt remains failed evidence, not a
pass; the settled attempt used Cargo's actual cache-root markers. Original full
gate failed before this correction. Corrected full session 9281 was intentionally
aborted at crossed hold, actual exit 143: neither pass nor defect failure. It does
not replace a settled corrected full run. Root will commit through normal hooks
and qualify the unchanged corrected patch/full render before claiming completion.

No blocking source or focused-qualification finding remains for this correction.
