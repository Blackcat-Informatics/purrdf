<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Chinese domain relevance fixture

Original synthetic documents, queries, word-span annotations and relevance judgments,
written before inspecting baseline-tokenizer or ranker outputs. No dictionary entries
or retrieval outputs supplied the labels. Graded judgments use 3 for the direct topic,
2 for substantive supporting context, and 0 for deliberately judged distractors.
Unlisted pairs in this original closed corpus are nonrelevant. These labels are
independent of the algorithms, but are not a native-speaker consumer review or a
population estimate. They are a small reproducible technical, symbolic and memetics
acceptance corpus, including unknown compounds, traditional text, supplementary Han,
mixed identifiers and emoji. Do not tune the dictionary or fusion weights on these
held-out rows. Library and data remain under the stated licenses.

`cargo run -p purrdf-retrieval --example text_relevance --locked -- <artifact-directory>`
compares the complete baseline lexical producer, the independent Han producer and
explicit equal-weight reciprocal-rank fusion at k=60. It reports exact-word
segmentation precision/recall/F1 and retrieval P@3, recall@3, F1@3, Hit@3, MRR and
nDCG@3 by stratum. Metrics use host floating point only; every production score and
fusion contribution uses the native exact fixed-point engine.

The same host evaluator accepts an external document TSV, query TSV and TREC qrels.
Its `judged-pool` mode filters each ranking to that query's explicitly judged
candidates before fusion and scoring. This is judged-pool ranking evidence, not a
full-corpus retrieval result. External corpus texts are never included here.
