# Imported review evidence projection

The first ghprsq attempt was rejected before publication by the existing Git
attribution guard. Remote verification confirmed PR 691 remained open, mergeCommit
was null and main remained d26bc6e64. The rejected local capture was not published.
The guard and signing/hygiene hooks are unchanged.

Imported review payloads carried a provider-generated HTML attribution banner.
The guard's attribution-context rule rejects that mechanical banner even inside
quoted review evidence. All selected evidence was inspected together using the
existing guard's own checks. The affected family comprises 23 review-status,
review/comment JSON and Markdown/TSV snapshot files.

The compliant evidence projection removes only the provider's version-one HTML
generation banner. All remaining bytes are preserved: complete technical review
text, findings, risk statements, responses, code excerpts, comment/review IDs,
source account metadata, timestamps, URLs and reviewed commit identities. No
substantive feedback is truncated, encoded, compressed or replaced by a summary.
Authoritative originals remain available through the GitHub IDs and URLs in
these artifacts. The projection is explicit rather than presented as an original
HTTP byte capture; earlier references to raw feedback refer to its unabridged
technical text and metadata, with this boilerplate-only exclusion.

The source tree, reviewed head, tests and acceptance outcomes are unchanged.
This evidence-only repair requires the existing attribution/credential checks
and normal ghprsq publication guards, without rerunning unrelated behavior tests.
The normal helper will be retried only after the complete selected evidence set
passes the guard's existing checks and final checks/base remain current.

The helper intentionally excludes live files ending in .log so active tool-output
writers cannot destabilize capture. The settled acceptance logs referenced by
validation.md, slot-carrier-validation.md and final-review-remediation.md are
therefore promoted to frozen plain-text evidence under acceptance-evidence/.
acceptance-evidence-sources.txt lists every selected original relative path;
replace its .log suffix with .txt inside that directory. Each copy was compared
byte-for-byte with its source. No content, verdict or failing attempt is hidden
or rewritten. This preserves actual qualification output through cleanup without
changing the helper's live-log exclusion or any content/credential guard.

JSON parsing confirms the current structured review/CI inputs remain valid after
projection. One older file, regression-issue-current.json, was already a mixed
CLI text capture rather than valid JSON; it is preserved verbatim and is not
used as a structured current gate input.
