# Settled release campaign results

Root ran the immutable v3 collector, production CLI and public-dispatch allocation
observer from /opt/purrdf-rules-479-3ec2dbba8-qualification. Collector exit0;
all20 production/observer cases passed complete fact/proof/authored-premise,
inference-count/channel checks and conservative10x-input/20x-growth envelopes.
No limit was raised. Parent source remains3ec2dbba8 with the recorded Task5
source patch and new-source receipt. See T5-campaign-admission.md for conditions.

Actual corpus/results: /opt/purrdf-rules-479-campaign-20261007-v3.
The exact measurements.tsv records every run; CLI collector log is
T5-logs/cli-campaign-v3.log. Time and OS wait4 peak RSS belong to production
processes; allocation traffic/retention/working peak belong to separate observer
processes calling the same public CLI. They are different measurements.

| Original case | 1k time (s) | 10k time (s) | 100k time (s) | 100k OS peak RSS (KiB) | 100k observer requested bytes | 100k observer working peak bytes |
|---|---:|---:|---:|---:|---:|---:|
| single |0.045863412|0.431725546|5.094892865|1325292|6778391241|1262225970|
| pair |0.054787089|0.452926830|4.587200005|1037184|6472849157|1011017508|
| notype |0.038677900|0.441060922|4.311693975|1293256|6484773502|1231247056|

Original pair has exact empty fact/proof output; single and notype each derive
allN expected facts and complete authored proofs. Positive two-Spool pair at1k
admits the two distinct assignments per vault, emits exactlyN deduplicated
conclusions and checks each canonical proof's five authored premises;
production0.056180457s, observer0.085600630s. Its OS peak RSS reports
251700KiB, while observed allocation working peak is13450314bytes. OS process
peaks include startup and are not interchangeable with evaluator/observer heap
peaks. The campaign is one ordered run, with external host activity and recorded
per-child load; it is empirical growth evidence, not a statistical speedup claim.

Root then executed immutable seminaive-v3 --bench with fresh task-owned
PURRDF_BENCH_HOME under the same campaign directory. Exit0:9 measured,0failed.
All nine estimates.json files exist; T5-logs/seminaive-campaign-v3.log retains
sample counts, intervals and outliers. Medians: fanout8/24/48 is268.4us/6.447ms/
48.78ms; frame-width2/6/12 is4.781us/13.36us/31.61us; recursion16/48/96 is
448.6us/2.312ms/10.52ms. These run the existing benchmark bodies and do not
assert comparison against an absent prior timing baseline. Independent factor
allocation still reproduces the earlier exact additive3N work/byte measurements.

Timing is finished; the heavy-build lane is released. Task5 independent review,
normal commit/push and Task6's settled full qualification/completion/PR/hosted
integration remain outstanding. Neither479 nor the broader364 is closed.
