# Frozen lookup measurement diagnosis

Current local emitted code identifies the production computation exactly. Both
actual library graphs report RdfDataset::term_id_by_iri as one instruction: a
tail jump to FrozenTerms::term_id_by_iri. That shared lexical owner contains
204 instructions and seven vector operations in the baseline x86 configuration.
The source delegates directly to this owner for both RDF carrier shapes.

The manifest still selects the public forwarding method. Counting its zero
vector operations is correct; selecting that wrapper as the hashing kernel is
wrong. The correction must point both existing terminal-hash measurements at
the shared FrozenTerms implementation. Preserve every configuration, vector
floor, AES mnemonic requirement, FMA ceiling and relaxed-instruction prohibition.
No new runtime hash or duplicate lookup body is needed.

The metadata-artifact explanation was incorrect. Remove its reader filter and
restore all original graph membership. Earlier ARM assembly found in the hosted
diagnostic archive contains the old full public-method body; archive presence
does not prove it is the artifact reported by the current graph. Do not promote
that retained 13-vector body to current-source qualification.

The existing local probe is still running through its consumer graph. Do not
edit its launcher or inputs while it executes. After it completes, apply the
bounded manifest/reader correction, run the existing reader self-tests, and
qualify the actual shared body on all seven configurations. Regenerate counts
through the complete unchanged-criteria producer. No manual numeric cell edits,
timing, extra toolchain or exclusive host period is required or authorized.

This is owner diagnosis, not independent approval or complete issue acceptance.
