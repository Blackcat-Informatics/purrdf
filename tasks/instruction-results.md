# Retired instruction results

Five paired query-minus-control values per operation; no timing acceptance.

- target, 1,024 reifiers: 353.001, 353.000, 353.000, 353.000, 353.001; median 353.000.
- target, 16,384 reifiers: 453.000, 453.000, 453.000, 453.002, 453.000; median 453.000.
- target, 262,144 reifiers: 553.000, 553.001, 553.000, 553.000, 552.999; median 553.000.
- target growth ratio: 1.5666; PASS.

- components, 1,024 reifiers: 258945.864, 258945.935, 258945.810, 258945.911, 258945.968; median 258945.911.
- components, 16,384 reifiers: 259601.871, 259601.920, 259601.818, 259601.940, 259601.958; median 259601.920.
- components, 262,144 reifiers: 260258.030, 260257.904, 260257.798, 260258.131, 260258.011; median 260258.011.
- components growth ratio: 1.0051; PASS.

- annotations, 1,024 reifiers: 250226.742, 250226.859, 250226.882, 250226.904, 250226.830; median 250226.859.
- annotations, 16,384 reifiers: 250450.789, 250450.907, 250450.938, 250450.929, 250450.877; median 250450.907.
- annotations, 262,144 reifiers: 250674.922, 250674.830, 250674.931, 250675.046, 250675.038; median 250674.931.
- annotations growth ratio: 1.0018; PASS.

All 90 executions passed; all instruction events have 100% coverage and all 45 paired deltas are positive.
