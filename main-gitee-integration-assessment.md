# Main mirror repair in the next delivery

Owner: the first qualified recovery delivery. This is part of that delivery's
normal base integration, not a separate repair branch or a direct-main change.

The actual failing mirror job is retained in `main-gitee-failure.log`.
Authenticated noninteractive readback and fetch are retained in
`main-gitee-ssh-readback.log` and `main-gitee-fetch.log`. The original Gitee main
is preserved at `refs/stage/409/recovery-gitee-main`, pointing to
8229d73f97251e1a0502a486eee0f753a6e81d20. Its two commits absent from origin/main
are40a2cbf88 (upload buffering/idle limits) and8229d73f9 (README rename).

The current origin/main workflow equals the original Gitee workflow. Gitee's
renamed README equals the merge-base's original README verbatim. Its commit
contains no unique prose to discard. Current main already has the modern
README_zh.md, including subsequent changes. The prospective merge's sole
conflict is the rename/delete plus add/add at that canonical README path;
`main-gitee-prospective-merge.txt` retains the complete conflict result.

After a normal hook-verified local implementation checkpoint, merge current
origin/main into the owned delivery. Preserve both sides of actual source,
generated, translated and portable-registration overlaps. Then normally merge
the original Gitee ref if its tip is not already an ancestor. Resolve the
evaluated README conflict by preserving all current README_zh.md prose and
canonical spelling, while retaining both original Gitee commits in ancestry.
Use normal merge/commit hooks and signing; never force Gitee or manufacture an
artificial merge, rewrite remote history, or push main directly.

Source ancestry alone does not repair the remote failure. The qualified delivery
must land through ghprsq, then the ordinary mirror job and remote main/tag
readback must actually succeed. Until then the mirror repair is pending.
