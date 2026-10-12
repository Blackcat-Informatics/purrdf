# Landed XSD range synchronization

The geo successor was fast-forwarded from `a77b743bcc6fad89d88581ef9cb5a626a326834f` to `a4bb1232d3b4bdf682eab126504dd942ce2b3d5b`. Source qualification is still pending; no compiler, build, or test was run for this synchronization.

Complete preservation stash `eed2cd36c8e6537f02611ba79efec998ad972264` remains retained. The earlier preservation stash `feedf28ff79ceb6efe04c16dc09149d25e271381` also remains retained. `before-499-staged.patch`, `before-499-unstaged.patch`, `before-499-untracked.paths`, and `before-499-status.txt` capture the layers before synchronization.

The indexed stash restoration refused overlapping staged hunks before modifying either the index or worktree; both were verified clean. Source restoration without index restoration then produced one content conflict. Original staged/unstaged layers were reconstructed from the saved index for independent paths and the combined source for overlapping paths. `after-499-status.txt` exactly matches the saved status. All 193 restored untracked files match their saved blob bytes.

## Both-side resolutions

- `crates/xsd/src/bigint.rs`: retained the geo public `prepare_decimal_digits_in` admitted-arena adapter and the landed range producer's `pub(crate) prepare_decimal_digits_using` capability. Both use the same existing decimal-division body. Kept main's `Mag` visibility and shared `mul_pow10_using` dispatch together with the geo detached-magnitude/render ownership.
- `crates/lex/src/walk.rs`: retained the landed `VecReserve` original-owner construction interface and the geo native sorting exports and insertion layout. No alternate vector allocator was introduced.
- `crates/xsd/src/bigint/storage.rs`: retained main's fallible native arithmetic destination and the geo magnitude ownership operations.
- `crates/xsd/src/exact/decimal.rs`: retained main's native comparison/rounding storage adapters and allocator-threaded exponent calculation, plus geo prepared text and retained coefficient ownership.
- `crates/xsd/src/exact/integer.rs`: retained main's storage-threaded decimal-group adapter and the geo native detach/publication and borrowed-integer operations.

The last four files merged without conflict markers; their combined changes were inspected against the landed change and saved geo source. The landed `range.rs`, `range/storage.rs`, and `tests/range_storage.rs` are unchanged from the new HEAD. The index has no unmerged entries. The resolved bigint source passes syntax formatting only.

The complete geo scalar, profile/index, host-adapter, and qualification work continues on this synchronized source. Historical donor receipts do not qualify this source.
