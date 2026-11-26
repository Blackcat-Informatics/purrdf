<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Exact-first decimal division

The public `numeric_div` operator first tries an exact decimal quotient
and rounds a repeating quotient to eighteen fractional digits, half to even.
Repeating quotients leave the inline exact path before creating the result
coefficient and normalization frame. A terminating quotient only scales its
coefficient by the missing power of two or five.

The `exact` benchmark measures three divisions of signed decimal operands
with varied scales. The terminating and repeating cases have small inline
coefficients. The wide repeating case uses larger inline coefficients that
can spill during arithmetic. The fixture and output capacity are prepared outside the
counter window. All 1,024 results per case stay live through the window and
are compared with an independent rational oracle afterward.

On a Ryzen AI Max+ 395 with Rust nightly `5df8c02af`, these are median
retired userspace instructions from ten fresh processes per case using the
release-derived bench profile. No empty-control subtraction is applied.

| Quotients | Baseline instructions | Current instructions | Reduction |
| --- | ---: | ---: | ---: |
| Terminating | 528,273.5 | 500,378 | 5.28% |
| Repeating | 513,692 | 476,825 | 7.18% |
| Wide repeating | 4,284,332 | 4,254,607 | 0.69% |

All output witnesses and heap measurements match. The two small cases use
no heap in the measured window. The wide case makes 123 allocations and
requests 3,936 bytes, with zero retained bytes and 32 peak working bytes.
These figures do not establish elapsed-time, whole-query, or spilled-input
performance.

```bash
cargo bench --locked -p purrdf-xsd --bench exact -- --decimal-division-instructions .stage/exact-first-counts
cargo test --release --locked -p purrdf-xsd --test exact_governance exact_first_decimal_division_matches_the_oracle_and_measures_heap -- --exact --nocapture
```
