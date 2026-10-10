<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Combined independent intake, plan and applied-contract review: #521 / #520

Status: PASS for the complete applied Task1–3 source contracts at the final
correction identity. Neither #521 nor #520 has an identified remaining source
contract gap. No issue closure evidence is claimed;
full required gate, final-head hosted acceptance and protected integration are
NOT MET. This is a source review with previously recorded evidence, not a new
test run. Source findings below were sent together for one coherent correction.

Original review 2026-10-09T19:36:45-06:00; correction disposition updated
2026-10-09T19:55:35-06:00, then final correction-family4 disposition on2026-10-09
from the same substantive review. Worktree HEAD is
`d608b0584e11cb803347ab1edc09c90d2dadbada`; implementation is uncommitted.
The source view includes the new six-message corpus, including the unchanged
public CPython delivered-message and Mailman-digest originals. It does not claim
the earlier four-message transcript validates that expanded corpus. The actual
expanded six-message Rust transcript and native/WASM execution are now proved
in `correction-family-3.log`. The final three lexical/healthy-neighbor edges
below are now source-addressed and proved by correction-family4. Task4 remains
pending the actual full gate and publication/integration.

## Intake, prior art and design disposition

Read the complete captured issue #521 (`issue.md`, `raw/issue.json`) and #520
(`mime-issue.json`); both captured comment surfaces are empty. Read `prior-art.md`,
the whole `plan.md`, MIME `SPEC.md`, all MIME production modules, core cover/emitter
modules, native cover/MIME fixtures, the shared corpus/transcript producer and
workspace/layer/facade/portable registration. No fresh forge fetch was needed.

The prior-art crawl's missing ADRs are a search-location limitation. The plan
names the real sibling Katamari ADRs. ADR-0022 retains original bytes and makes
repair additive; ADR-0029 29.22 binds windows to literal digest/span/space. Those
decisions support digest-bound byte references and prohibit silently repairing
MIME. They do not require importing a consumer vocabulary or a repair engine.
The original core reconstruction walk, XSD binary/hex decoders, native RDF IR,
fixed hashing, lexical search and existing JSON/Markdown occurrence laws are
reused at their existing homes. No external runtime dependency or semantic
feature is introduced. All new public APIs document their explicit selection and
refusal behavior. The proposed grouped boundary covers both complete issues;
the plan's full scope is appropriate. The original defect-recognition findings
have been corrected at their production homes, without a scope cut.

## Complete requirement map

