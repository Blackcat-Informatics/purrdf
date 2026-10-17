feat(text): support exact literal delta updates

Consumers previously rebuilt the complete text index after every write.
Add an owned, source-bound TextIndex update that analyzes changed documents,
merges their lexical and auxiliary postings, and retains unchanged token,
alignment, gram and pronunciation data. Verify the effective multiset delta
before changing tables; preserve RDF layers, carrier profiles, exact ranking
statistics, canonical identifiers and fingerprints.

Reuse the existing analysis, scorer, fingerprint, partition and field homes.
Share one ordered join with the existing pronunciation union. Keep snapshot
copying explicit and document remaining source-scan/hash/table-merge costs.
No runtime dependency, semantic feature or timing claim is added.

Validation already observed: 353 text-package tests in 33 suites; 195 chained
fresh-build comparisons; exact changed-document analysis and retained-buffer
controls; public native and actual WASM execution; strict affected compilation;
helper census; normal commit and current-main integration hooks. CodeRabbit
reviews all twelve changed files at the integrated candidate with no actionable
finding; complete native review/inline/thread endpoints contain no open threads.

Pending before final notes: terminal full local gate, final hosted checks and
integration assessment. This draft does not authorize or claim merge.

Published-candidate CI exposed the native anyURI primitive bypassing the old
cast-source classification. The working repair restores both URI source and
target admission, uses the native parsed value for URI-to-string collapse, and
avoids duplicate collapse for string-to-URI. The existing operator table and
governed portable target carry the controls. These changes remain uncompiled;
the published source review does not yet cover them. The issue 571 audit traces
the native model and actual downstream replacement already merged elsewhere;
its closure is not yet added to this draft.

Closes #538.
