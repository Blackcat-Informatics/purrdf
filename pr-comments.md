# PR review evidence

Technical projection; identifiers, coverage and all findings retained.

## github:comment:IC_kwDOTKq-Ms8AAAABfNxBjQ
https://github.com/Blackcat-Informatics/purrdf/pull/709#issuecomment-6389776781

Technical review projection, head b45cef24c7f1198cfff0a28f5a02b40cfc378df3,
base 2cc11ce451ee529032be27100c725d45797855bf.

No actionable comments generated. Merge risk minimal; no identified blocker
after normal checks. Security architecture risk low; no architecture concern
or introduced security weakness established. Older images need regeneration;
deployed reader/writer compatibility remains unverified.

Profile 5 uses a 408-byte header, checkpoint stream 11, 37-byte records
(digest 32, ordinal 4, stream 1) and one checkpoint per 32 rows. Prefix requests
read the preceding checkpoint and at most 31 tail digests, return exact
work.members and refuse endpoints beyond the directory. Construction and
certification check checkpoint counts, sums and block packing. Related scopes:
564 streaming/common-order scans, 705 oversized terms, earlier 687 directory.

The integrity boundary is a receipt-bound image and derived source comparisons.
Incorrect checkpoint authority could hide changes. Semantic certification and
authenticated selective reads protect that boundary. Receipt fields are private;
restoration needs host authentication; opening checks snapshot identity/length;
pages are authenticated before use. A checksum does not replace certification.
Failures latch in shared state; cache publication follows successful reading,
authentication, decoding and reservation. Full validation precedes authority.

Title, description, linked-issue coding requirements and scope checks pass.
All eight changed files relate to compact records, prefix reads, checkpoints,
certification, refusal, documentation or reviewed source-bound capacity proofs.
No unrelated change, new dependency, general-semantic WASM work or compatibility
decoder was identified. Proof reasons remain intact.

Optional docstring advisory: 9.52% against 80.00%, 21 functions in six files,
two unsupported skipped. This does not expand frozen scope.

Review limits: build.rs and test bodies were not directly read. Certification,
tamper results, 53 native cases and 82 versus 205 pages rely on change summaries
and author evidence; tests were not rerun by the reviewer. Checkpoint packing
(block_bytes - 9) / 32 was not independently checked. Hosted qualification was
pending at review time and is a process gate. Scope assessment directly read
source_directory.rs, session/statistics.rs and format.rs. No formal approval
is inferred from this review.

This projection retains technical findings, advisories and evidence limits.
Decorative UI, private navigation, automation controls and poems are omitted.


## github:review:PRR_kwDOTKq-Ms8AAAABUC4CLA
https://github.com/Blackcat-Informatics/purrdf/pull/709#pullrequestreview-5640159788



## github:review:PRR_kwDOTKq-Ms8AAAABUC6Ijw
https://github.com/Blackcat-Informatics/purrdf/pull/709#pullrequestreview-5640194191



## github:inline:PRRC_kwDOTKq-Ms8AAAABBGQZng
https://github.com/Blackcat-Informatics/purrdf/pull/709#discussion_r4368636318

## CodeQL / Cleartext logging of sensitive information

This operation writes [...::certify(...)](1) to a log file.

[Show more details](https://github.com/Blackcat-Informatics/purrdf/security/code-scanning/411)

## github:inline:PRRC_kwDOTKq-Ms8AAAABBGSVKg
https://github.com/Blackcat-Informatics/purrdf/pull/709#discussion_r4368667946

The sink at line 1783 prints two page counts from the private native fixture.
Both counts derive from `directory_image(1025)`, which interns only the fixed
`urn:directory:{index}` sequence and constructs deterministic quad rows. The
entire module is under `#[cfg(test)]`; this fixture accepts no external data or
credentials. The adjacent byte-equality assertion is a separate determinism
control. Alert 411 is classified as `used in tests` for this specific synthetic
input and diagnostic. The codec and its certification behavior remain covered
by the existing native controls.


