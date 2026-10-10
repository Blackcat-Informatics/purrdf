# Algebra Clippy correction 3

Complete proposed correction for the algebra findings in
`implementation-native-owner-clippy-correction-3.log`, excluding
`parser/table.rs` and `scope.rs`, which belong to the numeric lane.

Apply `algebra-clippy-correction-3.patch` to the current selected worktree.
Nine full postimages and source/postimage SHA-256 identities accompany the
ordinary unified diff. No shipping files were modified by this helper.

The diagnostic-template expectation now annotates the actual native template
body. The presentation-error helper borrows its input and precedes the test
module. Each of the four large native parser enums retains its existing inline
layout, with a fulfilled, individually justified size-heuristic expectation;
no extra boxes or owner allocations are introduced. Each whole-vector drain
retains its original admitted backing array until `Memory::release_vec` destroys
and refunds it, with an individually justified expectation at that operation.

Other changes are Clippy's direct syntax/lifetime/unit-pattern/semicolon fixes,
equivalent borrowed-key sorting, and borrowed singleton-slice test assertions.
Cleanup still executes even when the first render failure is already latched;
later cleanup errors do not replace that failure. Visitor admission still stops
at the first refusal. Templates, emitted text, parse laws, buffer capacities,
callback order, and destruction/refund order are unchanged.

Validation: Stage-only `rustfmt` syntax/format pass on all nine postimages and
ordinary `patch --batch --dry-run -p1` pass on all nine homes. No compiler,
Clippy, build, test, or gate was run by this helper. The writer's next coherent
qualification must confirm these expectations are fulfilled and the source
remains warning-free after the core and numeric corrections are integrated.
