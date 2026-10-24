

<!-- This is an auto-generated reply by CodeRabbit -->
> [!TIP]
> For best results, initiate chat on the files or code changes.

<details>
<summary>🧩 Analysis chain</summary>
<dl>
<dd>

🏁 Script executed:

```shell
#!/bin/bash
set -euo pipefail
git rev-parse --short HEAD
rg -n -C3 'fn (prepare_with_expressions|check_builtin_cardinalities|openapi_doc)\b' crates/shapes
fd --type f 'iri-only-surface\.golden\.txt|core_parameter_cardinality\.rs|AGENTS\.md'
git diff --stat d26bc6e64 HEAD -- crates/shapes
```

Length of output: 7426

---

🏁 Script executed:

```shell
#!/bin/bash
set -euo pipefail
cat AGENTS.md
ast-grep outline crates/shapes/src/shapes.rs --match prepare_with_expressions
ast-grep outline crates/shapes/src/shapes/parser/cardinality.rs --match check_builtin_cardinalities
ast-grep outline crates/shapes/src/json_schema.rs --match openapi_doc
ast-grep outline crates/shapes/tests/core_parameter_cardinality.rs
sed -n '2279,2355p' crates/shapes/src/shapes.rs
sed -n '225,310p' crates/shapes/src/shapes/parser/cardinality.rs
sed -n '3860,3890p' crates/shapes/src/json_schema.rs
git diff d26bc6e64 HEAD -- crates/shapes/src/shapes/parser/cardinality.rs crates/shapes/tests/core_parameter_cardinality.rs crates/shapes/tests/fixtures/iri-only-surface.golden.txt
rg -n -C4 'ontology-complete openapi|#/\$defs/(Annotation|Literal)' crates/shapes/tests/fixtures/iri-only-surface.golden.txt
rg -n -C4 'UnknownReference|fn lower_reference\b' crates/shapes/src/plan.rs crates/shapes/src/shapes.rs
python - <<'PY'
import subprocess
p = 'crates/shapes/src/json_schema.rs'
def body(rev):
    text = subprocess.check_output(['git','show',f'{rev}:{p}'], text=True)
    start = text.index('fn openapi_doc(')
    end = text.index('\n}', start) + 2
    return text[start:end]
print('openapi_doc identical to d26bc6e64:', body('HEAD') == body('d26bc6e64'))
PY
```

Length of output: 68792

---

</dd>
</dl>
</details>

`@paudley`, your reconciliation is supported by the targeted source and baseline checks. I withdraw the general risk summary as a basis for additional changes in `#691`.

- **Property cardinality:** `prepare_with_expressions` calls `check_builtin_cardinalities()` before shape collection. The guard compares interned RDF object IDs and is unchanged from `d26bc6e64`. The test diff changes catalog access, not the expected constraint counts. I have no PR-specific cardinality counterexample.
- **OpenAPI references:** `openapi_doc` is byte-identical to `d26bc6e64`. The unchanged golden already contains `#/$defs/Annotation` and `#/$defs/Literal` references under `components/schemas`. This supports treating the packaging defect as pre-existing work tracked in `#692`, not requiring its repair in `#691`.
- **Admission hardening:** the inspected reference-resolution paths return `PreparationRefusal::UnknownReference` for missing definitions. I withdraw the proposal to require eager whole-catalog semantic closure in this frozen delivery.
- **Documentation:** I accept your decision not to expand this PR into a coverage campaign. This also matches the supplied repository Learnings: aggregate coverage alone does not establish a documentation defect. Repository-required documentation and compiler gates still apply.

This inspection did not rerun tests or verify hosted CI. I have no additional actionable finding from these four concerns. Before merge, confirm that all required hosted checks pass for `ea133b288`. The full LFP/GFP solver remains outside this PR under `#569`.

---

<details>
<summary>🧠 Learnings used</summary><blockquote>
<dl>
<dd>

```
Learnt from: paudley
Repo: Blackcat-Informatics/purrdf PR: 502
File: crates/helper-census/src/glossary/mod.rs:150-174
Timestamp: 2026-10-08T07:52:59.249Z
Learning: In Blackcat-Informatics/purrdf, a generic 80% private-function docstring threshold is not a repository requirement. Do not request boilerplate documentation solely to meet that threshold. Assess documentation against the applicable repository requirements, module documentation, behavioural contracts, and lint rules.
```

```
Learnt from: paudley
Repo: Blackcat-Informatics/purrdf PR: 431
File: :0-0
Timestamp: 2026-10-04T23:25:50.313Z
Learning: In Blackcat-Informatics/purrdf, PR #431's docstring-coverage warning was a false positive: the new public MMR API was fully documented, while uncovered items were private helpers, standard Display::fmt, and descriptive test entrypoints. Do not treat an aggregate coverage percentage alone as evidence of a substantive documentation gap or request padding comments solely to raise that percentage.
```

</dd>
</dl>
</blockquote></details>

<sub><i>You are interacting with an AI system.</i></sub>
