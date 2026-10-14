# Issue #490: Typed capacity errors instead of u32 overflow panics, and 64-bit room for Datalog ids and global dictionary text

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

Several core tables are `u32`-indexed and **panic** when they fill: term tables, the term string arena, quad tables, delta term tables, merge and blank-node scope counters, and the Datalog id space. At Katamari's scale these are reachable data states, not programming errors. We ask for (1) typed capacity errors instead of panics on every such path, and (2) a 64-bit path where data can legitimately grow past `u32`: the Datalog id space, and the `GlobalDictionary` string arena (whose ids are already `u64` but whose string offsets are `u32`).

## Current behaviour (`main` at `6273b6173`)

Panicking sites (non-test):
- `crates/rdf-core/src/ir/term.rs:109-112` `TermId::from_index`: `expect("term table cannot exceed u32::MAX entries")`.
- `term.rs:252-256` `push_arena_str`: `expect("term arena exceeds u32::MAX bytes")` (`StrRange { offset: u32, len: u32 }`, `term.rs:224-226`).
- `ir/builder.rs:98` (interner), `builder.rs:259` (term table), `builder.rs:478-481` and `1078-1081` (`assert!` on quad capacity), `builder.rs:1062, 1069` (merge scope counter).
- `ir/dataset.rs:1097`: `expect("dataset quad count exceeds u32::MAX")` in lazy permutation build.
- `ir/mutable.rs:174`: `expect("delta term table exceeds u32::MAX")` on the delta insert path.
- `imports.rs:891, 1168` (blank-node scope counter), `provenance.rs:287, 344, 405` (unit / artifact / origin-set tables).
- `crates/datalog/src/id.rs:68-74` `Id::from_index`: panics past `u32::MAX - 1`, documented as "a programming error … never a reachable data state". Every Datalog `TermId`, `RowId` and `PartitionId` is this 32-bit type (`id.rs:50, 195, 203-207`).

Already typed, and the model to follow:
- `ir/import.rs:172-174` returns `DatasetImportError::Capacity` (term count only).
- `ir/global.rs:371-398` `try_intern_lookup` returns `Result`; `paged/stack.rs:82` `PagedStackError::Capacity`; segmented `SegmentedError::AddressExhausted`.

64-bit already exists: `GlobalTermId(NonZeroU64)` (`ir/global.rs:65`), `PackTermId = u64` (`ir/pack/dict.rs:102`), `EventTermId(u64)`. But `GlobalDictionary` stores strings through the same `u32` `StrRange`, and `global.rs:388` refuses past `u32::MAX` total bytes, so a global dictionary holds at most 4 GiB of term text.

## Why it matters

One Katamari mail vault is about 15 million messages, 300 million claims and 2-3 billion triples in naive RDF 1.2 form; its memory vaults are meant to grow for decades. In that range:
- a single dataset's quad count approaches `u32::MAX` (4.29 billion), and a rule evaluation over a vault's reifiers approaches the Datalog id space;
- the distinct literal text of a mail corpus (subjects, addresses, names, extracted text) passes 4 GiB long before the term count does.

A panic in a library linked into a store process takes down the store with no evidence of what was reached. Katamari's rule is that the only acceptable limits are physical ones, and a type width that cannot be made wider counts only when it really cannot: here it can.

## Proposed direction (a sketch)

1. **Typed errors everywhere first** (cheap, no layout change): every listed site returns a `Capacity` error naming the table and the count, propagated to the public entry points (builders, `MutableDataset`, imports, Datalog `compile`/`evaluate`). The Datalog id doc should stop calling overflow unreachable.
2. **`GlobalDictionary` with 64-bit string offsets** (or a chunked arena of `u32`-offset blocks addressed by a 64-bit position), so the global path's only limit is memory.
3. **A 64-bit Datalog id space**, either a type parameter on `Id<C>` (keeping `u32` where a caller wants the compact form) or a build that uses `GlobalTermId`-width ids, so rules can run over data that does not fit 32 bits.
4. The in-memory `RdfDataset` can stay `u32` per dataset (contract C0.8 makes ids dataset-local, and the paged/segmented layers are the 64-bit route), provided its limit is a typed error.

## Acceptance criteria

- A test per site drives the table past capacity (with a reduced-width test build or a mocked counter) and asserts a typed error, no panic.
- `GlobalDictionary` interns more than 4 GiB of term text in a test gated to 64-bit hosts with enough memory (or a unit test on the offset arithmetic with a synthetic arena).
- A Datalog evaluation with more than `u32::MAX` distinct ids is representable (unit test on id minting at the boundary; an end-to-end test where practical).
- No `expect`/`assert!` on table capacity remains in non-test code of `rdf-core` and `datalog`, enforced by a census check.

## Related

#90 (introduced the 64-bit global dictionary path), #457, #364, #477.


## Comments (0)

