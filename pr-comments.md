# PR #524 — comments and review threads

3 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- This is an auto-generated comment: rate limited by coderabbit.ai -->

> [!WARNING]
> ## Review limit reached
> 
> Your organization has reached its usage spending cap. Adjust your spending cap in the [billing tab](https://app.coderabbit.ai/settings/billing?tab=usage&orgId=b268aba6-a194-4fd6-8ca5-f1a13a313209).
> 
> **Next included review available in 23 minutes.**
> 
> [Check out review usage here](https://app.coderabbit.ai/dashboard/review-capacity?orgId=b268aba6-a194-4fd6-8ca5-f1a13a313209).
> 
> <details>
> <summary><strong>View limit details</strong></summary>
> <dl>
> <dd>
> 
> **Limit details:** You’ve used the included review currently available. Your 111 included PR review attempts over the past 7 days set your current allowance at 1 review per hour.
> 
> [Learn how review limits work](https://docs.coderabbit.ai/management/plans#rate-limits).
> 
> **Review configuration:**
> 
> <details>
> <summary><strong>⚙️ Run configuration</strong></summary>
> <dl>
> <dd>
> 
> - **Configuration used**: defaults
> - **Review profile**: CHILL
> - **Plan**: Team
> - **Run ID**: `5cb2f4e8-0073-4bb2-bb38-a78f8b73ad36`
> 
> 
> <hr>
> 
> </dd>
> </dl>
> </details>
> 
> <details>
> <summary><strong>📥 Commits</strong></summary>
> <dl>
> <dd>
> 
> Reviewing files that changed from the base of the PR and between 37e3a26a71fe74d30ecb79de5291161a2f8919b5 and 81caed9159638db765190f0a489ddcf0785de86c.
> 
> 
> <hr>
> 
> </dd>
> </dl>
> </details>
> 
> <details>
> <summary><strong>⛔ Files ignored due to path filters (1)</strong></summary>
> <dl>
> <dd>
> 
> * `Cargo.lock` is excluded by `!**/*.lock`
> 
> 
> <hr>
> 
> </dd>
> </dl>
> </details>
> 
> <details>
> <summary><strong>📒 Files selected for processing (49)</strong></summary>
> <dl>
> <dd>
> 
> * `.gitattributes`
> * `.github/workflows/ci.yaml`
> * `AGENTS.md`
> * `Cargo.toml`
> * `Makefile`
> * `crates/hash/README.md`
> * `crates/mime/Cargo.toml`
> * `crates/mime/SPEC.md`
> * `crates/mime/examples/gen_serialized_corpus.rs`
> * `crates/mime/licenses/CC-BY-4.0.txt`
> * `crates/mime/licenses/LICENSE-APACHE`
> * `crates/mime/licenses/LICENSE-MIT`
> * `crates/mime/licenses/LICENSE-MULAN`
> * `crates/mime/licenses/NOTICE.txt`
> * `crates/mime/licenses/inventory.json`
> * `crates/mime/src/decode.rs`
> * `crates/mime/src/error.rs`
> * `crates/mime/src/lib.rs`
> * `crates/mime/src/model.rs`
> * `crates/mime/src/parse.rs`
> * `crates/mime/src/profile.rs`
> * `crates/mime/src/project.rs`
> * `crates/mime/src/transfer.rs`
> * `crates/mime/tests/corpus/cpython/LICENSES/Python-2.0.txt`
> * `crates/mime/tests/corpus/cpython/PROVENANCE.md`
> * `crates/mime/tests/corpus/cpython/REUSE.toml`
> * `crates/mime/tests/corpus/cpython/msg_01.txt`
> * `crates/mime/tests/corpus/cpython/msg_02.txt`
> * `crates/mime/tests/golden/serialized-corpus.sha256`
> * `crates/mime/tests/lossless.rs`
> * `crates/mime/tests/support/corpus.rs`
> * `crates/purrdf/Cargo.toml`
> * `crates/purrdf/src/lib.rs`
> * `crates/rdf-core/Cargo.toml`
> * `crates/rdf-core/src/cover.rs`
> * `crates/rdf-core/src/cover/emit.rs`
> * `crates/rdf-core/tests/byte_cover.rs`
> * `docs/RELEASE.md`
> * `docs/WASM_TESTING.md`
> * `docs/book/po/zh-Hans.po`
> * `docs/book/src/project/releases.md`
> * `docs/design/purrdf-simd.md`
> * `helpers-ledger.toml`
> * `layers.toml`
> * `scripts/check-doc-claims.py`
> * `scripts/check-generated.sh`
> * `scripts/conformance-frozen/mime-cpython.sha256`
> * `scripts/conformance-frozen/roots.toml`
> * `scripts/release-crates.sh`
> 
> </dd>
> </dl>
> </details>
> 
> 
> <hr>
> 
> </dd>
> </dl>
> </details>

<!-- end of auto-generated comment: rate limited by coderabbit.ai -->

<!-- autopilot:start -->
- [ ] <!-- {"checkboxId":"2708ad07-9f24-4260-9c11-7dc76a49f2e3"} --> <strong title="Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts">Autofix</strong> · Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts
<!-- autopilot:end -->
<!-- tips_start -->

---




<sub>Comment `@coderabbitai help` to get the list of available commands.</sub>

<!-- tips_end -->

## 2. paudley — 

# Generalized cover and MIME qualification

Both complete contracts #521/#520 are being delivered together; nothing counts closed
before protected integration, issue closure, archive verification and cleanup.

## Written implementation

One generalized core reconstruction body serves UTF-8 and opaque bytes. Shared
ByteCover/CoverBuilder, identity/delimited emitters and digest-bound arbitrary
SpanReference are applied. Existing Markdown/JSON callers still use that home.
The MIME crate is workspace/layer/facade/WASM/release-wired with first-party-only
runtime dependencies and explicit profile/vocabulary/limits. Flat original-byte
parsing, duplicate/folded fields, multipart/digest/message ranges, typed defects,
exact attachment transfer identity, native RDF projection and strict complete
graph reparse validation are implemented. Nested part_cover calls the shared
identity emitter. Mutable model declarations are checked against source before
projection or public decoded attachment publication. Projection uses the same
decoder after its whole-model validation, avoiding repeated parsing per part.

## Settled evidence

- complete-matrix-8.log, terminal session96276 exit0: core/MIME all-target
  warning-free Clippy PASS, original-byte cover5/5 PASS, MIME13/13 PASS.
- Actual SPARQL observes repeated original Header occurrences and order.
- Six real graph mutations refuse (missing header, altered ordinal/payload/
  source digest, named graph and invented predicate).
- Turtle, N-Triples and JSON-LD production serialization/reparse preserve exact
  source for original, nested, multipart and malformed message corpus; emitted
  bytes repeat exactly across runs.
- Decoded base64 and QP attachments share frozen SHA256 abc identity independent
  of message/filename. Nested part cover proves original child bytes.
- Actual >16MiB source/line,65,537 headers and depth257 admit under memory-only
  policy; explicit caller refusal neighbors pass. There is no hidden cap.
- Earlier matrix1–3 compiler failures and4 fixture-label mismatch are retained,
  not counted as passes. Matrix5's corrected depth neighbor passes; its Clippy
  failed three style findings. Matrix6 exposed the core fixture single-range
  lint; matrix7 exposed two test assertion lints. Those complete families are
  corrected without allowances or weakened assertions; matrix8 is settled PASS.

## Outstanding

Legacy-portable-matrix-1.log is terminalPASS: original UTF-8 cover7/7, JSON23/23,
Markdown9/9, faulting portable runner control and actual WASM byte-cover5/5 and
MIME13/13. Frozen-corpus-1.log is terminalPASS for the original four-message
transcript on native and actual WASM. These are historical settled-source results,
not proof for later corrections.

Independent combined-contract-review.md found three actual malformed-MIME
semantic families. The complete identity-transfer/line, boundary/delimiter and
contextual encoded-word correction is now written with three new family tests.
The shared production serialization corpus now includes two unchanged pinned
public CPython originals, including a15-part Mailman digest; their payloads have
the existing corpus guard and upstream license/provenance. The actual Rust
producer generated104f6e43fabc7234f9c27be2800879148b5daffcec7cfaf0d4ed7c459786e281
for the expanded six-message transcript and passed its reconstruction/repeat
assertions. Correction-family-3.log is terminal PASS: core5/MIME16 native and
actual MIME16 WASM, with warning-free affected all-target Clippy. Family1 failed
one inefficient byte-count lint; family2 failed
three fixture empty-value assertion lints. Corrected through the original byte
scanner and explicit value-reporting assertions; no allowance or weaker oracle.

Metadata-hygiene-1.log failed on registration/prose/domain-table/shim drift.
Metadata2 failed the release ordinal24 spelling. Source/prose/domain/ledger fixes
are written, and the focused doc-claim correction passes all154claims. The
existing ordinal table gained24 on one existing line, preserving the non-Rust
ratchet and every guard. The new MIME crate is accurately in the unbootstrapped
ledger: public crates.io-index lookup returned404; direct crates.io API was403,
so that API was not claimed successful. Historical31-record lock evidence remains
dated; no new record/publisher/functional publication is claimed.

The metadata writer now copies projections only when bytes actually differ;
every original comparison/refusal still runs. This avoids rebuilding unchanged
Unicode tables simply because a metadata pass touched their timestamps.
The independent recheck found three remaining edges in the same families:
charset-token backslash, mandatory phrase whitespace, and unrelated preamble
delimiter-looking text. Corrected together with exact malformed and healthy
neighbors. Correction-family-4.log is affected terminal PASS: all-target Clippy,
native MIME16 and actual WASM MIME16, including the unchanged expanded transcript.
Metadata3 was stopped on these concrete source findings before any full gate
began; its TERM output is retained in settled-full-1-unit.log and is not a pass.
Metadata4 is terminal PASS. Full-check-1 passed strict workspace/preserve-order
Clippy and compilation, plus the complete original repository hygiene collection
(layers, helpers, non-Rust ratchet, licenses, dependencies, frozen corpora,
generated projections, documentation claims and release/tooling controls).
It is now compiling/running full workspace tests on the settled source. The
native/WASM remainder is still pending; this is not a full-gate PASS.
The independent correction4 recheck passes all three behavior-task contracts;
reviews/combined-contract-review.md and tasks/T1-review.md through T3-review.md
contain the actual source/evidence dispositions. Independent intake, prior-art
and plan artifacts derive from that same substantive review. No new panel or
reviewer-run suite was required. Required makecheck, normal hooks/publication,
hosted acceptance and ghprsq merge remain NOT MET.

Hosted PR523's book failure identified stale translated source messages as a
required documentation dependency. This MIME branch changes two release-book
messages (31 to 32 crates); both matching zh-Hans msgids and their actual Chinese
translations are now updated together. msgfmt --check --check-format passes.
The actual rendered check-i18n gate runs after full1, without restarting its
unchanged Rust source qualification.
The generated MIME NOTICE retains the license writer's normal final section
separator; its narrow whitespace attribute preserves that format with visible
text diffs. The complete staged ordinary whitespace check now passes.

Mandatory full-check-1 is TERMINAL PASS: original process22822 returned exit0.
All original strict workspace checks, hygiene, native tests/doctests, consumer
and ring-fence checks passed, followed by all32 release-WASM crates (6m03s).
No assertion, corpus or required gate was weakened. The two translated release
messages and NOTICE formatting attribute have independent unchanged-runtime
applicability review. Actual original check-i18n and both English/Chinese book
builds are also TERMINAL PASS in book-final-1.log/process94431: all six rendering
gates, original poison controls,3040 translated/zero fuzzy/zero untranslated and
zero template drift, both actual HTML builds. Signed hooks, PR/hosted acceptance
and protected integration remain pending; no issue completion is claimed.

Normal commit guard initially stopped commit-1 on an inherited credential-assignment
example in the modified release guide. The example now consumes an already exported
registry token; no gate was bypassed. Original documentation-claim check154 PASS
after that documentation-only correction. Normal signed commit-2 and push PASS
at9e6c6af64. PR524 is OPEN with both complete issue closure links. Stagectl
pr-create returned a response parse error after creating it; actual remote lookup
establishes publication, so creation was not retried. Hosted acceptance, review
debt and actual integration assessment remain pending.

## 3. paudley — 

Preserve opaque bytes and MIME messages through one verified RDF cover

The existing core byte-cover law now serves arbitrary octets through one
reconstruction path, with verified borrowed spans, declared continuations,
digest-bound references and a delimited-record consumer preserving empty and
duplicate records. Its UTF-8 adapter retains the established text behavior.

The first-party MIME codec preserves duplicate/folded headers, nested messages,
part boundaries and original ranges in queryable RDF 1.2. Attachment decoding
uses the existing lexical homes. Reconstruction verifies the source cover and
complete selected projection before releasing bytes. Profiles, vocabularies and
optional physical resource bounds are caller supplied. Iterative parsing and
owned destruction avoid a recursion limit; malformed messages receive typed
problems without invented content or guessed metadata.

Core cover/emitter files hold the shared law. crates/mime owns the production
parser, model, projection and verification; facade, layer, release and WASM
registration expose that same implementation. Public CPython originals retain
their revision, byte digests and license evidence. Generated license projections
remain writer-produced. Release-book translations match the 32-crate source.
No external runtime dependency, semantic feature or vocabulary default is added.

Observed qualification: legacy UTF-8 cover/JSON/Markdown controls; actual native
and WASM core byte-cover cases; native16 and actual WASM16 MIME families; strict
affected Clippy; metadata4 and original full-gate hygiene collection. Frozen
six-message Turtle/N-Triples/JSON-LD transcript identity is
104f6e43fabc7234f9c27be2800879148b5daffcec7cfaf0d4ed7c459786e281.
Independent full-contract review and all three behavior-task reviews pass,
including the final documentation applicability delta.

The unchanged mandatory full local gate passes, including strict workspace lint,
all native tests/doctests, consumer/ring-fence checks and all32 release-WASM
crates. The original rendered translation gate and English/Chinese HTML builds
pass with zero template drift. Normal signed hooks and publication pass. The
release guide now uses an already exported registry token after the ordinary
commit guard identified its inherited assignment example. The new MIME member
is registered under the assembly audit's existing no-benchmark criterion; no
measurement or threshold is changed. Independent review retains the original
runtime qualification across these documentation-only corrections.

The clean integration candidate includes accepted PR522's DatasetView,
SPARQL planning and retrieval changes. Cover, lexical/hash/XSD homes and MIME
runtime have no conflicting delta. The current hosted test-merge controls
exercise the combined query consumers. Current-head hosted acceptance is complete:
48 successful checks, five expected optional skips, zero failures or pending
checks. Fresh stagectl merge brief reports pass; the refreshed candidate remains
2448b668cc84e9626b59b2e63b5ff0c7a8fc1f67 with a clean source worktree.
The distinct final review-debt audit passes against the refreshed full surface:
zero submitted reviews or threads, both connections completely captured, one
administrative bot spending-cap notice and the factual validation comment.
The bot notice has no source remedy; its successful status is not represented
as a substantive review. Complete independent contract review supplies that
judgment and applies to the actual candidate. No actionable feedback remains.

No user-authorized scope cuts or silently deferred acceptance criteria.
Authoritative plan: .stage/cover-law-generalized-identity-cover/plan.md.
Selected evidence: .stage/cover-law-generalized-identity-cover.

Closes #521
Closes #520

Defect-Class: none

