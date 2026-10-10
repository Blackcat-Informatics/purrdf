# Retained diagnostic owner review

Current candidate: `retained-diagnostic-core-owner-v2.patch`; earlier core-owner and
core-owner-context patches are superseded. Plain `git apply --check` and rustfmt
syntax parsing passed. This is not compilation, allocator qualification or a
completed diagnostic egress contract.

The five-home candidate provides exact concrete retained-byte inspection for
private diagnostic/presentation arrays, boxed detail and location fields; a
single admitted native render through lexical Memory; allocation-free location
Display; immutable RetainedDiagnostic sharing; and two actual allocator fixtures.
RetainedDiagnostic::render uses the shared LexicalFrame home and adds its actual
Shared control layout to the SAME surviving grant before publication. Its ordered
stack payload keeps text destruction before grant release even if Shared's
allocation-first factory is refused. Public borrowing exposes no mutable owner or
lease-free extraction.

Remaining paired production work:

- Replace the raw diagnostic in FallibleSparqlError::Query with the retained
  carrier, preserving borrowed code/message/location/presentation access and
  operational first-cause precedence.
- Pass the existing execution/reporting owner into finish_fallible_read. Native
  Evaluation failures render through RetainedDiagnostic::render; check the
  original account failure and final operation_status AFTER rendering.
- Keep initial account/control refusals inline and typed before any diagnostic
  allocation. AdmissionError::from currently renders all non-allocator EvalError
  values; that is an actual early allocation seam requiring replacement.
- Raw EvaluationFailure::Diagnostic producers need original physical admission
  BEFORE construction. Existing preparation and parameter diagnostics are not
  certified merely by moving them into a newly charged carrier. The currently
  inspected producers include prepare_for failures, ParameterPlan::check at
  engine.rs, RefusalPublication options adapters and graph-building diagnostics.
- Preserve exact typed failures without re-rendering a source error before
  selecting its sticky operational root. Remove obsolete raw production routes
  once their admitted callers are wired.
- Qualify real error lifetime/clone/drop and refusal paths together with the
  governor, native regex and broader public-query matrix. No test was run for
  this candidate, and no broad acceptance follows from apply/syntax checks.
