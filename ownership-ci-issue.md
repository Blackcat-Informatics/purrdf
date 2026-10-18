## Defect

The [main workspace job](https://github.com/Blackcat-Informatics/purrdf/actions/runs/39073548150/job/117515931802) fails helper hygiene because prepared DL proof storage and retrieval profile storage independently implement the same owned-or-borrowed immutable dereference. The prepared-context change introduced the second implementation. The full failure is the native helper census, not an ontology or model verdict.

## Required repair

Use one first-party generic ownership implementation in the existing common dependency home. Move both production callers onto it, remove both private duplicate enums/accessors, and preserve inline owning storage, zero-copy borrowing, immutable lifetime exclusion and ordinary move/drop behavior. The wrapper must impose no Clone or ToOwned requirement and no new allocation for borrowing. Do not change the census, add an exemption, box or clone the prepared knowledge base, add a shipping dependency, or introduce a semantic feature.

Validate non-Clone payload ownership and pointer identity, drop behavior, the actual prepared proof contexts and retrieval profile callers, strict affected all-target Clippy, and the full helper-hygiene target. Preserve both native proof decisions and retrieval behavior. Run normal hooks, inspect review feedback and merge only through ghprsq. Keep the larger checked causal-support implementation in #596 separate and unfinished.
