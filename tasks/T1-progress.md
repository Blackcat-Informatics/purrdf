Task 1: truthful Cargo and C phase evidence

Commit3d0f398e1 instruments the existing C smoke and projection callers with one
shared Rust receipt helper. Cargo still prepares the cdylib on every invocation,
now locked, with exact package/target/artifact validation. C compilation, linking,
runtime and output checks have separate phase/status records. Source, header,
library, tool and configuration evidence is retained; malformed artifacts,
failed children and receipt-write failures remain hard failures.

Five focused tests passed, including the actual header/link/runtime and projection
permission checks; strict package all-target clippy, formatting, whitespace and
locked metadata passed. Git source paths are NUL-safe and temporary fixtures use
the shared testkit home. Only an existing first-party dev dependency was added.
Independent Task1 review PASS and normal commit hooks passed.

No performance improvement is claimed from these focused runs. Comparison
controller, hosted capture, admitted cold/warm campaign and full qualification
remain Tasks2–5. Existing merged sharding/profile fixes are preserved.
