# Measured schema width scaling

Status: PASS for all 12 report-only benchmarks; no failed lane. This is current-implementation scaling evidence on this host, not a matched before/after comparison or a claim of deployment throughput.

Command: `/opt/purrdf-schema-five-qualification/run cargo bench --locked -p purrdf-shapes --bench schema_surface -- schema_input_width_scaling`. The admitted lane used four Cargo jobs, 16GiB and no swap. Build completed in 5m 55s; the original harness retained ten samples per lane and 10,000 seeded bootstrap resamples at 95% confidence. Log: `schema-width-scaling-1.log`. Exact samples, median, MAD, outlier classification and confidence interval are preserved in `benchmark-estimates/schema_input_width_scaling/*/new/estimates.json`.

Each time is a median in seconds, followed by its 95% interval. Both class and property widths grow. The represented coverage-cell count is the appropriate size for compiler output; emitted definitions also carry increasing field width.

| Declared classes / definitions | Properties | Coverage cells | Compiler seconds | TypeScript seconds | GraphQL seconds | Pydantic seconds |
| ---: | ---: | ---: | --- | --- | --- | --- |
| 16,384 | 4 | 65,536 | 0.480 [0.469, 0.485] | 0.667 [0.657, 0.674] | 1.593 [1.299, 1.740] | 1.942 [1.615, 2.205] |
| 32,768 | 8 | 262,144 | 1.050 [1.040, 1.063] | 2.029 [1.898, 2.242] | 3.965 [3.508, 4.225] | 5.482 [4.951, 6.400] |
| 65,537 | 17 | 1,114,129 | 2.605 [2.514, 2.909] | 9.209 [8.780, 9.612] | 13.294 [12.209, 14.411] | 19.874 [18.493, 21.322] |

The largest fixture crosses the former 65,536-class and 1,048,576-cell limits. The independent native `large_schema_emission` acceptance target also proved actual public compiler coverage of 1,114,129 cells and 65,537 definitions through each of the three emitters, plus shared depth refusals and neighboring successes. Those assertions remain unchanged and passed settled native matrix2. The separate >16MiB and importer controls passed too.

These measurements show the complete supported widths execute and provide distributions for future comparisons. They do not isolate concurrent host load or claim an asymptotic proof from three sizes; input-derived checked bounds and the source algorithms are separate evidence.
