# Ordered storage and candidate ownership checkpoint

The independent source review found no rotation, deletion, free-list or
bidirectional traversal defect in the initial arena. It identified a duplicate
of the existing parser AVL kernel and a missing move-out-by-borrowed-key API.
Both are corrected in the shipping source: the original parser Entry/Slot arena
now lives in lex allocation::OrderedTable; parser arena iteration is preserved;
ordered/range/pop/remove-entry operations share that kernel. The evaluator
wrapper declares its table before LexicalFrame, resumes the original live total
for insertion, and retains a still-live replacement after failed shrinking.
No blanket Memory-scope refund is used for live-table edits.

Existing parser association, refusal and comparison-scaling controls moved with
their implementation. The new independent BTreeMap control exercises mixed
insert/delete/reuse, crossing bidirectional iteration and bounded ranges, then
moving first/last removal. These new sources have not been compiled or run.

The production candidate index and shared execution/environment controls now
use admitted owners. Native value bindings retain SharedWorkspace<WorkspaceTerm>
and prepared substitution borrows that value through its original memory home.
The independent review confirms this value-door ownership but identifies two
remaining production gaps: id bindings still use resident grounding, and an
external canonical candidate still parses through a resident decoder. Both must
be admitted at allocation birth. All fusion/result physical owners and complete
runtime/portable/performance/full-gate acceptance remain mandatory and open.

This is scoped source review evidence, not issue completion or runtime proof.
