The blanket docstring-coverage warning is declined after checking the changed
contracts. The new public `SegmentedFileProvider` and `from_file` constructor
have rustdoc, including the trust boundary and constructor errors. Provider
overrides implement the documented `SegmentedProvider` trait. The shared visitor
explicitly documents that Break stops both streams before another source row,
and the new storage guide documents admission ownership and measured block sizes.

The reported percentage also includes private helpers, trait implementations and
test functions. No missing public contract was identified, and the repository
does not impose this blanket percentage. No product requirement or executable
acceptance criterion is waived. The source review's no-actionable-comment result
and the complete warning are retained with this disposition.
