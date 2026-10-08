#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C
export CARGO_BUILD_JOBS=8
logs=.stage/benchmarks-replace-the-java-lubm/tasks/T2-logs
checker=$(cargo run --locked --release --jobs 8 -q -p purrdf-bench --bin lubm-check -- executable)
binary=$("${checker}" artifact purrdf < "${logs}/cli-build.jsonl")
source=/opt/purrdf-lubm-native-v1-task1.wjKdyJ/default
[[ "$(b3sum "${source}/receipt.json" | cut -d ' ' -f1)" == 09438d399fc64246fa5f72fa127ca972212c491b97937dd820ec3d55cae22c6e ]]
smoke=$(mktemp -d /opt/purrdf-native-lubm-task2-smoke.XXXXXXXX)
printf '%s\n' "${smoke}" > "${logs}/smoke-path.txt"
printf 'owned smoke artifacts: %s\nchecker: %s\nCLI: %s\n' "${smoke}" "${checker}" "${binary}"
"${checker}" output "${smoke}"
mkdir "${smoke}/converted"
for file in "${source}"/*.nt; do
  name=$(basename "${file}" .nt)
  "${binary}" convert --from ntriples --to nquads "${file}" "${smoke}/converted/${name}.nq"
done
cat "${smoke}/converted/"*.nq > "${smoke}/aggregate.nq"
"${checker}" verify "${source}" "${smoke}/converted" "${smoke}/aggregate.nq" 0 0 1 http://swat.cse.lehigh.edu/onto/univ-bench.owl http://example.org/lubm/ "${smoke}/acceptance.json"
printf '%s\n' 'PREFIX ub: <http://swat.cse.lehigh.edu/onto/univ-bench.owl#> SELECT ?X WHERE { ?X a ub:GraduateStudent . ?X ub:takesCourse <http://www.Department0.University0.edu/GraduateCourse0> . }' > "${smoke}/Q1.rq"
printf '%s\n' 'PREFIX ub: <http://swat.cse.lehigh.edu/onto/univ-bench.owl#> SELECT ?X WHERE { ?X a ub:UndergraduateStudent . }' > "${smoke}/Q14.rq"
for id in Q1 Q14; do
  "${binary}" query --data "${smoke}/aggregate.nq" --results-format json "$(cat "${smoke}/${id}.rq")" > "${smoke}/${id}.json"
  printf '%s actual exact-set accepted rows: ' "${id}"
  "${checker}" results "${smoke}/acceptance.json" "${id}" < "${smoke}/${id}.json"
done
"${checker}" recheck "${source}" "${smoke}/converted" "${smoke}/aggregate.nq" 0 0 1 http://swat.cse.lehigh.edu/onto/univ-bench.owl http://example.org/lubm/ "${smoke}/acceptance.json"
sha256sum "${checker}" "${binary}" "${smoke}/acceptance.json" "${smoke}/aggregate.nq" "${smoke}/Q1.rq" "${smoke}/Q14.rq" "${smoke}/Q1.json" "${smoke}/Q14.json" > "${logs}/smoke-identities.sha256"
