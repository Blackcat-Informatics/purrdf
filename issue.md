# Issue #683: purrdf-text: a `TextIndex` delta update whose work follows the change, not the source

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Problem

#538 delivered `TextIndex::with_delta`, and its closing note says "source scans, sorting, hashing
and metadata traversal remain". At a real corpus size that remainder is almost the whole cost. An
update after a one-claim write costs about what a fresh build does, because `with_delta` collects
every literal row of the next snapshot (`collect_rows(next, ..)`) to recompute the source
fingerprint and coverage and to validate the delta, before it analyses the changed documents.

## Measured

Katamari (`katamari-query-purrdf`), which keeps each view's index and applies `with_delta` with the
statements added and removed since. The corpus is memetics' synthetic import corpus, its dreams
store: 953,604 claims, a real shape and none of a person's content. The run is
`katamari-import-scale --corpus .. --fraction 1/1`, release, frame pointers, `perf record -F 49
--call-graph fp`. The run's own counters show one build, one update, and no deltas refused.

| Work | Samples |
|---|---|
| fresh build (`TextIndex::from_dataset`) | 1,005 |
| update after one write (`TextIndex::with_delta`) | 948, i.e. 94% of a build |
| of which `collect_rows` over the next snapshot | 909, i.e. 96% of the update |
| of which `merge_changed`, `validate_delta`, `digest_rows`, `compute_fingerprint` | under 40 together |

Analysing the changed documents is not measurable at this size. The scan of the unchanged source
is the cost. Wall clock agrees roughly: 51 s first text query, 54 s after the write. Wall clock on
this host is noisy, so the sample counts are the evidence.

## Ask

An update whose work is proportional to the delta. The analysis already is; the source
accounting is what remains:

- **Keep the source fingerprint incrementally.** Use a multiset hash, such as a sum of row
  digests in a group, so adding and removing rows updates it in O(delta). `validate_delta` then
  checks the delta against the old fingerprint and the new one without reading the source.
- **Keep coverage incrementally**, from the delta's nonliteral statements, as the delta already
  carries them.
- **Changed documents only.** Collect their rows from `next` by subject probe, not by a full
  `collect_rows`.

The oracle stays the one #538 already uses: the updated index equals a fresh build over the same
dataset, statistics and fingerprints included. Katamari's 150-step walk
(`maintained_full_text_answers_as_an_index_built_for_the_snapshot`) holds that today and would
hold the new path to it.


## Comments (0)