| Contract | Applied production path and assessment |
| --- | --- |
| #521 opaque identity, every octet and empty source | `ByteCover::identity` borrows one original whole-source span and hashes the bytes. `reconstruct_bytes` uses the same `reconstruct_over` as the original UTF-8 API. Native/actual WASM every-octet fixtures pass in recorded evidence. |
| #521 format-neutral emit/continuation law | `CoverBuilder::emit/finish` uses the original typed gap, overlap, earlier-parent and digest law; no inferred continuation or format recognition. DelimitedCover preserves repeated/empty records and separator windows through that emitter. |
| #521 reusable references | `SpanReference` carries source digest and exclusive-end byte coordinates, proves digest before slicing, checks ranges and 32-bit narrowing. Empty/mismatching/out-of-range neighbors are covered. Byte addressing satisfies the issue's alternative to code-point addressing. |
| #521 determinism and old callers | Fixed digest, total byte-span ordering, one original decode body. Legacy core cover7/7, JSON23/23, Markdown9/9 and actual WASM cover5/5 have recorded passes. |
| #520 exact cover and verified reparse | All original physical lines emit unchanged opaque spans; empty input is lawful. Projection validates mutable model fields against a fresh original analysis. Decode reconstructs, verifies profile/content/document identity and compares the regenerated selected RDF facts before returning bytes. No identified escape in that selected-quad verification. |
| #520 ordered/queryable headers | Distinct occurrence IRIs, original ordinals, field-name ranges, ordered continuation segments and exact unfolded bytes; actual SPARQL fixture sees two equal Received occurrences. |
| #520 nested parts/messages and structure | Flat part indices and iterative jobs preserve absolute source ranges, parent/child ordinals, message/rfc822 and multipart/digest defaults. Preamble/delimiter/epilogue ranges retain the CRLF owned by the parent delimiter. Outer scanning bounds children before inner scanning, so a truncated inner multipart cannot consume an outer delimiter. Revised source observes close-first and invalid declared-prefix lines. Final correction leaves unrelated preamble lines opaque, with an exact native/WASM healthy neighbor. |
| #520 decoded attachment identity | Covered body bytes pass the shared strict base64/hex homes or the MIME QP body. Original filenames/source IDs are excluded from the decoded content digest; abc has a frozen SHA256 equality fixture across base64/QP. No charset guessing. Revised source shares identity-transfer validation across leaf and container, with exact healthy binary/8bit neighbors and original bytes preserved. |
| #520 malformed as found | Header LF/non-ASCII/invalid-name/orphan-fold, missing separator, structural duplicate/conflict, missing boundary/closure and bad base64/QP are modeled as typed occurrences. The three original families and their three remaining lexical/healthy-neighbor edges are now source-addressed with exact native/WASM16 passes. Syntax defects remain typed observations; no source bytes are normalized or discarded. |
| #520 explicit profile/vocabulary and limits | Every entry requires the selected Profile; no implicit namespace. All five resource maxima are explicitly optional. Flat lifetime/destruction avoids a recursive depth limit. Recorded >16MiB,65,537-header,depth257 controls and refusal neighbors pass. Syntax violations must become problems, never new resource ceilings. |
| #520 real/adversarial, native/portable byte identity | Six originals share the same production analyze→project→Turtle/NTriples/JSONLD→parse→decode transcript. Two public CPython files are pinned, byte-frozen and retain upstream LF. The Rust producer regenerated the six-original frozen transcript; actual native/WASM16 cases pass including exact transcript, Mailman15part/4rootchildren and graph mutations. |

## Original MIME findings

These source observations describe the original review identity. Their current
dispositions are recorded in the correction recheck below; historical line
numbers and failures do not describe the corrected source.

These are source-proven findings, not runtime reproductions. Keep exact original
bytes and typed problem occurrences. Do not repair them, guess boundaries/charsets,
replace them with a refusal to cover the source or add caller-resource defaults.

### 1. Identity-transfer and contextual line defects are unobserved

`parse.rs:382` checks the container's allowed transfer-name family but skips its
actual byte validation. The leaf-only decode check at395 misses container
non-ASCII under default/declared7bit. `transfer.rs:15` tests only high-bit bytes;
7bit NUL/bare CR/bare LF and all 8bit line defects are accepted without a problem.
Header and preceding-delimiter LF observations do not cover those body cases.
First/closing delimiter line endings also need contextual observation.

