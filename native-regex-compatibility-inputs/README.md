# Frozen unselected regex Unicode identity

`UnicodeData.txt` is the verbatim official Unicode 16.0.0 input retrieved from
https://www.unicode.org/Public/16.0.0/ucd/UnicodeData.txt on 2026-10-09.
Stage holds the input pending sole-writer integration at
`crates/iri/unicode/16.0.0/UnicodeData.txt`. Its license is Unicode-3.0,
the same license and provenance carried by the adjacent existing UCD inputs.

| Input | BLAKE3 | SHA-256 |
| --- | --- | --- |
| UnicodeData.txt | `24dd932e1b587f076f3895081f4eb2fd41c77881b3d84a2f743157f6b3f96c40` | `ff58e5823bd095166564a006e47d111130813dcf8bf234ef79fa51a870edb48f` |
| Existing CaseFolding.txt | `a9e649104f7da0ed7263e3b62543711a279ed4762409abc7bbe221a2afca4d5a` | `6f1f9c588eb4a5c718d9e8f93b782685e5c7fec872cf05e8e6878053599e09bb` |

The added mode in the existing Rust generator verifies the exact native BLAKE3
identities before deriving categories and simple folds. It shares category
decoding, category union construction, fold partitioning, and range emission
with the existing native generator. No external implementation or table array
is copied. The existing block lookup and shared XML terminal tables remain the
single homes of those jobs.

Writer generation is required: no generator binary was available and this
support agent has neither built nor generated the final Rust table. Emit
`xpath-compatibility` through the managed lane, format it, then compare a second
generation before qualification. The exact generated table is a required input
to the companion compatibility implementation patch, not an optional follow-up.
