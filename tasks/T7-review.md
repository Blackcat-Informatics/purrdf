# T7 review: computed decimal spelling, SECONDS, cli_rules, cdt pin

Reviewer: independent. Scope: the uncommitted diff over 85f014788 in the
worktree (43 files), plan "Task 7" with G1, G2, G9a, and `tasks/T7-triage.md`
including its addendum. No source was modified.

## Evidence run (all in the worktree, all green)

| Command | Result |
|---|---|
| `cargo test -p purrdf-shapes --test w3c12_conformance -- --nocapture` | `W3C12 TOTAL: passed 591, xfailed 0, ledger 0` |
| `cargo test -p purrdf-shapes --test w3c_conformance` | ok |
| `cargo test -p purrdf-xsd` | 661 passed, 0 failed |
| `cargo test -p purrdf-sparql-eval` | 3176 passed, 0 failed |
| `cargo test -p purrdf-sparql-conformance` | 245 passed, 0 failed (incl. W3C SPARQL suites, governor_corpus, cdt_lexical_divergence, evaluator-trace goldens) |
| `cargo test -p purrdf-cli --test shapes_tools_cli` / `--test query_cli` | ok / ok |
| `cargo test -p purrdf-shapes --no-fail-fast` | 2163 passed, 0 failed |
| `cargo clippy` (xsd, sparql-eval, shapes, sparql-conformance, cli; `--all-targets -D warnings`) | clean |
| `cargo check --workspace --all-targets` | clean |
| `helper-census --non-rust-ratchet --merge-base-with origin/main --target worktree` | OK, nothing legacy grew |
| `scripts/conformance-matrix.py --self-test`, `scripts/check-doc-claims.py --self-test` and plain run | OK (154 claims agree) |
| `scripts/check-corpus-frozen.py` | OK, 29 corpora byte-frozen |

Script line counts (git numstat): `check-doc-claims.py` +20/-43,
`conformance-matrix.py` +10/-21. Both shrink.

## (1) Do the texts fix a computed literal's lexical form?

Verified against the checkout at `4cd469867a1c`:

- `shacl12-node-expr/index.html:2776`: "For the following data graph, the sum
  will be `42.0`." (revenues 2.1, 37.8, 2.1). Confirmed; it is informative
  and spells the sum with a point, matching `sum-totalRevenue`.
- `shnex-sparql/seconds.ttl`: `sparql:seconds ( "2023-12-25T10:30:00"^^xsd:dateTime )`
  with `mf:result ( "00"^^xsd:decimal )`, status approved. Confirmed.
- SPARQL 1.2 Query is not in the checkout. From the quoted text and my own
  knowledge: the function/operator mapping maps typed values back to literals
  without choosing a lexical form; the numeric-function examples (`ROUND(2.5)`
  = `3.0`, `CEIL(10.5)` = `11.0`) show the point; `SECONDS` is defined as "the
  seconds part of the lexical form of arg" (SPARQL 1.1 wording, same example
  `13.815`). XSD 1.1 Part 2 does not require processors to emit the canonical
  mapping for computed results. I agree with the triage: no dated text forces a
  spelling incompatible with the approved one, so none of the six is
  `SPEC_CONTRADICTED`. `"00"` is a valid `xsd:decimal` lexical representation
  (XSD decimal lexical space allows leading zeros); it is legitimate as the
  seconds field of the lexical form, though not the output of any canonical
  mapping. The triage already says this. No objection to G1 outcome.

## (2) Casting vs computed split

The split is implemented coherently and in one home: `xsd_workspace_value`
(computed) versus `xsd_cast_to_term` / `governed_cast_to_term` (casts,
constructors, unary plus, pass-through), both over one `render_xsd_workspace_value`
with a `computed` flag, backed by `prepare_computed_numeric_text` and
`PreparedDecimalText::with_required_point` (the length is adjusted by 2 so
admission stays exact; unit-tested for exactness). Consequences are consistent:
`STR(1.0+3.0)` is `4.0`, `STR(xsd:decimal(4))` is `4`, `+x` keeps the stored
spelling, `-x` computes. This matches what Jena and RDF4J do for operators
(a computed decimal is spelled `4.0`), from my knowledge, not verified here. For
`CEIL`/`FLOOR`/`ROUND` and `SECONDS` I am less sure the mainstream engines
agree (Jena may print `3` for a scaled `BigDecimal`), but the W3C SPARQL suites
compare numerics by value and still pass, and the SPARQL spec examples use the
point.

User-visible oddities (not blocking by themselves):

- `SECONDS` now returns `05` for a dateTime whose seconds are `05`, while
  `MINUTES`/`HOURS` return integers and `SECONDS(x) + 0` returns `5.0`. It is
  what "seconds part of the lexical form" yields, but it is a surprise to a
  user and it is the only function whose result spelling depends on the
  argument's lexical form. The updated `querying.md` sentence states the new
  value (`05.1234...`) but not the rule.
- The casting-versus-computed rule is documented only in code comments
  (`expr.rs`, `numeric.rs`, `decimal.rs`). Nothing in `docs/book` or
  `docs/CONFORMANCE.md` tells a user that `xsd:decimal(4)` is `4` while `4.0`
  is the result of an operator. Recommend a short paragraph in
  `docs/book/src/sparql/querying.md` beside the exact-numerics text.
- `MIN`/`MAX`/`SAMPLE`/`COALESCE`/`IF` return stored terms and keep their
  spelling (correct, but DISTINCT/GROUP BY over `?a+?b` vs a stored `4` now
  see two terms). That is standard term-equality behavior.

## (3) Regeneration procedures

