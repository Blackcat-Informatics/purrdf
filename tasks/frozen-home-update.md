The frozen IRI lookup measurement is corrected in published `463d0bf16`.

Current emitted code shows the public dataset method is a one-instruction tail
jump to the shared `FrozenTerms::term_id_by_iri` implementation. That actual
owner contains the hashing and table probe: 204 instructions and seven vector
operations on baseline x86 in both captured library graphs. The manifest now
names this existing owner for both native carrier forms.

The earlier metadata-artifact explanation was wrong. Its reader filter is
removed, restoring the original graph membership. All configuration coverage,
vector floors, AES mnemonic requirements, FMA ceilings and relaxed-instruction
prohibitions remain unchanged. Existing reader self-tests and normal commit/push
hooks pass. No runtime lookup or hashing implementation was duplicated or changed.

The optional diagnostic was stopped after establishing this cause in both library
graphs; its incomplete consumer graph earns no acceptance credit. The required
complete projection is now dispatched on the corrected source:
https://github.com/Blackcat-Informatics/purrdf/actions/runs/38647095855

The seven-configuration result, generated document parity and current hosted
review/checks remain pending. No merge or issue closure is claimed.
