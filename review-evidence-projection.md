# Review evidence projection

Current paginated issue comments, review submissions, inline comments and
GraphQL threads retain original identifiers, authors, timestamps, URLs, states
and commit coverage. Mechanical HTML/UI blocks (authorization, attribution
banners, help/buttons and decorations) are removed structurally; all technical
feedback, risk assessments and exact coverage markers stay. No authorization
token is retained. The collector comment is refreshed from the authoritative
current capture by original URL, then uses the same projection. Historical
captures remain explicitly historical.

Current review covers de2095b46 with kind reviewed and no actionable findings.
There are zero formal review submissions, inline comments or threads, with
complete pagination. The linked-issue warning requires completed hosted CI;
that gate now passes in hosted run 40056143633 and the final full PR rollup.
The service's 75% docstring coverage is informational,
not a configured repository check. Changed private key/resolver comments describe
their invariants; public regression tests name the covered behavior. No
implementation or acceptance requirement is waived.
