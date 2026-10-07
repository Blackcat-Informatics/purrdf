# Process capture publication remediation

GitHub rejected the atomic evidence push because old raw perf captures contain process-stack credentials. Remote main and PR remained unchanged. All raw perf files are excluded from publication and retained only in an owner-only task artifact directory (0700, files 0600). No encoding or compression is used to evade the guard. The published projection contains only percentage, DSO and symbol rows from perf report, with all original bytes identified by SHA-256. Final instruction/allocation CSV tuples, complete result guards and schema-2 samples are unchanged; their acceptance does not depend on the private stack dumps.

raw/baseline-metered-32.perf sha256=cd244fdfc90340131edf5ea65a9ad7bb51659a7af707e12151c437b80135f043 report_exit=0 symbol_rows=109 projection=raw/baseline-metered-32.perf.symbols.txt
raw/candidate-initial-metered-32.perf sha256=2f1b9652548a0a6cbf454617442c19213fe9d7386d51bdec568333974de51f7b report_exit=0 symbol_rows=101 projection=raw/candidate-initial-metered-32.perf.symbols.txt
raw/candidate-batched-metered-32.perf sha256=b0956915ed042972cb5194cdac50a31527687bcbf56bcedbc86005a55af4dd44 report_exit=0 symbol_rows=107 projection=raw/candidate-batched-metered-32.perf.symbols.txt
raw/candidate-empty-arena-metered-32.perf sha256=7d8c4c7c21b1dd8328cb761268d011cc0f6aa433c60eec7d4fcf82ac14875562 report_exit=0 symbol_rows=104 projection=raw/candidate-empty-arena-metered-32.perf.symbols.txt
raw/optional-200000-final.perf sha256=9b95cfe073baee49b059d551be06b5487591f0322c2cca189666a0eb0fbe43d7 report_exit=0 symbol_rows=134 projection=raw/optional-200000-final.perf.symbols.txt
