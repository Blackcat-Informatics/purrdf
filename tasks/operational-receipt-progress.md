Published the remaining operational receipt assertions at `29f3ae565`.
Both complete affected test files pass: 17 fallible-query cases and 13 SERVICE
cases, including cold/warm/configured/stopped entries, request-source accounting,
capability denial and first-failure evidence. Every non-descriptor receipt byte
and all original behavior assertions remain compared. The shared helper explicitly
asserts scan-default access; no descriptor is simply ignored.

The preceding CI round completed with 47 successful checks. Its only failing
test jobs were these two files, followed by their aggregate. The new head is
pushed through normal hooks and CI is running again. The full local gate remains
live; it is executing the benchmark lane validator tests, whose standard nested
Cargo build is active. No full-gate pass is claimed while it runs.

Production source is unchanged from the last reviewed head. Refresh review and
the integration assessment for this two-file delta before merging.
