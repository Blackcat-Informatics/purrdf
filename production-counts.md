# Production count acceptance

Frozen clean source 13dd0b009, baseline 5db00aaae9; same compiler 21b707e3f, cache, profiles and collector workloads. Normal unprobed rebuild session 13742 exit 0; all 21 lanes, ten samples per allocator mode, 37 exact artifact-bound worker proofs and allocation-coverage controls completed in session 87774 exit 0. Normal Cargo shared-report comparison exited 0: raw/verified-count-comparisons.tsv (105 quantity rows) and verified-count-comparison-run.log (all 21 full receipt comparisons). Every sample verifies all 5000 typed SUM results or complete BIND answers and complete GovernorEvidence; no timed measurements.

Numeric requested traffic improves in every lane, approximately 397–400 KB/query. Returned retained bytes are identical in all lanes. Allocation calls are unchanged apart from small observed decreases. Whole-process peak is a tradeoff: observed numeric median changes range from -329780 to +259096 bytes (largest percentage increase +1.69%). Original chunk capacity now overlaps parent admission rather than a compact flattened minted bag; descriptors remain until consumed. Concurrent worker/interner peaks can vary. No lower-peak claim is made. Numeric instruction changes range -0.0154% to +0.0817%; BIND ranges -0.0836% to +0.3649%. All four BIND allocation quantities are exactly unchanged in every lane. All raw samples and shared comparison intervals are retained; no small instruction change is dismissed as noise or treated as a speedup requirement.

Deltas below are candidate minus baseline medians; requested/retained/peak are bytes, instructions are percent change.

| Case/lane | Workers | Calls | Requested | Retained | Peak | Instructions % |
|---|---:|---:|---:|---:|---:|---:|
| numeric/ungoverned | 1 | 0 | -399904 | 0 | 0 | 0.013043 |
| numeric/ungoverned | 4 | 0 | -399592 | 0 | 50156.5 | 0.008166 |
| numeric/ungoverned | 32 | 0 | -396904 | 0 | -329780 | 0.067857 |
| numeric/fuel | 1 | 0 | -398440 | 0 | -201280 | 0.017363 |
| numeric/fuel | 4 | -2 | -398624 | 0 | -45824 | 0.043829 |
| numeric/fuel | 32 | 0 | -398104 | 0 | -236260 | -0.015439 |
| numeric/scratch | 1 | 0 | -398440 | 0 | 257560 | 0.028428 |
| numeric/scratch | 4 | 0 | -398104 | 0 | 1896 | 0.026498 |
| numeric/scratch | 32 | 0 | -398104 | 0 | 1896 | 0.056948 |
| numeric/metered | 1 | 0 | -399904 | 0 | 11773 | 0.015009 |
| numeric/metered | 4 | -1 | -399852 | 0 | 256408 | 0.038518 |
| numeric/metered | 32 | 0 | -396904 | 0 | 259096 | 0.081635 |
| bind/ungoverned | 1 | 0 | 0 | 0 | 0 | 0.006942 |
| bind/ungoverned | 4 | 0 | 0 | 0 | 0 | 0.006379 |
| bind/ungoverned | 32 | 0 | 0 | 0 | 0 | 0.364830 |
| bind/stop_signal | 1 | 0 | 0 | 0 | 0 | 0.000009 |
| bind/stop_signal | 4 | 0 | 0 | 0 | 0 | -0.000826 |
| bind/stop_signal | 32 | 0 | 0 | 0 | 0 | 0.043485 |
| bind/fuel_and_stop_signal | 1 | 0 | 0 | 0 | 0 | -0.006849 |
| bind/fuel_and_stop_signal | 4 | 0 | 0 | 0 | 0 | -0.001493 |
| bind/fuel_and_stop_signal | 32 | 0 | 0 | 0 | 0 | -0.083515 |

## Inconsistent first candidate record

Original-chunks-13dd is retained as historical negative evidence, not current acceptance. Its artifact-bound instruction executable identity was ebbba6fd…, while the clean rebuilt artifact is 472c16db…. At the same clean tracked source, subsequent diagnostic and clean uninstrumented builds consistently show the buffer saving expected from the implementation. The original discrepancy has not been assigned an unverified cache/compiler cause; no cache, compiler, profile or admission setting was changed. Original reports were not overwritten. Current conclusions bind the rebuilt artifact and the verified-chunks-13dd record.
