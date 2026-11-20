# Issue #1407: Documents state the current contract: remove quoted decisions, history and status

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

The working documents of this repository state the current contract and nothing else. This is a bounded clean-up, not a rewrite: it removes history and corrects facts, and it keeps every rule.

Scope: the instruction files (`AGENTS.md`, `CLAUDE.md` and anything they import), the README, and any state, status, plan or gate document at the root or under `docs/` that a working session reads as instruction.

Do:

1. Remove quoted or attributed decisions, dated amendments, supersession notes and accounts of earlier approaches. Where such a passage carries a rule that still binds, keep the rule, stated directly in the present tense, and drop the quotation, the date and the attribution. Where it carries none, delete it.
2. Remove status and progress from instruction files: lists of open work, "currently", "pending", "blocked on", commit hashes given as state. What is still open becomes an issue in this forge; the document does not track it.
3. Correct every statement that is false against the repository: a path, script, crate, command, flag or count the document names that does not exist or no longer behaves as stated. Verify each named thing.
4. Where two passages state the same rule, keep one. Where two passages contradict each other, do not choose: list the pair in this issue and leave both until it is settled.
5. Where one document repeats another, keep the statement in the document that governs it and link to it from the other.

Do not: change the meaning of any rule, add a rule, soften a constraint on data, verification, lifecycle or publication, or reorganize beyond what the removals require. A passage whose removal might lose a rule stays, and is listed here.

Acceptance:

- A before-and-after list in the pull request: each passage removed, and for each the rule kept (quoted from the new text) or the statement that it carried none.
- Every path, script and command named in the changed documents exists, checked mechanically, with the check's output in the pull request.
- An independent review whose only question is whether a binding rule was lost or changed, with a pass.
- The repository's text check, where it has one, passes on every changed file; `prose-check` passes.

Size: one pull request, about an hour of work. If it grows past that, stop at what is done, merge it, and list the rest here.


## Comments (0)

