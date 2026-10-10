Move prepared-schema ownership tests after production items

Keep both original allocator and borrowed-guard fixtures unchanged while
satisfying the current nightly's strict items-after-test-module lint. Production
preparation, proof checking, admission and all acceptance assertions are unchanged.
