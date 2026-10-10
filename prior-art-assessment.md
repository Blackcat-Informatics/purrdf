# Prior art and donor assessment

Stagectl's actual brief includes the complete two-comment preservation record,
14 related forge items, and a bounded last200-commit trailer census. No linked
issue crawl or governing ADR was found within that stated coverage; this does
not prove repository-wide absence. The captured census is22 none,2 performance,
1 semantic-identity-loss, not a count reconstructed from memory.

The preserved two-file patch is a source donor, not current acceptance. Its
explicit Emit walk removes recursive output production and quadratic nested
String construction. It also handles cyclic once-referenced blank components
and blanks occurring inside RDF1.2 triple terms, avoiding vanished cycle blocks
or identity-changing anonymous nodes. Those are material to full statement
preservation. Reconcile against current homes, use existing lex::walk::WorkList,
and leave the old dirty donor worktree and scratch example untouched.

Donor MAX_INDENT_LEVELS32 bounds output size, but it changes original bytes at
levels33–40. Permanent key guard40 is a separate ordering guard, not a license
to remove rows or to reject deep input. The plan must choose and freeze the
indentation policy explicitly; preserving indentation through40 then saturating
provides bounded linear output without changing the existing shallow guard band.

The donor comment's16-level quoted-term bound is supported by actual current
frozen-dataset validation. Keep the native triple-depth invariant and guard40;
no unsupported assumption that public datasets bypass validation is needed.
Quoted term identities, literal/IRI writers, fixed hashers and statement side
rows retain their original homes. Existing general Turtle/TriG SerGraph writers
are independent public consumers and must receive their own deep parity coverage,
including named graphs, rather than being silently replaced by graph-only packed
normalization. Related closed helper/streaming issues establish useful homes,
not acceptance of the new deep renderer.
