<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Lossless compiled-proof packaging

Compiled-proof packaging is complete after the independent reviewer finished plaintext reads. No publication or final stable Stage capture is claimed.

The completed signed-baseline Python module LLVM is 172,422,504 bytes, above the forge's individual blob limit. Its complete assembly is 86,569,160 bytes. Both are preserved losslessly through deterministic `gzip -n -1 -k`, with no content reduction or encoding to bypass a content guard. Before compression, the live capture helper's exact credential expression was searched in plaintext with filename-only output; no matches were found (exit 1).

`gzip -t` completes successfully for both files. Decompression yields the exact SHA-256 values already recorded in `t2-base-python-ir-sha256.log`:

| Original relative to this raw directory | Decompressed SHA-256 |
|---|---|
| `t2-base-python-ir/purrdf_native.ll` | `b1b793a02913ad2dcd0e1c854e9476e888068331064806d1dea3891a9e8490e0` |
| `t2-base-python-ir/purrdf_native.s` | `f111691eb9a9d10c43c3d330170d6c23acb01ba16f38ed38a3f02a5f626c865b` |

The adjacent `.gz` files contain every original byte. After readers finish, oversized originals will be removed from selected publication evidence only after byte-for-byte decompression comparison. Existing proof references retain their original path meaning: reconstruct by running `gzip -dc PATH.gz > PATH` and verify against the original SHA-256 manifest. Candidate packaging will use the same operation after its actual emission finishes. Final archive verification and owned cleanup remain required.

## Failed ordinary-host candidate retained

The first complete Task2 candidate is failed native-cost evidence, not a passing candidate. Its LLVM has been packaged with the same deterministic command after the specialist confirmed plaintext comparison complete. The exact plaintext credential scan found no matches (exit1), `gzip -t` passed, and `cmp ORIGINAL <(gzip -dc ORIGINAL.gz)` exited0. Decompressed SHA-256 is `028a7ad2ebf8abe0fb0ff48577fcfb9af5dd8607f042c43413a526b9c5621cfc`, matching `t2-candidate-python-ir-sha256.log`. Only that oversized original LLVM was removed from the selected directory; every byte remains in `t2-candidate-python-ir/purrdf_native.ll.gz`. Its assembly remains plaintext, SHA-256 `ab016192e4a181e145fc20ca9b8ecf057a8a5a18c8b356324ef78f06219cdcb9`. The signed-baseline plaintext remains available for the repaired comparison.

## Unit-mode repaired emission preparation

The repaired module LLVM is187,178,194 bytes. Its complete deterministic gzip companion is prepared at `t2-unit-mode-python-ir/purrdf_native.ll.gz`; original plaintext remains for review. The exact plaintext credential scan found no matches (exit1), `gzip -t` passed, byte-for-byte decompression `cmp` passed, and decompressed SHA-256 is `1f0c458d50e92bd5d1244c45dd0a51b69f4eec61695db601667fabc77464d13e`, matching `t2-unit-mode-python-ir-sha256.log`. Assembly remains plaintext93,269,420 bytes with SHA-256 `7860e6d9d30703ee5392bb679cd95b0df40d2016f8144d4d041cc785229adb5d`. This packaging preparation does not supply a cost verdict; final removal of oversized plaintext waits for the reviewer's completed-read signal.

The native reviewer has now delivered structural cost MET and explicitly confirmed all plaintext comparison reads complete. Original baseline/repaired manifests checked successfully immediately before removal; baseline byte-for-byte decompression comparison also passed at finalization. Only the two oversized original LLVM files were removed; their complete adjacent gzip files remain. All three oversized LLVM paths (baseline, failed candidate, repaired candidate) are reconstructible by the command above with their original manifest hashes. Assembly, complete native ThinLTO artifacts, section inventories and failed receipts remain preserved. The baseline assembly's gzip companion is supplemental; its plaintext remains under the hosted blob limit.
