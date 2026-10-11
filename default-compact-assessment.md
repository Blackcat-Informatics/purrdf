# Compact default-performance rejection and diagnosis

VERDICT: NOT MET. The original complete 36-lane panel ran both artifacts with
zero harness failures. Twenty lanes report significant regression; the harness
success count is not performance acceptance. Current command exit 0 was collected
by root. Original baseline exit 0 was collected before the interrupted turn.
Raw estimates remain in `default-compact-estimates`; neither result is replaced.

Representative query changes: selective join +46.93%, scan/FILTER +39.28%,
OPTIONAL +22.35%, UNION +15.37%, GROUP +33.45%, path +66.73%, ORDER/LIMIT +34.06%,
DISTINCT +18.39%. Several later query lanes improve. The footprint correction
alone is therefore not qualified by this pair.

The pair also moves unchanged direct arithmetic controls: integer promoted
comparison 4.514ns to 6.206ns (+37.47%), double promoted comparison 7.101ns to
15.125ns (+112.99%), and integer total comparison +145.82%. These routines call
`purrdf_xsd::{value_cmp,value_total_cmp}` on the same pre-parsed operands, without
evaluator rows. Both artifacts use the same XSD dependency, compiler, bench
profile and `-D warnings`. Linked `value_cmp` has 349 instructions in each;
instruction bodies are identical after relocation normalization, with identical
64-byte entry alignment. This establishes unchanged arithmetic instruction work,
not a performance pass or an attribution of every query difference to noise.

The baseline log ended 18:32:33; the current log ended 18:36:28. No contemporaneous
scheduler/CPU samples were captured for the original pair. Post-run observation:
CPU8 uses the performance governor, reads about 3.97GHz and shares SMT thread24;
host load is about30 and CPU pressure avg300 about7.4%. These observations cannot
retroactively establish the original run's precise resource state. No historical
sysstat logs are available at the usual paths. The original double-control
samples range6.96–8.57ns; current samples range10.99–29.24ns.

Before further source-performance judgment, preserve this rejection and capture
time/resource context around an immediately adjacent original36 baseline/current
execution with the same artifacts and policies. If the control dispersion clears
and evaluator regressions remain, profile and repair their actual source paths.
The mandatory full local qualification and publication remain blocked.
