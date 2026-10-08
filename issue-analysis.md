# Independent issue analysis

Status: intake requirements, not implementation or qualification.

Read the generated issue body (zero comments), prior-art and brief, independent
prior assessment, current glossary policy/table, Python glossary/PO readers,
Makefile, pre-commit snapshot and workspace CI callers. Root `.baseline` and
`.goals` and the supplied AGENTS instructions apply. No forge retrieval, build,
scan execution or source edit was performed. Only this Stage report is written.

The generated search covers four linked items, six related results and defect
trailers in the last 200 commits. It cites no governing ADR. The performance
trailer does not establish glossary recurrence. Historical zero collisions is
not evidence for the current 69-row glossary and expanded catalogue.

## Scope and executable acceptance

### 1. Anchored-paragraph substring collisions

Existing protections for 输出处理/provenance, 决定性能/determinism and
参考资料集合/global dataset are already present. Current neighbours primarily
test unrelated English anchors. Retain those controls and inspect every current
rendered catalogue unit whose English paragraph activates each rejection row.
Inventory the Chinese occurrences and surrounding visible text, classify actual
wrong renderings versus legitimate embedded neighbours, and record the sweep
counts and exact catalogue identity in Stage evidence. Do not assert a current
zero count without executing this sweep.

Every newly evidenced legitimate neighbour needs a narrow glossary exception
and a self-test under the SAME activating English paragraph. Pair it with a
still-refused own-term rendering. Retain generic unrelated-anchor tests and
cross-row rendering consistency. Do not exempt arbitrary embedded Chinese
substrings: that would also exempt genuine wrong renderings. If the sweep finds
an actual translation defect, correct its rendering instead of weakening the
rule. Poison a real anchored catalogue unit to prove the default gate refuses
wrong text, then restore it; changes used only for testing must not persist.

### 2. Anchors in link destinations

`Row.anchored_in` currently scans raw msgid, and K survival scans raw msgid and
msgstr. A destination-only term can activate a rule; a destination-only retained
name can also incorrectly satisfy K survival. Implement one explicit visible
Markdown surface used consistently for English anchors and both K sides. Preserve
visible link labels and inline code identifiers required by existing anchors
such as `outside purrdf-core`; rejection scanning still excludes code contents.
Invisible destinations, titles and reference definitions must neither activate
nor satisfy a term. A displayed autolink is visible text and must have a tested,
documented treatment distinct from a hidden destination.

Default CLI self-tests must cover destination-only negative controls and visible
label positives for inline/reference links, title text, escaped/nested destination
parentheses, images/alt text and visible autolinks. Include anchor and K cases
on both msgid and msgstr. Removing a whole link is not a valid fix because it
loses its label; removal must not concatenate unrelated fragments into a new
token. Preserve exact backtick-run semantics, and test multiline fences with
state retained across tracked-file lines. The present file_units/strip_code
combination resets state per line and can reject fenced GLOBAL examples.

### 3. Half-width Research Object typography

The concrete existing accepted form is `研究对象 (Research Object)` (not a new
translation). Row 40 explicitly accepts it and the production self-test already
proves it. General policy 5 requires full-width CJK punctuation without stating
this exception. Document a narrow English parenthetical-gloss exception in that
authoritative glossary policy, retaining the required preceding half-width space.
Keep both full-width accepted forms and reject bare 研究对象 under a Research
Object anchor as before. Do not reverse the specific row's accepted form or
broaden Chinese punctuation generally. Test policy and row agreement explicitly.
No live house-site claim is needed: this is reconciliation of current repository
policy, not verification of externally changing content.

### 4. Whole-token K survival, row by row

The current gate already generates drop/keep/no-token cases for every K row and
anchor token. Embedded-name controls are mostly RDF-specific. Extend the default
self-test matrix for EVERY actual K token: English source prefix/suffix collisions
must not activate it; translated prefix/suffix collisions must not satisfy an
activated requirement; exact case-sensitive survival must pass, and case near
misses must fail. Positive punctuation/CJK adjacency must pass according to the
existing ASCII `[A-Za-z0-9]` boundary rule. Derive fixtures from multiword and
hyphenated tokens too, rather than hardcoding RDF. A new K token must receive
this matrix automatically and failures must identify its row/token.

## Rust migration is part of this coherent delivery

Tooling and tests must be Rust under current repository law. Replace the Python
glossary gate and its callers with one host-only Rust production gate; do not add
a competing implementation, semantic feature or shipping/runtime dependency.
The independent prior assessment identifies the existing public
`purrdf_jsonschema::ecma` compiler/matcher as the shared pattern home. Reuse that
direct API with bounded matching and propagate exhaustion as hard failure. A
helper-census first-party edge and corresponding layer/generated-ledger updates
are appropriate; a new regex VM is not.

Preserve or explicitly reconcile the entire current behavior:

- Seven named table columns, required term/rendering, escaped pipes/backticks,
  non-pipe backslashes, list separators, GLOBAL reasons and K anchors.
- Literal substring rejection, prefix/case-insensitive anchors, case-sensitive
  ASCII-boundary K tokens and all current regex lookarounds/classes. Current
  Python Unicode `\\w`, `\\b`, `\\s`, case-fold and dot/newline semantics must
  be translated or explicitly documented and pinned before handing patterns to
  ECMA. Unsupported/malformed patterns and resource exhaustion hard-fail.
- PO continuation/contexts/header exclusion, declared escapes, fuzzy/obsolete/
  empty suppression, first-plural fallback and truthful source line diagnostics.
  External `--po`/`--glossary` paths remain supported. Preserve existing unknown
  escape spelling deliberately, rather than silently changing decoding.
- All existing self-tests execute before EVERY normal scan; missing rejection
  neighbours and the real standardized-spelling specimen remain hard failures.
  Migration unit tests alone do not replace this default CLI contract.
- Deterministic `git ls-files -z` tracked selection, glossary exclusion, current
  CJK ranges and 15-percent file-level selection; only GLOBAL rules apply to
  tracked translated Markdown lacking msgid. Preserve this actual file-level
  selection and fix fence continuity without falsely claiming line selection.

Update the glossary's Python-pattern/tool-path claims and catalogue gate pointer
to match the final implementation. Keep `po_catalog.py` for the independent
render gate; this migration does not authorize rewriting that unrelated surface.

## Integration and qualification requirements

Both current Makefile calls (`make check` hygiene and `check-i18n`), workspace CI
and staged-index pre-commit must invoke the SAME Rust gate. The hook currently
loops `python3 snapshot/scripts/<gate>.py`; removing its glossary list entry
without an explicit Rust snapshot invocation loses coverage. Compile the admitted
helper implementation and pass the staged snapshot root, preserving Git/index
selection and nonzero failure propagation. Test a deliberately poisoned staged
snapshot independently of clean working-tree text, plus inverse staged/working
cases, without bypassing hooks or creating a false verified commit.

The small coherent implementation should establish the shared parser/surface and
compatibility fixtures, port existing gate behavior and all four residual tests,
then replace all callers/documentation and remove the obsolete glossary script.
Focused Rust tests, default/self-test CLI scans, current anchored collision
inventory, external-path failures, snapshot poison controls, generated/layer
hygiene and normal required workflow gates must pass before completion. Actual
render-tool availability or live publication is separate from glossary acceptance;
no release, publication, broad book translation or unrelated i18n rewrite is in
this gate delivery.

All current collision counts and Rust/Python parity are NOT RUN in this intake.
Required behavior above is executable acceptance, not accepted residual risk.