- Governor vectors: `governor_corpus.rs` documents
  `PURRDF_UPDATE_GOVERNOR_CORPUS=1 cargo test -p purrdf-sparql-conformance
  --test governor_corpus`, then `check-corpus-frozen.py --update`, then re-pin
  `GOVERNOR_CORPUS_DIGEST` from the manifest's `sha256sum`. Spot check:
  `sha256sum scripts/conformance-frozen/vectors-sparql-governors.sha256` =
  `ba1f32e7...3bb2e` = the pinned constant; `the_corpus_digest_is_a_well_formed_reproducible_sha256`
  and `check-corpus-frozen.py` pass. Only the two `temporal-exact-fuel-*`
  vectors changed, by the expected `5.123...` to `05.123...` and
  scratch-bytes 429 to 430.
- Evaluator-trace goldens: they pass in `purrdf-sparql-conformance`; the diff
  is `"3"` to `"3.0"` and the corresponding scratch-byte and digest moves. I
  could not find a separate regeneration command beyond the test's own env
  switch; they are byte-compared by the test, so they are consistent.
- Wasm numeric transcript digest `GOLDEN_DIGEST`: a hand-pinned constant
  compared by `assert_transcript_digest`; the native run passes. The wasm32
  run (`make wasm-test`) was not run here.
- No frozen W3C vector was edited: `git status` shows the only `vectors/`
  changes are the four first-party `vectors/sparql-governors/expected/` files.

## (4) W3C conformance suites

`cargo test -p purrdf-sparql-conformance` passes in full (245), and the SHACL
suites pass (591 of 591 approved; 2017 subset unchanged). The SPARQL harness's
by-value numeric comparison is why `ceil01`-style `"3"` expectations still pass.

## (5) Scripts and ratchet

Clean, see the table. The `conformance-matrix.py` regex and fixture edits and
the `check-doc-claims.py` pin edits all remove the `non-canonical` split
consistently; self-tests and the claim check agree with `docs/CONFORMANCE.md`
(591, 0, 0, GREEN).

## (6) cli_rules refusal

Correct. In the SPARQL 1.2 RL text (`sparql12-rl/index.html`, "Run-once rule",
line 962): a rule whose head has a triple-term template in which a variable
occurs is run-once. A "closed dependency" is a dependency of R1 on R2 where R1
is run-once. `nesting.srl` has body `?x ex:n ?y` and head `?x ex:n <<(...)>>`;
the body pattern can match the head template, so the rule depends on itself,
and because it is run-once the self-edge is closed, a cycle through a closed
edge, so not stratifiable. The new neighbour (`ex:m` head, same body) has no
body-to-head match, so it is accepted. The fixture is the same shape the
`stratification-bad-06`/`-07` entries use. Minor: the neighbour no longer
demonstrates that `--max-term-generating-rounds` reaches the SRL route (the
refused rule never runs a round, and the accepted one terminates anyway), so
the sentence "The knob reaches the SPARQL 1.2 RL route too" in the test comment
is now context, not coverage. Not blocking.

## (7) cdt pin 4122 to 4161

Substance holds: the pin is a count of tokenized tracked files under the scan;
the +39 matches the re-vendored `vectors/shacl12` file count in the triage
(442 to 481), and the test's substantive assertions are unchanged and pass. It
was re-derived from a run, as the comment on the constant requires.

## Findings

1. **BLOCKING: governor profile version not bumped.**
   `GOVERNOR_PROFILE_VERSION` (`crates/sparql-eval/src/governor/mod.rs:1870`,
   still 20) is documented as "Incremented by any change to what a given query
   costs ... any change that could move the point at which a caller's budget
   trips." This change moves a metered quantity: `temporal-exact-fuel-boundary`
   and `-over-bound` scratch-bytes 429 to 430 (a frozen, digest-pinned vector),
   and `suite.trace` scratch-bytes 5779 to 5781 (two extra characters per
   computed whole decimal), so every scratch ceiling trips at a different
   point. v18 set the precedent (scratch cut points moved, version bumped,
   "consumers must remeasure"), and the `governor_corpus.rs` header says a
   regeneration that moved a charge is a profile-version change. The diff
   regenerates the corpus and re-pins `GOVERNOR_CORPUS_DIGEST` but leaves the
   version, its `# v21` note, `docs/SPARQL-GOVERNOR-PROFILE.md` and
   `GOVERNOR_PROFILE_DIGEST` (if it covers the version) untouched. Either bump
   to 21 with a note ("the point schedule stays unchanged; a computed whole
   decimal is two characters longer and `SECONDS` keeps its leading zero, so
   scratch cut points move") and update the dependents, or record why scratch
   bytes are exempt. Fuel in the vectors did not move.
2. Non-blocking: document the cast-versus-computed rule and the `SECONDS`
   rule in `docs/book/src/sparql/querying.md` (see section 2).
3. Non-blocking: `docs/book/po/zh-Hans.po` still carries the old SHACL 1.2
   scoreboard prose (`passed 538, non-canonical-expected-decimal 6`) for
   `validation/shacl.md`; it was already stale before this change (538), so
   this is pre-existing drift, but the English prose edit widens it.
4. Non-blocking: the wasm32 digest was not exercised here.

VERDICT: BLOCKED
## Recheck (orchestrator)
BLOCKING (GOVERNOR_PROFILE_VERSION) resolved: version 21 with v21 note; docs/SPARQL-GOVERNOR-PROFILE.md updated; governor unit/corpus/trace tests and check-doc-claims pass per the implementer's rerun and the greps above. Non-blocking notes 2 (book docs) and 3 (CLI flag coverage) addressed.
VERDICT: PASS
