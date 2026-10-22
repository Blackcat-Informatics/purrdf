# Issue #677: Preserve native content and reconstruction causes in columnar errors

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

Part of #558. Preserve the native content-integrity and RDF-reconstruction failures that current columnar producers still render into Malformed detail strings. The parent retains its complete shipping inventory, remaining native/malformed producers and host contract.

Verified current-main gaps:

- ContentStoreError has native stored/actual ContentDigest values but no structured visitor; ContentDigest's original32bytes are inspectable without hex rendering.
- Columnar project_read renders verify_all's ContentStoreError into a String.
- Public columnar blob reconstruction renders insert_checked's native error into a row-prefixed String.
- RDF reconstruction renders a non-storage RdfDiagnostic into a String and extracts storage errors while discarding their enclosing original diagnostic fields/owners.

Implement through the existing homes:

- Borrowed structured inspection for ContentDigest and ContentStoreError, retaining both original identities and every field without formatting, cloning or allocation.
- Native columnar cause variants for content-store validation, a blob-row integrity refusal and the complete original reconstruction diagnostic. Move each original cause by value, keep row context typed and preserve the standard source chain and human presentation.
- Avoid a new error-box/control allocation on storage refusal. Move the original diagnostic directly. Preserve existing Clone/Eq traits through RdfDiagnostic's native resident_clone implementation; transport itself must not clone. Document the native-cause API correction and compile actual consumers.
- Exercise actual core insertion/verification producers at their native owner and public columnar corrupt-blob/reconstruction paths. Verify all original bytes/codes/private or nested data after inputs are dropped; include full-width fields, standard downcasts and zero four-quantity inspection allocation. The global store verifier cannot be made corrupt through safe public insertion; test that producer in its owner and the shared transport explicitly rather than claiming an unreachable public corruption scenario was executed.
- Native and actual packaged-WASM inspection/failure cases, existing successful frozen bytes, strict affected compiler/hygiene checks, normal hooks, independent review and hosted qualification before supported merge/archive/cleanup.

No wire-format, hashing, profile, compiler, cache, semantic-feature or dependency change. Preserve all four parent donors. Existing generic source-session preservation stays at its merged home. Static/malformed reason strings, dropped UTF-8 detail and the rest of #558 remain under the original parent and are not declared complete by this unit.


## Comments (0)

