# Retire the obsolete value merge draft

The first protected merge push was rejected before publication because
`value-merged.probe` contained two unresolved conflicts from an old diagnostic
three-way merge. PR #550 and its issue remained open; no integration occurred.
The failed command result is retained in `current-merge.log`.

The draft was reproduced byte for byte from its three retained inputs using
standard Git from the worktree root:

```text
git merge-file --diff3 -p .stage/v4-0-exact-tower-only-and-an-exact-or/value-current.probe .stage/v4-0-exact-tower-only-and-an-exact-or/value-base.probe .stage/v4-0-exact-tower-only-and-an-exact-or/value-donor.probe
```

Git reported two conflicts; byte comparison with the diagnostic draft returned
zero. The exact three inputs remain in the selected evidence. The reproducible,
obsolete output was removed as diagnostic debris. It was never shipping source
or an accepted test fixture. Current `crates/xsd/src/value.rs` has no conflict
markers, and the qualified source and its passing hosted checks are unchanged.

All required failed-test and package receipts remain retained. No guard, hook,
source, acceptance criterion or required input was changed or bypassed. The
retry uses the same protected merge tool and captures the corrected evidence.
