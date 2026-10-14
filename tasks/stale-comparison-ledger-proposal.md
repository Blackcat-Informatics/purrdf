# Concrete stale source-ledger correction

Current final full gate35945 fails the unchanged shared-helper check because this
variant no longer names a production function. Root independently inspected the
actual ledger, the test-only method and public numeric module. The bounded decimal
production API was intentionally removed; reinstating its comparison merely to
resolve this entry would restore an obsolete production path.

Proposed change: delete exactly this seven-line variant entry from
helpers-ledger.toml, with its separating blank line. No replacement exemption,
checker, gate, acceptance criterion, script, toolchain or cache change is proposed.

```toml
[[job.variant]]
symbol = "purrdf_xsd::numeric::Decimal::cmp_exact"
file = "crates/xsd/src/numeric.rs"
detector = "forbidden"
criterion = "a"
anchor = "purrdf_xsd::numeric::Decimal"
reason = "The xsd:decimal value space this crate models is an i128 mantissa at a scale of at most 18 digits, which XSD 1.1 permits a processor to bound; a literal past it is refused, so its comparison is fixed-width integer arithmetic, not a big integer."
```

The source ledger remains untouched pending clarification of the user's freeze.
The failing gate stays failed. All later gate phases are unfinished.
