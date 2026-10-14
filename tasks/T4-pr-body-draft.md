<!-- Draft only: refresh final-head qualification and review before PR creation. -->

Closes #470

PurRDF previously exposed bounded integer and decimal values and truncated ordinary decimal division at 18 places. This change makes the exact integer/decimal tower the public numeric value path. Ordinary division now returns every digit of a terminating quotient and otherwise rounds to 18 fractional places, half to even. Explicit `exact`, bare `N` (toward zero), and `N:ROUNDING` policies remain available. Temporal seconds retain their exact fractional value through arithmetic, casts, and SPARQL `SECONDS`.

The same policy reaches Rust, CLI, installed Python, C, and synchronous and asynchronous WASM queries and updates. The governor profile advances to v18 with frozen vectors; Breaking guidance, the English book, and zh-Hans translation describe the changed default. The unrelated #443 arithmetic-kernel campaign remains separate and is not included in this PR.

Validation already recorded on the #470 branch: XSD exact-value/governance suites and strict all-target check; governor v18 replay; CLI numeric cases; installed C smoke and Python 4,223-test wrapper; selected optimized WASM numeric cases; native and actual wasm32 numeric determinism; conformance matrix 16,362 pass / 16 ledgered / 0 fail; metadata, i18n, and document claims. The previous complete optimized package check failed three unchanged #531 stack-depth floors; it is not a passing package gate. Final current-main integration, corrected-head `make check`, full optimized package acceptance after the separate #531 fix, and hosted checks are required before publication or merge.
