# Computed duplicate lifetime boundary

Source-derived implementation support, not an executed test or acceptance receipt.
Root inspected current `EvalCtx::admit_computed`, `ScratchInterner` admitted
ingress and the actual persisted fixture. The source writer owns the correction
and shipping regression; root made no source changes.

`admit_computed` currently stores every successful produced-value charge in a
context-long vector. Only CONCAT calls it so far. After interning an equal value,
the new value's strings disappear while its charge remains until context drop.
This measures cumulative production instead of live ownership and can cause a
capacity refusal even when the repeated temporary and one interned value fit.
Past owner-vector capacities are also kept via historic payload charges.

Use the existing `support::segmented::fixture` (201 matches for example.org/p) and
a prepared query whose second CONCAT argument is a 48 KiB caller-authored literal:

```sparql
SELECT DISTINCT ?value WHERE {
  ?subject <https://example.org/p> ?object .
  BIND(CONCAT(SUBSTR(STR(?object), 1, 0), "<48 KiB literal>") AS ?value)
}
```

The object-dependent zero-length substring prevents treating the full expression
as one query-wide constant; verify the actual evaluator takes the per-row path.
The resident oracle should return one distinct long value. Prepare outside the
execution allocation window to distinguish caller-owned prepared plan storage.
The current fixture's 8,000,000-byte live ceiling is below 201 copies of 48 KiB
but comfortably above the expected one-value scratch plus small row bag. Derive
the actual needed headroom from real admitted owners and measure the peak rather
than enshrine that intuition as proof. No arbitrary allowance may mask the leak.

Required assertions: bounded/resident payload parity; all 201 drivers actually
evaluate; scratch deduplicates equal values; simultaneous measured peak fits the
real ledger; current live charge does not accumulate 201 dead lexical payloads;
retained result clone/extraction holds its own admitted copy until last drop;
errors and final owner drop release every actual charge. Repeat with a genuinely
distinct per-driver output to prove that real retained scratch still grows.

Implementation direction: carry a producer's allocation guard with the value
until actual interning decides ownership. Transfer it to the arena for a new
computed value; release it on duplicate, invalid value or error. Keep temporary
intermediate guards until their actual payloads die, not automatically until query
end. Give the owner-vector's current capacity its own replaceable guard.
