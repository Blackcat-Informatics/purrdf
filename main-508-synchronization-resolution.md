# Native ownership synchronization

The successor was fast-forwarded to `a77b743bcc6fad89d88581ef9cb5a626a326834f`, then the preserved geo changes were restored once from stash `feedf28ff79ceb6efe04c16dc09149d25e271381`. That preservation object remains intact, including its original untracked allocation files in parent 3. Donor worktrees and their layers are untouched.

All eight textual conflicts were evaluated against their index main and geo sides. Resolution retains the landed native bodies and extends them at their existing homes:

| Home | Resolution |
| --- | --- |
| `lex/walk.rs` | Retains landed fallible/admitted spill reservation and destruction; adds an insertion-layout certificate using the landed generic capacity law. |
| `rdf-core/small.rs` | Retains fallible native growth and drop-before-refund; factors admitted exact reserve into the existing push body. |
| `xsd/bigint.rs` | Retains native arithmetic/provider and decimal-group bodies; adds original-owner detached publication, pooled absolute copy, and the borrowed native decimal preparation door. |
| `xsd/bigint/scratch.rs` | Retains allocator errors and adds typed lower storage errors. The same pooled destination engine retains its original arena through returned values. |
| `xsd/exact/binary.rs` | Retains admitted rounding API and callback; delegates to the single integer IEEE ratio-rounding home used by geo. Existing nearest-even, signed underflow, overflow and binary32/binary64 assertions remain required unchanged qualification. |
| `xsd/exact/decimal.rs` | Retains native prepared decimal groups and presentation; factors coefficient preparation and adds same-body admitted scaled-integer rendering. |
| `xsd/exact/integer.rs` | Retains all landed native operations; adds detached original-owner publication and borrowed coefficient access. |
| `xsd/exact/mod.rs` | Retains existing exports and adds the native geo rendering exports. |

The four allocation originals were also compared. `allocation/boxed.rs` and `allocation/text.rs` are identical and retain main unchanged. `allocation.rs` keeps main's typed `growth_capacity<T>` (initial 8 byte slots, 4 ordinary slots, or 1 large slot), exact-size collection, native string growth and all existing before-birth/refund laws. Geo heap certificates/builders and root-record certificates now use that same typed law. Its new heap regression uses a u64 old-32/new-64-byte overlap and the actual 95/96-byte refusal/acceptance boundary. `allocation/shared.rs` retains the landed factory-panic cleanup and control-before-payload destruction; additions expose owner count, an admission-preserving infallible payload factory, and owned-text capacity metadata. No original ownership lease is extracted or replaced by a later census.

The XSD value storage-error mapping preserves the new lower error directly. Native geo diagnostics also propagate lower numerical storage errors before rendering.

Explicit rustfmt parsed all affected source files. Conflict-marker inspection is empty. No compiler, build, runtime test, full gate, commit, push or merge was performed by this resolution unit. Complete #408/#409 acceptance remains NOT MET pending the remaining source closure and admitted consolidated qualification.
