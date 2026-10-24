The wildcard field pattern is valid Rust for this tuple variant. The exact
`rdfc_w3c` target compiled and all six native cases passed before this review,
including the negative poison case returning `BudgetExceeded`. CI identifies
`projection_archive` as the compilation failure, rather than this match arm.
That missed example caller and stale book translations are corrected in
2d046fd55f8899a3451b35076c5e24cf8576e638. The review's stale carrier documentation
and redundant digest comparison are also corrected in that commit.
