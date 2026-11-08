#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Shared, C-faithful lazy-parse decision logic.
//!
//! Upstream zstd has a SINGLE lazy parser — `ZSTD_compressBlock_lazy_generic`
//! (`lib/compress/zstd_lazy.c`) — parameterized by the search method
//! (hash-chain / binary-tree / row-hash) and the lookahead `depth`. The match
//! finders differ per strategy; the PARSE DECISION (how a candidate at the
//! current position is weighed against one a byte or two ahead) is identical
//! across all of them.
//!
//! This module hosts that one decision so the per-strategy matchers
//! (HC / Row / dfast) share it instead of each carrying a divergent copy. The
//! register-hungry part is the search kernel, which stays specialized per
//! strategy and is injected here as a closure — the decision itself is a
//! handful of comparisons per position, so sharing it adds no spill pressure.

/// C-faithful lazy commit/defer decision, ONE source for every strategy.
///
/// Expanded INLINE at the call site (like upstream's single
/// `FORCE_INLINE_TEMPLATE ZSTD_compressBlock_lazy_generic`, zstd_lazy.c:1516):
/// the strategy's match finder is spliced in directly, so it stays inlined
/// inside the caller's (target_feature) kernel — a `fn` taking the finder as a
/// closure de-inlines a `#[target_feature]` finder across the call boundary and
/// regressed the Row band ~6%; a macro does not.
///
/// Evaluates to `Option<Option<M>>`:
/// - `None` → COMMIT the current best match;
/// - `Some(carry)` → DEFER and carry `carry` (the one-ahead `Option<M>`) to the
///   next position so it is not re-searched (upstream searches each position
///   once). A caller that re-searches instead just tests `.is_none()`.
///
/// `search = |pos, lit| <expr>` is a SPLICE, not a closure: the macro inlines
/// `<expr>` with `pos` / `lit` bound, yielding `Option<M>` for any match `M`
/// with `.match_len` / `.offset` (`usize`). Mirrors zstd_lazy.c:1629-1700 — the
/// depth-1 lookahead defers when the one-ahead match is longer (ties go to the
/// smaller offset); the depth-2 one (only at `lazy_depth >= 2`) when it is
/// strictly more than +1 longer. This length heuristic is faster than upstream's
/// gain weighting and still beats C on ratio (drop-in, not binary parity). The
/// only early-outs are the out-of-bounds guard and the `target_len`
/// sufficient-length shortcut (pass `usize::MAX` to disable it).
macro_rules! lazy_decide {
    (
        best_len = $bl:expr,
        best_off = $bo:expr,
        target_len = $target:expr,
        lazy_depth = $depth:expr,
        abs_pos = $abs:expr,
        lit_len = $lit:expr,
        history_end = $hist:expr,
        min_match = $mm:expr,
        search = |$p:ident, $l:ident| $search:expr $(,)?
    ) => {{
        let best_len = $bl;
        let best_off = $bo;
        let abs_pos = $abs;
        let history_end = $hist;
        let min_match = $mm;
        if best_len >= $target || abs_pos + 1 + min_match > history_end {
            ::core::option::Option::None
        } else {
            let lit_len = $lit;
            let next1 = {
                let $p = abs_pos + 1;
                let $l = lit_len + 1;
                $search
            };
            // Length heuristic: the established choice — faster than the gain
            // formula and still beats C on ratio (algorithmic freedom, not
            // binary parity). Depth-1 breaks ties on the smaller offset; depth-2
            // needs strictly +1 length.
            let win1 = ::core::matches!(&next1, ::core::option::Option::Some(n)
                if n.match_len > best_len
                    || (n.match_len == best_len && n.offset < best_off));
            if win1 {
                ::core::option::Option::Some(next1)
            } else if $depth >= 2 && abs_pos + 2 + min_match <= history_end {
                let next2 = {
                    let $p = abs_pos + 2;
                    let $l = lit_len + 2;
                    $search
                };
                let win2 = ::core::matches!(&next2, ::core::option::Option::Some(n)
                    if n.match_len > best_len + 1);
                if win2 {
                    ::core::option::Option::Some(next1)
                } else {
                    ::core::option::Option::None
                }
            } else {
                ::core::option::Option::None
            }
        }
    }};
}
pub(crate) use lazy_decide;
