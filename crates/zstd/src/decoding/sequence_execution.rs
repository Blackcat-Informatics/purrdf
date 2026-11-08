#![allow(
    clippy::inline_always,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
/// Update the most recently used offsets to reflect the provided offset value, and return the
/// "actual" offset needed because offsets are not stored in a raw way, some transformations are needed
/// before you get a functional number.
pub(crate) fn do_offset_history(offset_value: u32, lit_len: u32, scratch: &mut [u32; 3]) -> u32 {
    // Fast path: offset_value >= 4 means a fresh (non-repcode) offset, which
    // is the dominant case for non-trivial corpora. Upstream zstd (zstd_decompress_block.c
    // ZSTD_updateRep) special-cases this with a straight shift: rotate the
    // history down and store `offset_value - 3` at slot 0. No rule table, no
    // branchless masks. The slow path below handles repcode 1..=3 with the
    // full RULES table dispatch.
    if offset_value >= 4 {
        let actual = offset_value - 3;
        scratch[2] = scratch[1];
        scratch[1] = scratch[0];
        scratch[0] = actual;
        return actual;
    }

    do_offset_history_repcode(offset_value, lit_len, scratch)
}

// Previously `#[cold]+#[inline(never)]`; profiling
// attributed 15.42% of decoder mispredicts to this function with
// 27.80% on the `pushq %rax` fn entry — call/ret BTB pressure from
// the never-inlined boundary. `#[inline(never)]` was dropped first,
// keeping `#[cold]` to preserve out-of-line layout for low-entropy
// blocks (the prior «drop both» variant regressed +15.9% on L14).
//
// For high-repcode workloads (z000033 L-5, decode_all flamegraph
// surfaces this helper at 1.93% self-time despite the `#[cold]`
// label), the cold-bias attribute itself blocks LLVM from inlining
// even at the hot call sites where the call/ret + BTB cost still
// dominates the body work. Drop `#[cold]` and let the inline
// cost-model see the full picture — body is small enough (RULES
// lookup + 6 branchless cmov), so duplication into hot callers is
// affordable, and cold callers don't pay anything they weren't
// paying before (their cost was already dominated by the surrounding
// rare-path work, not this helper).
fn do_offset_history_repcode(offset_value: u32, lit_len: u32, scratch: &mut [u32; 3]) -> u32 {
    #[derive(Copy, Clone)]
    struct Rule {
        scratch_idx: usize,
        use_new_offset: bool,
        subtract_one: bool,
        update_mode: u8,
    }

    // update_mode:
    // 0 = no history update
    // 1 = [actual, old0, old2]
    // 2 = [actual, old0, old1]
    // Indexing: class * 2 + lit_is_zero
    const RULES: [Rule; 8] = [
        // class=0 (offset_value=1)
        Rule {
            // lit_len > 0
            scratch_idx: 0,
            use_new_offset: false,
            subtract_one: false,
            update_mode: 0,
        },
        Rule {
            // lit_len == 0
            scratch_idx: 1,
            use_new_offset: false,
            subtract_one: false,
            update_mode: 1,
        },
        // class=1 (offset_value=2)
        Rule {
            // lit_len > 0
            scratch_idx: 1,
            use_new_offset: false,
            subtract_one: false,
            update_mode: 1,
        },
        Rule {
            // lit_len == 0
            scratch_idx: 2,
            use_new_offset: false,
            subtract_one: false,
            update_mode: 2,
        },
        // class=2 (offset_value=3)
        Rule {
            // lit_len > 0
            scratch_idx: 2,
            use_new_offset: false,
            subtract_one: false,
            update_mode: 2,
        },
        Rule {
            // lit_len == 0
            scratch_idx: 0,
            use_new_offset: false,
            subtract_one: true,
            update_mode: 2,
        },
        // class=3 (offset_value>=4)
        Rule {
            // lit_len > 0
            scratch_idx: 0,
            use_new_offset: true,
            subtract_one: false,
            update_mode: 2,
        },
        Rule {
            // lit_len == 0
            scratch_idx: 0,
            use_new_offset: true,
            subtract_one: false,
            update_mode: 2,
        },
    ];

    #[inline(always)]
    fn mask_from_bool(cond: bool) -> u32 {
        0u32.wrapping_sub(u32::from(cond))
    }

    #[inline(always)]
    fn select_u32(a: u32, b: u32, choose_b: bool) -> u32 {
        let mask = mask_from_bool(choose_b);
        (a & !mask) | (b & mask)
    }

    let valid_offset = offset_value != 0;
    let class = offset_value.saturating_sub(1).min(3) as usize;
    let lit_is_zero = usize::from(lit_len == 0);
    let rule = RULES[class * 2 + lit_is_zero];

    let from_history = scratch[rule.scratch_idx];
    let from_new = offset_value.wrapping_sub(3);
    let mut actual_offset = select_u32(from_new, from_history, !rule.use_new_offset);
    actual_offset = actual_offset.wrapping_sub(u32::from(rule.subtract_one));
    actual_offset = select_u32(actual_offset, 0, !valid_offset);

    let old0 = scratch[0];
    let old1 = scratch[1];
    let old2 = scratch[2];

    let update_none = rule.update_mode == 0 || !valid_offset;
    let update_b = rule.update_mode == 2 && valid_offset;
    let update_any = !update_none;

    scratch[0] = select_u32(old0, actual_offset, update_any);
    scratch[1] = select_u32(old0, old1, update_none);
    scratch[2] = select_u32(old2, old1, update_b);

    actual_offset
}
