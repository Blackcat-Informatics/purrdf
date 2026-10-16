# Segmented scan block retention

The complete issue requires the existing quad iterator (also used by reifier and annotation streams) to retain its current authenticated block and parsed record range across yields. The current iterator instead reacquires its cache entry and parses the header for every row. No format, identity or public API changes are needed.

Standing constraints: protected main, standard host tools/directories, one bounded Cargo job, normal hooks and ghprsq integration. No timing acceptance or subagents. Review below is owner review; independent hosted review will be assessed at publication.

## Task 1: Retain and exhaust one block

Store the existing Arc plus a byte range into its immutable encoded buffer. Validate with fixed_records once on admission; decode each row with the existing decoder. Drop the old Arc before obtaining another summary/block. Cover all three shared streams and bound patterns, exact row order, one-slot transitions, empty streams and partial final blocks. Observe provider snapshot checks across each row and block to demonstrate readiness/cache transactions scale with blocks, not merely cache misses. Check original grants/early-drop eviction and sticky first failure at a block boundary. Use focused native tests and strict affected compilation; run relevant portable execution with the existing runner.

## Task 2: Publish and integrate the complete repair

Review actual call sites and nested scan ownership. Run normal commit/push hooks; open a complete issue-linked PR. Reuse hosted workspace/portable/conformance gates for final qualification. Resolve material review findings, assess integration, merge using ghprsq, verify issue closure and mirror, archive Stage evidence and remove the delivery branch/worktree. Update the root portfolio tracker. No prerequisite is replaced by a follow-up issue.
