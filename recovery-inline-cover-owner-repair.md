# Inline cover owner repair

`recovery-inline-endpoint-cover-2.log` terminated 101. Compilation succeeded;
the original warmed cover test passed its 30 km bound and failed at 400 km:
76 allocations for 142010 visited cells against the unchanged limit of 64.
This remains a failed receipt. The preceding attempt failed compilation and
has not been represented as runtime evidence.

Inspection found original coefficient-array producers retaining physical
accounts even when their containers and detached scalar fields have zero
native heap layout. The current repair represents inline produced arrays
without a container account. Heap-backed production continues to retain the
original account. Detached fixed arrays use the same native scalar detachment
home and omit the account only after the complete array/scalar layout proves
zero. Layout quotes now omit that nonexistent account too.

The cover's allocation limit, emitted-cell limit, numerical equations and
classifier law remain unchanged. `recovery-inline-array-cover.log` is terminal0:
the original warmed cover allocation test passes both 30 km and 400 km cases
against the unchanged64-allocation bound. The complete704-case kernel collection
is now running on that same compiled implementation; no full-kernel pass is
claimed yet. No allocation
probe, test exclusion, increased bound or alternate arithmetic path was added.
