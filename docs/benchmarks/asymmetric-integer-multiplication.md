<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Asymmetric exact integer multiplication

The chunked product multiplies each long-operand chunk by the original
short operand. It accumulates only each product's magnitude and applies
the sign of the original pair to the final magnitude. The short operand
does not need a positive copy for every chunk.

The `xsd_exact_integer_products` benchmark uses public borrowed
`Integer` multiplication, with deterministic dense operands of exactly
the named binary limb widths. The 64-by-64 row is a balanced control.

A separate matched native probe retained batches of 32 results per
sample and checked every result against signed schoolbook multiplication
after the measurement. It ran on x86-64 Linux with Rust nightly
`5df8c02af`, the optimized bench profile, and the same pass-through
counting allocator on both sides. Instruction counts are user-mode
medians of seven fresh processes per shape with the median empty window
subtracted.

| Limb shape | Baseline instructions | Borrowed instructions | Allocations saved | Requested bytes saved |
| --- | ---: | ---: | ---: | ---: |
| 64 × 64 | 3,172,417 | 3,172,416 | 0 | 0 |
| 257 × 64 | 13,107,227 | 13,027,259 | 160 | 81,920 |
| 1025 × 64 | 51,900,689 | 51,628,630 | 544 | 278,528 |

The asymmetric rows save one 512-byte copy per chunk, or five and
seventeen copies per product. Retained bytes are unchanged. The peak
working allocation fell by 512 bytes for both asymmetric rows. These
figures describe local product work, not hosted CI, elapsed time,
portable performance, or a whole-query speedup.

```bash
cargo bench --locked -p purrdf-xsd --bench exact -- xsd_exact_integer_products
```
