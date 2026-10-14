<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# #531 copied-tree VM site ownership

Status: source design, not implemented or qualified. Current borrowed VM site
correction is source reviewed and its focused tests pass; it cannot be stored
beside its own copied AST in a movable worklist frame.

`CorrelatedPattern` already owns a stable `Box<GraphPattern>` and the original
admitted `LexicalFrame`. Moving its enclosing value does not move the root's
nodes or their recorded addresses. The existing `PreparedPattern::new` moves
that value into `SharedWorkspace<CorrelatedPattern>` without cloning the tree;
reuse it rather than adding another wrapper. The shared control is born only for a copied window;
an ordinary external `Linked` with no EXISTS retains its present inline/no-site
path. A worklist frame and any linked VM state each hold one clone of the same
shared control. The shared payload (including the tree) drops before its
control grant, and every frame's detached scope token restores before the last
tree owner drops.

An owned link cannot hold `&Expression` borrowed from that shared control: it
would be self-referential when the frame moves. At link time, identify the
attached expression by an admitted path from the copied root through
`purrdf_sparql_algebra::NodeRef::for_each_child` ordinals. That is the one
exhaustive graph-pattern/expression child-order home; the existing VM
`visit_operands` still supplies each EXISTS operand path within the attached
expression. At suspension, traverse both paths through the immutable shared
root, check every index and final variant, and borrow the actual `Exists`
pattern only for the synchronous resolution call. No raw-address dereference,
same-address replacement token, second expression body, or AST clone is used.
The original source node addresses remain those of the one copied root, so
ledger and deferred-site identities are unchanged.

The existing borrowed link and the owned copied-window link should share one
site collection and one path resolver. The owned case stores its copied-root
control and the additional admitted root-to-expression path alongside the
current admitted EXISTS paths. Fork `fresh()` clones only that shared owner and
the shared site control; it never deep-copies path vectors. Admission for the
root-to-expression path and controls precedes publication. On any refusal,
the copied AST and its original lexical grant remain paired and are released
without installing a scope token or dispatching a VM instruction.

The first real worklist caller must link an expression in a copied window and
retain the linked VM across its suspended child. Until that caller exists,
adding an owned-link constructor alone would be an unused foundation path and
does not qualify #531. Acceptance must include same-owner success, an
out-of-range/path-mismatch refusal, worker shallow-copy admission, panic-safe
LIFO scope restoration, and the original three packaged stack floors. Exact
new control allocation/live bytes and fixed work must be measured rather than
described as unchanged.
