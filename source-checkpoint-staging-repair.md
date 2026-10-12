# Checkpoint staging failure

The first checkpoint command ended128 before any commit/hook execution:
`fatal: pathspec 'crates/geo/src/error.rs' did not match any files`.
The manifest wrongly used the HEAD-to-worktree union, including original
deletions already staged and therefore absent from both index and worktree.
Separate staged/unstaged patches and the original untracked archive were
captured before this attempt and remain preserved.

Retain the original staged changes. Add only actual unstaged tracked changes
plus untracked shipping source through a NUL-delimited manifest, then run the
normal signed commit and hooks. No reset, restored source, excluded required
change or hook bypass is used. This failed command is not a verified checkpoint.

The next staged whitespace check ended2 on the original new kernel NOTICE's
extra blank line at EOF. That blank line is removed; notice terms/content are
unchanged. The complete staged whitespace check now passes and the normal
signed commit/hook execution in `source-checkpoint-commit.log` ended1. It
collected formatting, two stale relocated scanner registrations, four glossary
claims and Python stub parity together. No commit was created.

The complete fixes format all source, register both neutral-kernel scanner
boundaries at their actual shared-reader homes, retain English profile and the
house entailment spelling, and exempt the physical phrase "whole surface" from
the API-surface glossary anchor. The Python classes are registered under
purrdf.geo and have correct geo.pyi declarations; the parity checker now follows
explicit PEP561 namespace re-exports into sibling stubs, retaining member/refusal
checks. Both legacy check scripts shrink within the non-Rust ratchet. No root
runtime class or phantom stub declaration is invented. The original failed
hook log is preserved; the next normal commit uses a distinct receipt.

`source-checkpoint-commit-2.log` ended1 solely because the new glossary regex
anchor lacked mandatory compatibility specimens. Other collected hook causes
were resolved, including complete formatting, scanner and Python stub checks.
The original Rust glossary home now tests API/Rust positives, physical whole
surface negatives, case folding and token-prefix exclusion, through both match
and production rejection controls. `recovery-glossary-compatibility.log` ended0.
The next normal signed checkpoint uses `source-checkpoint-commit-3.log`.
