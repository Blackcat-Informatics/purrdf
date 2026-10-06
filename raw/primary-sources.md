# Captured primary sources

Captured during Stage 1 intake on 2026-10-06, before implementation.
These are input evidence; no replay or conformance pass is claimed.

The normative composite law is pinned to published drafts, even though mutable
working-draft HTML has subsequently changed:

- https://www.ietf.org/archive/id/draft-ietf-jose-pq-composite-sigs-04.txt
  SHA256 `6c4288d87a14eddf8c9eaa2453edb6ea96d7134fa8673c19910048bac3315de6`.
- https://www.ietf.org/archive/id/draft-ietf-lamps-pq-composite-sigs-19.txt
  SHA256 `b4ee04416efc26e7d8de7c740dc3848ffe8b8cc9dcd035ad9d4e33e18844ec83`.
- https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.204.pdf
  SHA256 `57239b9f84c03227eda3ca0991204dc7764c79af9ce2e6824eda774918d46b6b`.

NIST FIPS 202 examples, each with 512 output bytes, are captured as PDFs and
plain-text extractions. Base URL:
https://csrc.nist.gov/csrc/media/projects/cryptographic-standards-and-guidelines/documents/examples/

| PDF | SHA256 |
|---|---|
| shake128_msg0.pdf | 3013d904e0f6cebc8dad6393cbf3eed4d1a537b3b3dc5e7b48552ff597942974 |
| shake128_msg1600.pdf | 437f905d9790ce7a18625b396ba57c6118bb46d1542ec3cf1d46faac23c261ef |
| shake256_msg0.pdf | d736c1a93eb6440e1e6b640402e31ea5258281b2b9237f84ea0ba4186518fc59 |
| shake256_msg1600.pdf | f4226f5c72914e5d2b331812c5973d8d685c8af1ccb97c2a3a2fd1c02fd173d1 |

NIST ACVP ML-DSA FIPS204 prompts and expected results are captured verbatim
from https://github.com/usnistgov/ACVP-Server at commit
`975de31eb83d87039ec88934fdc47d8c312b892d`, under
`gen-val/json-files/{ML-DSA-keyGen-FIPS204,ML-DSA-sigGen-FIPS204,ML-DSA-sigVer-FIPS204}`.

| Capture | SHA256 |
|---|---|
| ML-DSA-keyGen-FIPS204-prompt.json | 43e81ad820e495dbcad086fe27c1008393a8c32100bbbff77c558c3f06dcefef |
| ML-DSA-keyGen-FIPS204-expectedResults.json | 361f47ca19d592adcc66ff2cb591686ad785fea157b295648738bed6921a68df |
| ML-DSA-sigGen-FIPS204-prompt.json | 447749d72817b211160d243311ce32302f3023e59c355b0f70be2bd3e9e7830d |
| ML-DSA-sigGen-FIPS204-expectedResults.json | 228d011bbe274aeb93e22eea1e0d57b78f43795cf6a64fb5ef1e626485a0bedb |
| ML-DSA-sigVer-FIPS204-prompt.json | e2cba4589389756fa0bea1a7e6837138bf0a81f9d14234c9ee8f6d33caa1654e |
| ML-DSA-sigVer-FIPS204-expectedResults.json | e1d84ef1b2f35196278ab0b0ed6a46ec62cc03d2dfa92c564199e1999bfb8ea6 |

For ML-DSA-65 signature generation, the relevant pure external-interface groups
are deterministic group 3 and randomized group 15 (15 tests each). Internal
interface groups 9/10 and 21/22 also cover ML-DSA-65. Signature verification
groups 3/4 (external) and 9/10 (internal) cover ML-DSA-65. PreHash groups are
distinct from pure ML-DSA used by the composite; do not incorrectly label
unimplemented HashML-DSA as covered or treat pure fixtures as its evidence.

Fixture import must preserve identifiers and explicit coverage counts, use the
existing JSON/base16 homes, record provenance/license and avoid changing the
governed shared GTS vectors or copying external implementation bodies.
