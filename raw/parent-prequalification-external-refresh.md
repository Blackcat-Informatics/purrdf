# External prequalification refresh

Read-only retrieval at 2026-10-06T12:43:15Z while Task 5 implementation runs.
This is implementation/final-task input, not a final merge prerequisite readback.
Refresh again at the final required boundary if the evidence may have changed.

GitHub's main commit API for Blackcat-Informatics/gmeow-gts returned
`0d1c8299c9411ea4ead853e31721d42ea66f081e`, unchanged from intake. Its recursive
tree returned truncated=false. The complete vectors/cose tree contains only
sign1-basic.json and sign1-empty-id.json; no path in the complete repository tree
matches composite or ML-DSA. No shared composite vector publication is established.
Existing shared Ed vectors must still be executed; conditional absence is an
interop evidence limitation, not authorization to invent or regenerate vectors.

The live IANA COSE algorithm CSV at
https://www.iana.org/assignments/cose/algorithms.csv includes the unassigned range
from -256 through -54. Therefore -58 remains unassigned, while -49 is assigned
to standalone ML-DSA-65. Browser retrieval of the primary registry agrees.
Keep the pinned draft composite designation provisional; do not describe it as
a finalized allocation or replace the deliberately pinned construction.

Retrieved files and SHA256:

- parent-upstream-prequalification-commit.json:
  e4d78ae07e93bff91c3eca21b4f60a647a64ddf60c53389ddf36ddbd4b6a3dbe
- parent-upstream-prequalification-tree.json:
  f644842d31e6234af02e8fda1945b7afbfed24be00608c3fd957ca9db53d54c8
- parent-iana-prequalification-algorithms.csv:
  4dc4c64f84e6020a05403862219b66b0bfd9cc853245d87b82ed00f6d77600f3

No source, corpus, dependency or external repository mutation was performed.
