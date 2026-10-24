# Actual requested behavior

The issue prioritizes avoiding panics and then defining recursive SHACL behavior.
No fixed-point dialect or acceptance of recursive answers was specified. The
current W3C Core text permits an explicit recursion failure. Treating cycles as
empty shapes or skipping property edges silently yields answers instead of
failure and must be removed. Physical stack refusal is required as well as cycle
detection, because a long acyclic graph can exhaust the stack too. Preserve
non-recursive answers and original operational-failure precedence through the
same production parser/validator all hosts use. Full scope and checks are in
plan.md. This is owner analysis, not independent approval.
