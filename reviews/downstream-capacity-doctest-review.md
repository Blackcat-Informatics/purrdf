# Downstream capacity doctest caller review

Verdict: PASS for the bounded five-file source correction. Executable
qualification is still running under root's session83933; no terminal result or
whole-gate success is inferred by this review.

Inspected the exact diff from e8285d713 in datalog/resolve_fol.rs,
sparql-eval/agg_fn.rs, sparql-eval/path_relation.rs, shapes/engine.rs and
shapes/report.rs; read the surrounding examples, actual TermDag and report
return types, and original failed doctest diagnostics in
tasks/final-consolidated-full-gate.log.

The derivation_id example now obtains the NodeId through `intern_leaf(...)?`
before constructing the unchanged asserted proof; the final hidden StorageError
return matches the actual TermDag API. AggregateRegistry's example similarly
propagates builder interning/insertion errors while retaining its custom
aggregate implementation, inputs, query and assertion. Both crates already
depend directly on purrdf-lex, so these typed example return paths require no
dependency change. Path relation construction propagates the same builder
errors into its existing mixed-error example return path; graph pairing,
registry/environment setup, execution and assertions are unchanged.

PreparedValidator's hidden example main changes from String to Box<dyn Error>,
which admits the actual storage, RDF diagnostic and Shapes errors at this
documentation boundary. Removing `freeze().map_err(to_string)` preserves the
original error rather than reducing it to text. All terms, inserted rows,
schema, focus lookup and report assertion remain unchanged. ValidationReport's
already-no_run example likewise uses Box<dyn Error> for its genuinely different
ShapesError and serializer diagnostic domains, and returns the original
serialized bytes through `Ok(...?)`. Its no_run status predates this change;
this slice does not convert an executed test into a compile-only one.

No production function, error representation, helper implementation, manifest,
tooling or workload changed. Boxing occurs only in these example error-return
boundaries; it does not introduce allocation on a production refused-admission
path. No error is silently swallowed and no assertion is weakened. Source
review finds no material defect. Root's reported edition2024 formatting/diff
checks are separate evidence; this reviewer started no build. Preserve the
original failed gate and await the actual three-package doctest terminal before
claiming the five original block failures resolved at runtime.
