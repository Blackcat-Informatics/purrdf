# Caller-local iterator composition through one private kernel

SOURCE IMPLEMENTATION COMPLETE; independent source review, focused qualification and emitted-cost proof NOT MET / unrun for this tree.

Root-authorized modifier-only replacement staged tree72fa2c1ff03b00b4052bccf17895a091805785cf. Rustfmt0/no unstaged shipping changes; existing HEADfd963c263/MERGE_HEADdf2 preserved. Original579 failed artifacts and46120 actual cost800/threshold250 diagnosis retained unchanged.

One module-private macro_rules kernel defined before both uses replaces the one generic helper. Expansion first binds seq expression exactly once, then out expression exactly once. The previous complete kernel remains authored once: column index map, exact outer Vec capacity, ONE mapped outer Extend, once-per-source-row as_slice, existing inner SmallVec bulk projection, final SolutionSeq. No caller control flow/return/error branch added; no algorithm duplicate, unsafe/guard, marker/mode, dependency or compiler threshold change. Macro-local bindings are hygienic and source borrows end with their own expansion block.

Ordinary caller still evaluates &seq then layout, retaining original Lift construction/child/absorb/opaque early empty return/projection/finish. Contextual closure still constructs its Lift after layout and before allocation, then evaluates &seq and moves out once; finishes same caller-owned Lift. Standard Vec/SmallVec partial initialization/drop remains authoritative. Expansion gives actual iterator composition a call-site identity without inventing a nominal optimizer mode; it does not guarantee future inline decisions or erasure.

Pending root admission after independent source review: existing project-rust35 (Project1/prepared12/contextual22), strict eval all-target clippy/fmt, plus owning helper gate for actual macro/one-home registration applicability. Existing unchanged36 governed/Core53/worker45/docs/wheel evidence remains scoped. Later fresh candidate native+host complete twelve-row proof with strict immutable85204 baseline admission remains mandatory; no native-cost PASS from source or diagnostic scores. No build/commit/push/forge/copy executed.
