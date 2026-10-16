# Pack segmented dictionaries by bytes and grow construction on demand

Remove the need for callers to estimate private resident capacities or retry
whole builds with larger blocks. Keep explicit limits and block size unchanged.
Use the existing format and native memory/index authorities. No tooling,
toolchain/cache configuration, dependency or semantic Cargo feature is needed.
Main stays protected; all implementation is in the assigned isolated worktree.

## Completeness contract

| Requirement | Task | Actual proof |
|---|---|---|
| Limits do not allocate their ceilings | 1 | Native allocation window around maximum accepted limits and empty constructor reports zero allocations; actual three-row seal/reopen succeeds |
| Growth follows offered input | 1 | Same tiny dataset under small and full-width limits has equal constructor/ingestion allocations and equivalent sealed content; original refusals remain |
| Byte-filled dictionary blocks | 2 | Long adjacent and mixed term fixtures seal at unchanged block bytes/count cap; complete certification and every-ID read/reverse lookup succeed |
| Count is only an upper bound | 2 | Variable nonempty populations fit actual encoded bytes and never exceed records_per_block; forward/reverse streams may have different block counts |
| Single-record overflow is typed | 2 | A record whose required persisted encoding cannot fit one block returns ConstructionLimit, without publishing or retrying an image |
| Stable identity and bounded access | 2 | IDs above 2^53 survive; header-only open, sparse binary lookup, streamed enumeration, cache/pin accounting and RDF 1.2 side tables retain their contracts |
| Complete shipping | 3–4 | Focused package/portable/host checks, actual integration assessment, material hosted review and equivalent final gate; ghprsq merge, closure, archive, mirror, cleanup |

## Task 1: Demand-driven resident construction

Remove reserve_for_replay of build limits. Keep the public constructor/result,
typed limit validation and normal interning/row growth. Correct capacity wording.
Add native zero-allocation construction and tiny-input parity controls to the
existing segmented harness. Compile the coherent production slice immediately;
run only the affected controls, strict/helper checks and normal hooked publication.

## Task 2: Byte-aware encoding and complete reader/certifier consumption

Close a record block before the next encoded record would exceed block_bytes,
or at the count cap. Compute fit at the actual encoder, including required
framing/fence and front coding; do not duplicate a size estimator or serialize
successively larger candidate groups. Refuse a single oversized encoded record.
Keep old valid images readable and global insertion IDs unchanged.

Remove fixed-population header/forward/reverse cardinality assumptions. Preserve
nonempty blocks, total rows, strict order/identity, framing and authentication
checks. Replace direct division/modulo term lookup and fixed ordinal traversal
with bounded access to stored ID ranges; open must allocate no whole directory
or term map. Exercise long/mixed/triple terms in both dictionary orders, record
count caps, one-record boundaries, complete certification and actual reads.

Retain only authenticated page bounds learned from actual reads under the
operation's original Memory. Routing metadata must survive payload eviction
without retaining lexical bytes or pins; opening must not prefetch a directory.
This avoids repeating binary fence searches under a small payload cache while
preserving source identity, typed admission and exact request evidence.

## Task 3: Complete qualification and integration assessment

Run the affected segmented/stack/certification/source-fault and allocation controls
without changing their acceptance. Execute applicable scalar/SIMD portable paths.
Inspect the actual statistics/base delta, resolve shared format/proof changes
coherently and retain valid unchanged results. Update any generated/manual claims
actually changed. Use the equivalent hosted full gate once; no timing benchmark,
exclusive machine requirement or repeated whole local suite is introduced.

## Task 4: Publish the complete issue and integrate

Review the full production contract and evidence, create the complete PR, resolve
material hosted review and required checks, then integrate only through ghprsq.
Verify closure, selected evidence, actual integration and ordinary mirror before
removing the delivery branch/worktree. Do not split off a constructor-only PR.

## Scope decisions

No separate capacity hint is added: the existing native builder already grows
on demand. No jumbo-block subsystem is needed; oversized encoded records remain
a typed refusal under the caller's fixed block size. No full dictionary directory
or remap is introduced. The two fixes ship together. Zero subagents are used;
owner reviews are labelled honestly and hosted source review remains required.
