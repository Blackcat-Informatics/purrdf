<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Collision-admission frame readback

Inspected the retained raw bench WASM artifact from T3-531-bench-profile-artifact.jsonl
using the provided llvm-readobj and llvm-objdump, without recompilation or tool
changes. Full single-symbol disassembly is 531-exists-raw-frame-root.txt in this
review directory. This is the raw diagnostic artifact, not the linked package.

RdfDataset exists_prepared reserves3952 bytes at entry. At code addresses
0x1a53d7e through0x1a53e0a it references stack offsets3584 through3712. The call
at0x1a53dd2 is function15346 (0x3BF2), whose symbol is solution::SchemaBuilder::push.
The production source builds outer_bound with VarSchema::from_vars_admitted in
its collision-admission block, before choosing or descending through either
EXISTS strategy. These instructions attribute at least those high slots to the
inlined schema construction rather than only post-child evidence/memo completion.
They do not establish the complete size or live range of every source variable.
The whole frame remains reserved on nested substituted entries even where the
source skips collision admission.

A bounded owner hypothesis is to put the unchanged collision-admission block
in an out-of-line single implementation, returning before recursive evaluation.
It must preserve the original row-bound schema, whole-pattern collision walk,
error allocation/order and all workspace ownership; no additional allocations
or caller-path duplication are authorized. Root made no production edit. Native
semantic/strict controls and the unchanged optimized package stack slope/floor
must decide whether such a correction works. The prior two extraction candidates
failed that measurement and remain reverted; this finding is not acceptance.

## Initial source extraction review

The current expr.rs diff extracts exactly the prior collision-admission block
under the unchanged conditional. Bound-variable selection, admitted construction,
whole-pattern walk and error text allocation retain their order. The temporary
schema still drops before strategy selection and recursive descent. No additional
allocation or semantic change was observed. The helper needs only the shared
workspace rather than mutable context; that suggestion was sent to the owner.
Compiler and optimized stack acceptance have not run on this correction.
