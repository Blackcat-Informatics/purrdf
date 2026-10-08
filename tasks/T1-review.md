# Independent Task 1 review

**PASS after correction.** Independently re-reviewed settled source, canonical `tasks/T1-implementation.md`, validation index, actual focused/self-test/clippy/sweep logs and regenerated inventory. No builds, scans, source edits, Git mutations or forge actions were performed by this review. The initial review and its resolution are retained below.

## Independent correction verification

The correction first identifies blank-line, fence and reference-definition boundaries, then scans inline syntax across each paragraph segment. Labels, destinations and titles now balance across soft line breaks. Previously hidden bytes are skipped during subsequent scanning, so brackets/backticks within a masked destination cannot consume outside prose. Byte/newline offsets remain exact; labels and visible code identifiers remain while rejection code/fences stay masked. Unterminated inline syntax cannot hide the next blank-separated paragraph.

The new focused fixture covers multiline links/images/labels/titles, escaped destination parentheses, hidden title delimiters, literal fenced URLs, exact offsets and unclosed-link block boundaries. Production controls cover source/translated K survival through multiline links/images, visible label positives and true outside-link obligations, alongside hidden anchor/rejection controls. The previously failing URL-only shape is handled by the corrected source and production controls. No documentation scope cut substituted for the fix; no new regression was found in the corrected projection.

Actual rerun logs show ten focused native tests, warning-free all-target clippy, 1,716 default production controls and the 4,181-unit native scan. Regenerated inventory retains forty-one applicable real-paragraph poison controls and zero current anchored rejected/raw-specimen occurrences. Knowledge Graph and Research Object still honestly have no active anchored PO paragraph. Old caller, layer/fmt and source-dependent checks were rerun after correction; they are distinguished from full qualification.

No blocking Task 1 finding remains. This approval permits normal hook/commit handoff within the accepted plan; it does not claim completed Make/CI/staged-hook migration, external-input/index inversions, hosted gates, full i18n/local qualification, PR or merge. Tasks 2–3 still own those criteria. Everything below records the initial review before the correction, not the current disposition.

## Historical initial finding: multiline inline destinations (resolved above)

`crates/helper-census/src/glossary/surface.rs:127-152` projects each line separately and passes `source[..end]` to both label and destination balancing. A valid inline link such as:

```text
[ordinary](
https://example.invalid/provenance
)
```

cannot find its closing parenthesis on the opening line. The next line retains the destination in `visible`, so the provenance row still activates on a term that appears only in an invisible URL. The same shape with `RDF` can incorrectly demand or satisfy K survival. Inline image destinations and multiline labels/titles have the same structural hole. The explicit bounded-surface documentation excludes arbitrary HTML and multiline reference definitions; it does not exclude these ordinary balanced inline links. More importantly, the issue's URL-only anchor criterion has no single-line exception.

Correct balanced inline projection across the relevant document/paragraph span, preserving byte/newline offsets and labels, while retaining code/fence masking and preventing a scan inside a previously hidden destination from treating its contents as visible Markdown. Add production-path negative controls for URL-only anchors and source/translated K tokens in multiline links/images/titles; add visible-label positives and true outside-link refusals. Re-run the focused source-dependent checks, default controls and actual sweep after the correction. Do not solve this by documenting a new URL-only exemption or erasing the whole link label.

## Reviewed behavior otherwise supported

The existing bounded ECMA matcher is reused directly, with typed exhaustion propagated rather than converted to no-match. Current pattern classes/lookarounds have explicit compatibility adaptation and boundary fixtures; no second matcher or external/shipping dependency was added. The only dependency/lock/layer delta is the unpublished helper's first-party JSON Schema edge. PO handling preserves continuation/escape/context and first-plural behavior, rejects malformed inputs and suppresses fuzzy/obsolete/empty units. Table parsing, row-derived production controls, isolated K obligations, specific Research Object forms and typed source diagnostics are coherent.

The retained sweep is actual generated evidence, with per-row anchored counts and forty-one applicable poison controls. It reports zero current anchored rejected/raw-specimen occurrences and honestly records no active Knowledge Graph/Research Object paragraphs; historical zero counts are not substituted for execution. No broad rejection exemption or catalogue translation change was made. Nine reported focused tests and 1,214 default controls are useful evidence but presently miss the multiline case above.

Old Python Make/CI/hook callers deliberately remain for Task 2. This is not a premature completion claim; production migration, external paths/staged snapshot inversions, settled full gates, hosted qualification and integration remain outstanding. Task 1 may proceed to normal commit/handoff only after its blocking visible-surface correction is independently rechecked.
