<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Native SHA-2 instruction work

Measured on an AMD Ryzen AI Max+ 395 (Zen 5) with Rust nightly
`5df8c02af` in a release harness at optimization level 3, with fat
LTO and one codegen unit. The host selects SHA-NI with AVX2 loads for
SHA-256 and the AVX-512VL message schedule for SHA-512. It does not
have the dedicated SHA-512 instruction extension.

Each result is the median of three retired userspace instruction
counts. Setup and result checking take place outside the counter
window. Heap measurements come from the same workload without
instruction counting. These are work measurements, not elapsed times
or measurements of other architectures.

## One-shot digests

The same executable calls the selected native API and `sha2` 0.11.1
on identical deterministic byte strings. Each window hashes 4,096
messages of 64 bytes, 512 messages of 1 KiB, or eight messages of
1 MiB. All digest bytes match. For both implementations, allocation
calls, requested bytes, retained bytes, and peak working heap bytes
are zero inside each window.

| Algorithm | Message size | Reference instructions | Native instructions |
| --- | ---: | ---: | ---: |
| SHA-256 | 64 B | 1,696,192 | 1,565,097 |
| SHA-256 | 1 KiB | 1,187,750 | 1,155,982 |
| SHA-256 | 1 MiB | 16,648,869 | 16,386,460 |
| SHA-512 | 64 B | 15,741,411 | 11,284,939 |
| SHA-512 | 1 KiB | 15,824,809 | 11,872,657 |
| SHA-512 | 1 MiB | 221,182,967 | 167,597,840 |

## RDF canonicalization

The fixture gives each claim a named subject, one blank reifier, and
three literal annotations. Both builds canonicalize the whole fixture
with explicit RDFC SHA-256 and produce the same canonical bytes with
zero n-degree work. The reference uses the retained `sha2` implementation
on base `616f8cb0f457`. The native build uses the real canonicalizer's
batch selection, not a benchmark-only dispatch. A separate,
unmeasured debugger run of the counted executable observed 64 and
256 block rounds in the AVX-512 compressor, corresponding to eight
and 32 full batches. It also saw the final empty batch drain.

| Claims | Output bytes | Reference instructions | Native instructions | Allocations | Requested bytes | Retained bytes | Peak working heap bytes |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 128 | 68,296 | 6,061,176 | 6,050,472 | 607 | 318,874 | 73,746 | 97,825 |
| 512 | 274,504 | 25,483,742 | 25,408,663 | 1,967 | 1,268,582 | 294,930 | 394,157 |

All four heap measures are identical on the two builds. The native
batch also uses fixed stack storage, which is not part of the measured
heap footprint.
