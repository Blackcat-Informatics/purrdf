# Gitee main history repair for the next delivery

Authenticated noninteractive SSH readback succeeds and reports remote main
8229d73f97251e1a0502a486eee0f753a6e81d20. Its original commits are fetched only
to refs/stage/409/recovery-gitee-main. No remote push or main modification has
occurred. Original failing-job and readback receipts are preserved.

The merge base with current GitHub main e89e288ef476ed06c80c6593094c4d583d1f5214
is be63ef892a790093f309cac70de6f5ded2269e34. Exactly two Gitee-only commits
exist: 40a2cbf88c0ee0965ed8891ca10e039a8203d766 sets the Gitee upload buffer,
idle-connection bounds and declared push deadline; 8229d73f9 renames
README.zh-Hans.md to README_zh.md with identical content. Main already contains
both changes, including the identical current Gitee workflow. Their ancestry,
rather than missing production content, prevents the ordinary mirror push.

The prospective git merge-tree against these exact refs terminates1. Its only
conflicts are the Chinese README rename/delete and add/add at README_zh.md.
The Gitee README blob b9e626a70ab9ee12ed887a802875945bb7614f81 is exactly the
merge-base README.zh-Hans.md blob; it contains no Gitee-only prose changes.
Main's README_zh.md blob is 93c63d5953296e22f277ef33dfacd1d8cc2112bd. Thus the
reviewed resolution preserves the complete modern README_zh.md and its canonical
filename, while retaining both original Gitee commits as ancestors. This selects
the current README after evaluating both sides; it does not discard an unseen
Gitee edit. Re-evaluate against the final delivery refs if they change.

Once a complete delivery is ready for normal integration, synchronize its actual
base as required and merge this original Gitee main through the ordinary owned
worktree path. Preserve unrelated work and use normal hooks. Resolve the known
README conflict as evaluated above; verify that the resulting production tree
has no accidental stale README or workflow delta and that 8229d73f9 is an ancestor
of the delivery. Do not force-push Gitee, rewrite its commits, alter main directly
or fabricate a merge commit outside the normal verification path.

After qualified publication, hosted checks and ghprsq integration, verify the
ordinary Gitee mirror job and remote main/tag object IDs. Until those readbacks
succeed the main failure remains unresolved. No new repair branch is required.
