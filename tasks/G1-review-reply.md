Fixed in cdc43aaa6. K rows now refuse regex anchors before constructing patterns,
with the row, term and anchor in the error. Actual tests cover lone and mixed
regex anchors, literal K keep/drop neighbours, and valid non-K regex neighbours.
The glossary documents the literal-only K contract; existing rows are unchanged.

The restoration guard also preserves the original panic when a restoration write
fails during unwind. Normal restoration failure remains a hard failure with its
path and OS error; silently returning would conceal a failed qualification.
Three actual example controls verify exact restoration, normal refusal and
original-panic preservation.

Seventeen affected Rust tests, strict all-target clippy, production 1716 controls
and the 4181-unit scan, caller/parity/helper gates and source readbacks passed.
Independent completion review confirms all four issue requirements and nine plan
criteria remain met locally. The prior settled full/render checks apply to their
unchanged scope; current-head hosted checks are still running.

The generic 80% private-function docstring threshold is not a repository or
accepted-plan requirement. Existing module and behavioural contract documentation
and strict lint gates satisfy the applicable requirements; we decline boilerplate
added solely to meet that unrelated numeric threshold.
