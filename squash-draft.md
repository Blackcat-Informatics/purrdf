Pack segmented dictionaries by encoded bytes and grow builders with input

SegmentedBuilder starts empty and grows its existing native storage with input;
declared term, byte and row limits remain admission ceilings. Dictionary and
reverse blocks close before the next actual front-coded record, fence and framing
would exceed block_bytes. records_per_block remains an upper bound. A single
encoded record that cannot fit returns the existing ConstructionLimit refusal.

Header admission, full certification and production lookup consume variable
populations together. Forward/reverse block counts may differ; stable full-width
IDs, RDF 1.2 side tables and empty graph declarations retain their semantics.
Authenticated learned bounds survive payload eviction under the original Memory
owner without pins or lexical bytes. Opening does not preload a directory.
Optional routing grows only from spare capacity and cannot evict payloads.
Only optional hint-index Residency can omit a hint after a payload is admitted;
required payload, exact-request, source and allocator failures remain terminal.

build.rs, format.rs, session.rs and session/cache.rs implement the one construction,
encoding, validation and reader path. segmented.rs covers growth, actual fit,
bidirectional high-ID reads, refusal and routing pressure. The existing evaluator
control now includes the formerly unsupported 4096/128 layout. Temporary probes
are removed. Storage documentation explains limits, routing accounting and
compatibility. helpers-ledger.toml binds unchanged proof contracts to actual source.

Landed profile 2 statistics streams and semantic certification are preserved.
Main integration has no production conflict; source registration conflicts use
fresh census values without changed classes, reasons or exemptions. The new
long-term fixture's statistical lookup names its actual authored term and retains
all count, selective-read, no-domain-payload and first-refusal assertions.

Affected validation passes: all original eight 2,000/200,000-claim file-backed
LIMIT/key/COUNT lanes under unchanged 256 MiB; all 18 segmented and 15 stack
integrations; 20 unchanged unit passes plus the corrected fixture-case rerun;
strict compilation, formatting and final fresh helper hygiene. The 200,000-claim
4096/128 COUNT returns 400,000 rows with 996,135 requests and 169,204,364 peak bytes.
Actual scalar and SIMD pressure executions retain their recorded source scope;
current hosted portable execution qualifies the final artifact. No timing
acceptance or duplicate local full-workspace gate is claimed.

Hosted CI run 38876291016 is pending; this is a draft, not final merge readiness.
Material routing review 4291214207 is fixed, reproduced and answered at
https://github.com/Blackcat-Informatics/purrdf/pull/561#discussion_r4292209997.
The second routing finding 4292239479 is corrected at 225e078da and answered at
https://github.com/Blackcat-Informatics/purrdf/pull/561#discussion_r4292667750.
The extended two/128-slot pressure case, all 33 native cases, all 21 units,
original eight-lane consumer and both actual portable pressure configurations
pass. Final current-head review/check refresh remains. Owner contract assessment is
recorded honestly as owner work, not an invented independent audit.

Repository goals are preserved: one encoder/home, native Rust, portable code,
deterministic bytes, typed refusal, original physical owners, no new dependency
or semantic feature, and normal hooks/signing. Earlier profile 1 requires
regeneration from RDF as already documented on main; this work does not invent a
compatibility fallback or silently admit unsupported images.

Plan: .stage/segmentedbuilder-pack-dictionary-blocks/plan.md
Selected evidence: .stage/segmentedbuilder-pack-dictionary-blocks
Defect-Class: none
Closes #560
