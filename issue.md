# Issue #696: Retain and expose complete native CDT failure fields and causes

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

Expose the complete CDT failure surface through the workspace's borrowed structured-error protocol. Current `CdtError`, comparison errors and the fallible reader wrapper cannot all be inspected generically, and UCHAR/language-tag parser failures lose their original native cause.

This is a complete leaf-codec delivery under #558. Reuse the unique CDT work preserved in the original structured-error donor, without transferring its unrelated merge or other crate changes. The parent remains open for the complete shipping inventory and host contract.

Acceptance:

- Implement complete borrowed inspection of every current variant/field in `CdtError`, `CdtTypeErrorKind`, `CdtTypeError` and `memory::ReadError`: original byte offsets, bounds/counts, key/IRI text, type classification, static reasons and nested causes. Use the existing root visitor and shared macros; no rendered-error parsing or new dependency/semantic feature.
- Retain the actual UCHAR and language-tag decoder causes at their production construction sites. Static ECHAR/direction refusals have no fabricated nested cause. Preserve the original storage refusal in the fallible reader. Wire native causes through the standard error-source chain where applicable as well as structured inspection.
- Verify actual public list/map parsing, comparison and fallible-reader failures after input disposal using a generic consumer. Include accepted neighbours, lexical and storage errors, native cause fields, original offsets and complete comparison classifications. Preserve all existing grammar, human messages and deterministic lexical output.
- Keep inspection borrowed and allocation-free, with no copied diagnostic strings or per-field boxes. Preserve original healthy-path allocation/admission limits and correctly account owned diagnostic payloads.
- Compile real workspace/binding callers of the changed public variants, preserving their assertions. Run focused native semantic/ownership/resource checks, strict caller lint, normal hooks and complete hosted review/checks. General semantics remain native; no new WASM semantic matrix.
- Land the complete leaf contract through ghprsq, verify archive/closure and clean up the owned checkout. No upstream dependency or external repository work is authorized. No parent completion is inferred.

Target at most four hours of individual issue work; preserve all original requirements if another independently complete delivery is discovered.


## Comments (0)