The wire distinction is exact: RFC2045's 7bit/8bit domains disallow NUL and lone
CR/LF;7bit additionally excludes high octets. Binary permits arbitrary octets.
Protocol line syntax differs from resource admission: record a malformed textual
or constrained-domain line while retaining it; never reject a long source merely
because an RFC line-production length is exceeded. [RFC2045 §2.7–2.10](https://www.rfc-editor.org/rfc/rfc2045.html#section-2.7)

Focused missing neighbors: multipart preamble/epilogue containing `\xff` under
default7bit; message/rfc822 default7bit around an explicitly8bit child; leaf7bit
`a\nb`, `a\rb`, and NUL;8bit bare LF/NUL; first/closing delimiter with LF.
Application/octet-stream+binary containing the same LF/NUL/high octets must remain
legal opaque payload. Cover reconstruction must remain byte-exact throughout.

### 2. Multipart grammar and delimiter anomalies are incomplete

`parse.rs:418` validates only empty/CR/LF/conflicting parameter values. It accepts
non-ASCII, forbidden boundary characters, >70-byte values and trailing space as
normal declarations. At494 a closing delimiter before any opening delimiter
returns zero children and no typed problem. At503 prefix candidates are only
noticed before the first recognized delimiter; an invalid declared-prefix line
inside an opened part can be silently treated as ordinary body.

RFC2046 defines a 1..70-character boundary from its specified alphabet with a
nonspace final character. Multipart requires an opening part and a closing
delimiter; a body line beginning with the declared delimiter cannot be an
unremarked part payload. These are grammar observations, not source-size limits.
[RFC2046 §5.1–5.1.1](https://www.rfc-editor.org/rfc/rfc2046.html#section-5.1.1)

Missing neighbors: quoted `boundary="x "`, a71-byte boundary, illegal/non-ASCII
boundary and close-first `--x--`. Compare with valid70-byte boundary, quoted
allowed punctuation/interior space, valid transport padding, valid nested digest
and existing parent-CRLF ownership. A line `--xBAD` inside an opened `boundary=x`
part needs a typed anomaly without guessing a new delimiter or inventing a split.
Unrelated `--other` in arbitrary preamble/body/epilogue is not by itself evidence
of an intended boundary; do not manufacture such a judgment.

### 3. Encoded-word validation needs lexical and header-context law

`transfer.rs:113` scans any `=?` substring, accepts charset punctuation excluded
by its token grammar and has no whole-word length/context/adjacency rule.
`parse.rs:367` calls it on every unfolded field with no field/context identity.
Consequently `Subject: =?ut:f-8?Q?a?=` and overlong apparent words can have no typed
problem; `Subject: math x=? is ordinary` can acquire a false InvalidEncodedWord.

RFC2047 specifies token spelling, a75-character whole-word syntax, permitted
text/comment/phrase positions and adjacency. Received, addr-spec, quoted-string
and MIME parameters are not encoded-word positions. Charset interpretation is
separate and need not be guessed. [RFC2047 §§2,5–6](https://www.rfc-editor.org/rfc/rfc2047.html#section-2)

Fix the existing checker/caller once. Preserve B/Q case insensitivity, integral
encoding, Q underscore and supported valid folds. Test token/75-character
neighbors and relevant contexts. Literal ASCII parameter/address strings must
retain literal semantics: `boundary="=?utf-8?Q?a?="` is not permission to decode
or reject a declared literal boundary. An unknown charset with lawful lexical
syntax is not a guessed charset or automatically broken transfer syntax.

## Selected document graph verification

`decode.rs:one` requires one correctly typed default-graph scalar; canonical
integer and hex spellings are checked. The content digest and Profile identity
are proved before publishing any result. The derived document identity binds
original source IRI/profile/content. Span payloads go through the core decoder.
`selected` includes every statement whose subject is the selected document or
its occurrence namespace, plus explicit membership claims. It rejects foreign
members and named-graph selected statements, then compares self-delimiting
canonical term encodings against the full regenerated projection. The encoding
home frames each term and handles RDF1.2 term identity; concatenation boundaries
cannot slide. Missing/extra/conflicting selected facts fail the comparison.
Selected-namespace reifier/annotation extensions are explicitly rejected.
Unrelated document namespaces and independent judgments remain outside this
closed representation, as intended. No raw asserted span metadata can authorize
an unverified occurrence. Six recorded graph mutation controls cover missing
header, ordinal, payload, source digest, named graph and invented predicate.

## Coherent correction recheck history

The following partial disposition records the source before final correction4.
All three remaining edges are resolved in the final disposition below.

Read the revised original `parse.rs`, `transfer.rs`, typed problem registry and
the three new family fixtures. Rechecked primary wire clauses, not just test
counts. No suite, source probe, forge refresh or shipping edit was performed.

Identity-transfer family: ADDRESSED. One `validate_identity` body checks NUL,
7bit high octets, bare CR/LF and998-octet syntax for both leaves and containers.
The caller publishes typed defects and exact source remains reconstructible.
Binary bytes remain opaque; only recognized delimiter line endings create its
contextual LF observation. The original transfer decoder refuses malformed
identity payload rather than normalizing it. The native/WASM family fixture
covers both malformed and healthy neighboring declarations.

Boundary family: MOSTLY ADDRESSED, one healthy-neighbor defect remains.
Boundary1..70 alphabet/final-nonspace syntax is checked without a caller cap.
Close-first creates `MissingOpeningBoundary`; invalid declared-prefix suffix
inside an opened part creates `UnexpectedBoundary`. However,
`multipart_children` still tests
`marker.is_some() || (!found && payload.starts_with(b"--"))`. Therefore a valid
unrelated `--other` preamble before the real declared `--x` delimiter acquires
an invented `UnexpectedBoundary`. A preamble is arbitrary discard-text;
unrelated prefix spelling cannot authorize an inferred intended delimiter.
Observe the actual declared prefix only. Preserve the existing missing-boundary
and bad-declared-suffix refusals. This was an explicit healthy neighbor in the
original review, not a new conformance scope. [RFC2046 §5.1.1](https://www.rfc-editor.org/rfc/rfc2046.html#section-5.1.1)

Encoded-word family: MOSTLY ADDRESSED, two exact lexical edges remain. The
original scanner now carries field/context identity, skips Received, quoted
strings, addr-spec and literal MIME parameters, checks75/76 syntax, integral B/Q
payloads and phrase/comment Q alphabets, and preserves an ordinary `x=?` token.
The charset especials list in `encoded_word_valid` nevertheless omits backslash;
`Subject: =?ut\\f-8?Q?a?=` is accepted as valid by this source. Add that exact
excluded byte at the same token predicate. The RFC rendered text drops this
backslash; its raw grammar excludes it. [RFC2047 §2](https://www.rfc-editor.org/rfc/rfc2047.txt)

The scanner also explicitly accepts preceding comma/semicolon/colon and following
`<`/comma/semicolon/colon for a phrase with no intervening linear whitespace.
Consequently `From: =?utf-8?Q?a?=<sender@example.org>` and
`From: group:=?utf-8?Q?a?= <sender@example.org>;` have no InvalidEncodedWord
despite the phrase adjacency rule. Recognition at a syntactic phrase boundary
must still observe missing required whitespace. Preserve lawful comment-edge
positions and untouched addr-spec/quoted/MIME-parameter literals.
[RFC2047 §5(3)](https://www.rfc-editor.org/rfc/rfc2047.html#section-5)

All three observations followed that source computation; they were not runtime
reproduced by this reviewer. They were sent together and corrected as one small
production-home change, described next.

## Final correction4 disposition

Read the revised exact predicates and new neighbor fixtures. The charset token
now excludes backslash at its original predicate. Phrase candidate recognition
still notices the syntactic special boundary so missing whitespace can be
diagnosed; preceding and following separation require actual linear whitespace,
while comment-parenthesis positions and literal contexts retain their laws.
The multipart producer now observes only a matching declared prefix, preserving
unrelated `--other` preamble text. Healthy group-with-whitespace and complete
unrelated-preamble/real-boundary round trips accompany the three exact malformed
neighbors. None adds a resource ceiling, normalizes bytes, guesses a charset or
replaces the original decoder/scanner. All original findings are ADDRESSED.

`correction-family-4.log` proves affected all-target Clippy PASS, MIME16/16 native
PASS and actual WASM16/16 PASS, including those neighbors and the unchanged
six-original transcript. No suite was run in this reviewer lane. Task1, Task2
and Task3 VERDICT: PASS. Separate task artifacts project this same review; they
are not new reviewers or a repeated review panel. Task4 remains pending.

## Evidence and final gate state

- `correction-family-4.log`: actual affected Clippy PASS, native MIME16/16 PASS
  (0.52s) and actual WASM MIME16/16 PASS(0.99s). This is the final reviewed source
  identity and includes the three exact correction neighbors. The expanded
  six-original frozen transcript remains unchanged and passes.
- `correction-family-3.log`: terminal success/status0. Affected all-target
  Clippy PASS, core5/5 PASS, MIME16/16 native and actual MIME16/16 WASM PASS.
  The Rust producer generated the expanded six-original transcript
  `104f6e43fabc7234f9c27be2800879148b5daffcec7cfaf0d4ed7c459786e281`.
  The transcript comparison and actual Mailman15part/4rootchild controls pass.
  Its three remaining source edges are covered by correction-family4, above.

- `complete-matrix-8.log`: terminal native core/MIME all-target Clippy, core5/5,
  MIME13/13 PASS at that source.
- `legacy-portable-matrix-1.log`: terminal success; legacy cover7/7,
  JSON23/23,Markdown9/9 and actual WASM cover5/5,MIME13/13 PASS.
- `frozen-corpus-1.log`: historical terminal success on the earlier four-original
  transcript. The expanded proof is now in correction-family3, above.
- `metadata-hygiene-1.log`: terminal FAILURE, including stale crate prose and
  the vocabulary-standard shim census family. `metadata-2.log`: terminal FAILURE,
  documented release ordinal24 unsupported by the existing spelling table.
  Written source changes cannot retroactively turn these into passes; a current
  settled collector is required. `validation.md` still calls correction-family3
  running and must be reconciled with its actual terminal result.
- No full makecheck pass, normal-hook source publication, final-head hosted
  acceptance, protected merge, issue closure or archive/cleanup evidence is
  available for this group. Those gates remain NOT MET.

Reviewed source fingerprints (SHA256):

| Source | Identity |
| --- | --- |
| core `cover.rs` | `3eaf8fbbbbe482284c0aa0e3d76047d03f6aacc312f330e39619824fb14026ef` |
| core `cover/emit.rs` | `01766999682b84248850af15c3a3583786730707382784ae6be3aa6569754d6e` |
| MIME `parse.rs` (final correction4) | `0d56d030f4d3f802a58af242cf4421e772b383770f9fe82a3b0e9f5f9d3da2ab` |
| MIME `transfer.rs` (final correction4) | `5b4033dee0fdfff678a4f2088401bb230f4753f0a56e68d2c7c047a4916fa743` |
| MIME family and original-law fixtures | `e71c0ffb144828c80deacdc7e59032f1492c2bf4ecffd576387395cea9700197` |
| MIME `decode.rs` | `ecfc4f6d15bb44852b73730ece07a789f931c040b307594b94f6d6056e95134f` |
| shared six-original corpus | `bd5410e231ddcc58758095356ce7611ce4ac7e243fc5c146d7a6d150713d9393` |

The same correction review is complete and PASS for Tasks1–3. All original
findings are addressed with affected native/portable proof and the six-message
transcript. Current settled metadata/hygiene and required full gate are separate
from that evidence and remain pending at this disposition. Task4 publication,
hosted acceptance, protected integration and issue closure remain NOT MET.
Neither issue counts as closed early.

## Documentation applicability delta

The final staged `docs/book/po/zh-Hans.po` delta changes exactly the two release
book messages from the 31-crate release set to 32, with the corresponding Chinese
counts changed to 32. They match the actual English `project/releases.md` source;
no unrelated translation or acceptance clause is altered. The parent reports
`msgfmt` passing. The real rendered `check-i18n` gate is queued after the unchanged
Rust full gate and is still pending; catalog syntax is not rendered acceptance.

The additional `.gitattributes` notice entry names only
`crates/mime/licenses/NOTICE.txt` and disables only `blank-at-eof` whitespace
checking. It preserves the license writer's final empty section separator and
keeps the notice's text diff visible. The existing captured-public-mail rule
likewise preserves original payload bytes while retaining visible diffs. Neither
entry hides source changes or disables a required check. These two paths do not
change runtime implementation or the native/portable source judgments above.
They require their actual documentation/hygiene gates, without another runtime
suite or independent review panel.

## Settled mandatory qualification

`full-check-1.log` is now TERMINAL PASS, including the complete native tests,
doctests, consumers, ring-fence/hygiene checks and release WASM build of all
32 publishable crates. `book-final-1.log` is also TERMINAL PASS: the original
`check-i18n` glossary and every poisoned-catalog control, six real rendering
gates, all 3,040 messages translated with zero fuzzy/untranslated/template
drift, and both English and Chinese HTML builds. `validation.md` records their
actual completed results. These actual receipts supersede the pending full and
rendered-book statuses above; failed/terminated historical runs remain historical.

The complete unchanged runtime source and the reviewed final two-path docs
delta are therefore qualified by their required actual gates. No additional
suite or substantive review panel was run by this reviewer. Tasks1–3 remain
PASS. Task4's qualification component now passes, but normal source publication,
final-head hosted checks, protected integration and both verified issue closures
remain pending; Task4 is not DONE and neither issue is counted closed early.

The first normal source commit attempt did not create a commit: the real Stage
Git guard refused an inherited nonempty token-assignment example in
`docs/RELEASE.md`. The narrow reviewed docs correction requires the caller's
already exported `CARGO_REGISTRY_TOKEN`, checks its presence through the colon
parameter check, and keeps the same crate/version yank command. It puts no token
on argv and supplies no token assignment/example value. It changes no runtime
behavior and lies outside the translated book tree. The focused existing doc
claim gate and ordinary commit retry still require their actual terminal results;
the hook was not bypassed and the failed attempt is not a successful commit.

## Published-head applicability

The focused release documentation claim gate completed with all 154 checks
passing. The normal signed source commit and push then passed at
`9e6c6af64b45027151952e1fd9eb86bc17282339`; actual remote PR524 is open.
The safe exported-token correction changes only the reviewed release example,
so the complete runtime judgment and actual full1/book receipts still apply.
No hook was bypassed. The command's PR-response parse error did not undo the
remotely created PR, and no duplicate creation is needed.

The distinct captured-feedback audit is `review-debt.md`: no submitted review
or thread, complete false pagination, and one administrative CodeRabbit rate-limit
notice. The current successful bot status is not a substantive bot review.
Fresh required hosted CI has four actual SIMD configuration failures and other
required jobs pending; the root is diagnosing them. This supersedes the earlier
pending-publication status only. Tasks1–3 and local qualification remain PASS;
Task4 is not DONE, hosted acceptance and protected integration are NOT MET,
and neither issue has been verified closed.

The subsequent reviewed publication delta is normal signed/pushed
`81caed9159638db765190f0a489ddcf0785de86c`: one handwritten no-benchmark
`mime.crate` audit row in `docs/design/purrdf-simd.md`. It satisfies the original
workspace crate-coverage criterion before measurement and registers no invented
hot path or measured cells. No runtime source, threshold, generated manifest or
measurement changes. The root reports the original coverage self-test PASS.
The prior complete runtime/full1/book judgments remain applicable, while the
corrected-head required hosted run is pending. Historical failed checks are not
retroactively passes; current hosted qualification, protected integration and
Task4 completion remain NOT MET as recorded in the same `review-debt.md`.

## Final current-head and candidate applicability

The final distinct `review-debt.md` now has `VERDICT: PASS` against published
head `81caed9159638db765190f0a489ddcf0785de86c`. The refreshed Stagectl checks
actually contain 48 SUCCESS and 5 expected conditional SKIPPED, with zero failed
or pending checks and state pass. A fresh successful native GraphQL capture
confirms that head and base `37e3a26a71fe74d30ecb79de5291161a2f8919b5`, with
zero reviews or threads and complete false pagination. Both complete current
comments are accounted for; the spending-cap notice is administrative and is
not misrepresented as a substantive bot source review.

This same complete source/contract judgment and the actual full1/book/affected
controls apply to the root's clean candidate
`2448b668cc84e9626b59b2e63b5ff0c7a8fc1f67`. The intervening base changes
DatasetView/BGP/retrieval consumers, with no change to the cover, lexical/hash/
XSD homes or MIME implementation. Current passing hosted test-merge controls
now qualify the combined query consumer, alongside the original MIME SPARQL
occurrence-order control. The sole follow-through coverage row changes no
runtime or measured thresholds. No new panel, suite or branch synchronization
was run by this reviewer. Earlier pending and failed records are historical,
not overwritten results.

Task4's publication, review-debt and hosted-qualification components now pass.
Actual protected integration, both verified closures, selected evidence archive
verification and cleanup still require their real root/tool results. Neither
issue is counted completed by this pre-merge judgment.
