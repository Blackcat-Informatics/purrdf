# Issue #423: Arbitrary-precision numeric tower for exact reasoned mathematics

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github
Labels: enhancement, epic

## Body

## Problem
The v4.0 direction for PurRDF includes **exact, reasoned mathematics in RDF/SPARQL**: exact combinatorics, partition-function and state counts, exact rational and decimal results derived by reasoning, not by floating-point calculation. The 3.x numeric layer is deliberately bounded (`xsd:integer` in `i128`, `xsd:decimal` with at most 18 fractional digits). Those bounds are documented and never fail silently, but an exact result outside them is an error, not a value. For example 34! > 10³⁸, and exact quotients need unbounded scale.

## Proposed solution (v4.0)
An arbitrary-precision numeric tower in Rust, wasm32-clean, with no optional components:
- **`xsd:integer`:** arbitrary-precision integer, with machine-word and `i128` fast paths. It builds on the existing `purrdf_xsd::BigInt` used by aggregates.
- **`xsd:decimal`:** arbitrary-precision decimal (big-integer mantissa, unbounded scale), with exact `+ − ×`.
- **Division:** `op:numeric-divide` follows a documented, caller-configurable precision policy. Evaluate whether an exact rational representation is wanted for reasoned results, and if so how it is projected back to XSD lexical forms.
- **Conversions:** every conversion between integer, decimal, float and double is correctly rounded with a single rounding (double→float is round-to-nearest-even); parsing and canonical rendering are exact.
- **Governance:** big-number cost is charged to the governor by operand size, so hostile inputs cannot exhaust memory or time.
- **Reasoning integration:** entailment regimes and SHACL constraints operate on exact values.
- **API:** may replace the 3.x bounded `Decimal` type, since this is v4.0.

## Acceptance
- Property tests against an exact rational oracle cover arithmetic, comparison, casts and canonical forms, including extreme magnitudes and scales.
- Benchmarks show no regression on the small-value fast path and bound the cost of large values.
- W3C SPARQL and XSD numeric suites pass.
- Native and wasm32 results are byte-identical.

The 3.x precursor (documented limits, typed overflow errors, exact comparison at any size) is #422.


## Comments (0)

