<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

G2 is closed as an evidenced false positive on head `52988974f2d11a40214281648983f9145af7a3fb`. GitHub alerts [235](https://github.com/Blackcat-Informatics/purrdf/security/code-scanning/235), [236](https://github.com/Blackcat-Informatics/purrdf/security/code-scanning/236) and [237](https://github.com/Blackcat-Informatics/purrdf/security/code-scanning/237) now read back as dismissed, reason false positive, including their exact-head instances.

Alert 235 identifies the internal ExpandMask counter κ. [FIPS 204 Algorithm 7](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.204.pdf) derives the private per-signature seed from K, caller randomizer and message representative, then starts κ at zero and increments by five. The actual implementation reserves each five-counter group and refuses exhaustion before reuse. This is a specified sampler counter; production Writer still requires a fallible fresh-randomness provider.

Alerts 236/237 identify public fixed key/IV bytes in the registered single-encryption test `file_integrity_rejects_header_chain_and_torn_damage_but_allows_opaque_encryption`. The test signs controlled opaque ciphertext and verifies without decryption material, asserting valid author authentication plus MissingKey. These bytes are no production secret/default. Production Encrypt0Options takes caller key and IV. No source constant, rule, workflow or assertion changed.

Normal dismissal requests initially failed HTTP 422 because comments exceeded GitHub's 280-character limit. Specific bounded comments succeeded; fresh readbacks confirm the outcome. All failures and original annotations remain preserved in Stage. Independent `tasks/G2-review.md` is PASS, SHA256 `562c765413300db48a6d2b5dd5d18295c780819de25828bd3728380e53170542`.

The [CodeQL alert check](https://github.com/Blackcat-Informatics/purrdf/runs/112352384442) now reports completed SUCCESS on that exact head; readback SHA256 `a1ed978e564fb8e60e2729adc84d9f0cfb60c90936fa15e29dd392299973da3d`. Original annotation count remains three. Initial CI run 37487670367 also completed SUCCESS at head529 and captured base `0d6575a46088e9420d73654b7b5965b9580c5d36`; this qualifies that captured input, not the later source/base.

G1 reader allocation and G3 owned SHAKE secret cleanup remain required/open. G1 implementation is in progress. Neither this disposition nor initial green CI establishes Stage 2 completion.

