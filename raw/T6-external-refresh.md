# Final Stage 1 external evidence refresh

Retrieved 2026-10-06T13:52:50Z for PR preparation; this is not a later
pre-merge readback.

Official IANA COSE registry CSV:
https://www.iana.org/assignments/cose/algorithms.csv
The current row is Unassigned,-256 to -54, so the requested composite -58
remains unassigned. Standalone ML-DSA-65 is assigned -49 and must not be
substituted for this composite. The official HTML registry independently
agrees and identifies its last update as 2026-08-25:
https://www.iana.org/assignments/cose/cose.xhtml#algorithms

Authoritative governed GTS upstream:
https://github.com/Blackcat-Informatics/gmeow-gts
Read-only GitHub commit/main API returned
0d1c8299c9411ea4ead853e31721d42ea66f081e. The recursive tree at that exact
identity reports truncated=false. Its vectors/cose directory contains only
sign1-basic.json and sign1-empty-id.json. No path in the complete tree matches
composite or ML-DSA. No shared composite vector is published in this captured
identity; the issue's conditional shared-vector check has no additional fixture
to consume. Independent NIST/IETF fixtures are not shared GTS interoperability.
Existing frozen EdDSA vectors remain required executed regression evidence.

Files and SHA-256:

- T6-iana-algorithms.csv:
  4dc4c64f84e6020a05403862219b66b0bfd9cc853245d87b82ed00f6d77600f3
- T6-upstream-commit.json:
  e4d78ae07e93bff91c3eca21b4f60a647a64ddf60c53389ddf36ddbd4b6a3dbe
- T6-upstream-tree.json:
  f644842d31e6234af02e8fda1945b7afbfed24be00608c3fd957ca9db53d54c8

No external corpus, repository or registry mutation occurred. Pinned JOSE
draft-04/LAMPS draft-19 construction remains unchanged. Allocation and upstream
publication must be refreshed at their required merge boundary.
