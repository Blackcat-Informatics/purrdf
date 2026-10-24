# Implementation review

Implementer's review, not independent judgment. The root error_visit macro
destructures every variant/field exhaustively, retaining original byte offsets,
bounds, reason text, canonical keys/IRI text, classification and explicit absence
or native causes. CdtTypeErrorKind is a structured classification record rather
than a fabricated underlying failure. ReadError borrows the actual lexical or
storage failure through both protocols. Every production UCHAR/language-tag
construction retains its real decoder refusal; ECHAR and direction checks have
no native decoder cause and report absence. IRI validity has a boolean verdict,
so no invented parse cause is attached.

No decoder or alternate error path was introduced. UcharError receives standard
Display/Error at its existing lexical home solely so the original typed cause can
participate in source(). Existing CDT human messages remain unchanged. Inline
unit causes add no owned payload; diagnostic accounting still charges the actual
IRI/key String capacities. Public failure cases verify lifetime independence,
borrowed pointer identity, original native downcasts, accepted neighbours and all
comparison classifications. Complete native/resource/output checks pass.

The unrelated dirty donor remains preserved. Only its inspected CDT patch and
unique native test were transferred, then completed for source chains/resource
acceptance. No upstream or external repository work is authorized or performed.
Current-head independent review and full hosted qualification pass. The actual
current combined tree also passes41 focused native controls.
