# Prior-art assessment

The Stage brief finds zero prior commits/branches/PRs for this issue and seven
related forge items. No earlier delta implementation was established. Read
index.rs, surface.rs and surface/postings.rs: from_dataset calls collect_rows,
analyze_rows and assemble; SurfaceIndex reconstructs every gram and pronunciation
code in assemble. Existing sorted arrays and canonical ID order provide an
ordered remap/merge path. Reuse those homes and the existing analyzer/scorer;
do not introduce another tokenizer, ranker, hash or general merge framework.

The relevant design is docs/design/text-analysis.md and
docs/design/purrdf-text-scoring.md. No governing ADR is named by the issue.
This assessment is owner-performed, not independent approval.
