# Existing homes and prior-art applicability

The Stage brief found no prior issue-specific branch, commit or PR; its search
coverage is retained in prior-art.md. The actual source provides checked
growth_capacity and Memory::reserve_for_push in the lexical allocation home.
Memory::reserve intentionally supports exact known-size replacements, including
sorting, session arrays and conversion paths. Reusing amortized admission at the
incremental representative caller avoids changing those contracts and avoids a
duplicate growth implementation. The segmented hash-table home already checks
available capacity and needs measurements before attributing quadratic behavior.

The existing retained-term unit control exposes the real representative owner,
so the scaling regression extends that function. Existing segmented integration
controls cover memoized zero-allocation lookups, first fault, retained results and
chronological publication. Reuse their runner/provider/allocator rather than add
new infrastructure. The larger persisted reifier/annotation fixture must still
exercise the real fallible engine. No independent review occurred.
