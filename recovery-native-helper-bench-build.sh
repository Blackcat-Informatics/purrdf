#!/usr/bin/env bash
set -euo pipefail
cd /home/paudley/Active/purrdf/.worktrees/515-sparql-eval-datalog-per-row-evidence
bash .stage/sparql-eval-datalog-per-row-evidence/recovery-native-control-bench-build.sh
bash .stage/sparql-eval-datalog-per-row-evidence/recovery-physical-bench-build.sh
