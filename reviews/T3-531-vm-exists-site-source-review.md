<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# #531 VM EXISTS site primitive — source review, 2026-10-13

**REVISE before claiming the owner-relative VM slice qualified.** This is a source-only finding on the current dirty `vm/mod.rs` and its production callers, not a runner, stack-floor or whole-issue verdict.

The new `Suspend::Exists(u32)` removes the direct borrow from VM suspension. Linking records operand-index paths in `compile::visit_operands` order, and resolution walks that same helper; the compiler already emits `Op::Exists(site)` in traversal order. A link with no EXISTS skips the new path loop and keeps an empty site vector. An EXISTS link now checks the supplied expression's root address and that its path ends at an `Expression::Exists`, then passes the actual borrowed pattern into the original `helpers::exists[_evidence]` body. This preserves the immediate pattern address for placeholder/ledger lookup if the same owner is retained. The current new test covers a single site and a distinct-address clone, not multiple ordered sites or reuse of an old address.

Two material gaps remain:

1. `Linked::fresh` deep-clones each `ExistsSite.steps` and collects a new site vector with every cloned site's `_allocation=None` and the cloned link's `_exists_allocation=None`. `expr.rs` FILTER/EXTEND and `binop.rs` filtered outer-join worker initializers call `fresh()`. Those new path payloads and the vector grow physically without the workspace admission retained by the original link. The prior borrowed-reference vector also cloned here, but this change adds variable-sized path storage at each worker. The original bounded/refusal contract needs either admitted shared immutable site storage at the existing VM home or a source-backed resident-only exemption; current source provides neither.
2. `ExistsSite::root_address` is an integer identity check, not an owner. The old `Linked<'e>` tied the link to a live expression borrow; the new link has no such lifetime or owner. A caller can relink/use a distinct expression at a reused address, and the root-address plus `Expression::Exists` shape check can accept a different site. The supplied expression is live during `term`, so this is not a dangling dereference, but it can silently evaluate a pattern unrelated to the compiled program. The planned copied-tree frame must retain the exact owner and bind that owner to the link, not merely compare raw root addresses. A test with a different-address clone does not exercise address reuse or changed contents in the same slot.

The focused VM test result was still running when this source was read. No build, test or edit was performed for this review.
