# Production caller migration preparation

Task1 is reviewed, committed and pushed in97b05f758; progress6051940506.
Task2 is authorized by the sole plan and remains unimplemented. Main-root laws
apply. Root owns Git/forge and the single heavy-build lane.

Actual callers to replace: both Make check/check-i18n glossary invocations,
workspace CI invocation, and check-i18n-glossary in the pre-commit parallel gates
array. Hook helper code is compiled from the admitted working tree, while the
glossary root must be the staged checkout-index snapshot with its exported Git
index/worktree context. Preserve the real gate slot and every other hook gate.

Retire scripts/check-i18n-glossary.py; preserve po_catalog.py and independent
check-i18n-render.py. Update glossary documentation and the translated catalogue
header's obsolete tool reference. Find hidden CI/hooks with rg --hidden, excluding
Git/Stage evidence. Do not treat historical Stage mentions as production callers.

The existing gate-parity helper policy scanner recognizes only no-features,
python-binding-tests and self-test modes. Include the new glossary mode so actual
Make/CI parity continues to inspect it. The minimal parser alternative can be
changed without increasing legacy Python lines; meaningful new negative fixtures
belong in Rust and must prove missing local or hosted glossary coverage refuses.
Do not silence or remove parity to pass migration.

Required runtime controls include external root/PO/glossary paths, content-selected
renamed tracked Markdown, multiline fences, and both staged/worktree inversions:
clean working tree with poisoned temporary index must refuse; poisoned working
tree with clean temporary index must pass. Use isolated temporary indices and
normal hook invocation, preserve the real index, no fake commit or bypass.

Regenerate affected projections using their actual generators; focused checks,
non-Rust ratchet and strict affected clippy precede independent review and normal
commit/push. Task3 owns the single settled full suite and actual i18n/render gates.
