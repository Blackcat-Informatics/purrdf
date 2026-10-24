# Iterative ShEx syntax/dependency walks: owner assessment

The actual structural checker and validator preparation callers now use one
private syntax census over the existing WorkList. Its preorder preserves source
order, includes value expressions nested below triple constraints, and treats
declaration references as leaves just as the former census did. The two mutual
recursive census implementations are removed rather than retained as fallbacks.

Dependency analysis needs contextual state rather than just syntax: its explicit
frames retain odd negation, reference-only paths, enclosing EXTRA predicates and
inverse direction. Inclusion entry schedules a leave continuation below its
expanded body. This preserves the active-path inclusion guard and lets a later
sibling inclusion be evaluated under its own enclosing shape. It does not make
inclusions globally visited or erase polarity on return. Tarjan remains the
existing core implementation.

Four focused public controls pass, covering 10,001-level missing/negated
references, 10,000-level mixed shape/triple EXTRA and inverse paths, preparation
on an empty map and repeat inclusion under different enclosing shapes. Only the
borrowed computations run on 256 KiB; construction and AST destruction explicitly
do not share that claim. Two structural unit controls pass, including the exact
nested label preorder. All 14 frozen negative-structure inputs retain refusal;
all 1,105 exact/plain validation manifest entries pass, with zero skips/xfails.
Strict affected all-target and actual wasm32 library compilation pass. Initial
lifetime-elision and test-assertion lints were fixed without allows. This changes
no public type, wire encoding, runtime option, corpus input or expected verdict.

Helper hygiene, hooked commit and publication are tracked in validation.md.
Runtime matcher compilation/count/action recursion and owned AST teardown remain
unfinished, as do the separate recursive SHACL model and solver contracts. This
is an owner assessment, not independent approval or issue completion.
