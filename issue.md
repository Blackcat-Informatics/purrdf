# Issue #580: fix(entail): retain source terms for class-expression role refusals

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

CeExtractor::role_of converts RoleHierarchyError directly into EntailError, unlike the other native DL role producers that first retain original source terms through the existing with_source helper. The recorded GMEOW default input therefore reports only private term 9336 for malformed role admission, preventing source repair and blocking the generic class-model ownership move. Route this producer through the existing source-witness home before its interner dies; preserve the typed original classification, source text and existing allocation/cancellation behavior without introducing a new grant or filtering premises. Add public Reasoner construction controls for malformed restriction roles and verify retained typed source arguments after constructor failure. Run affected public/native tests, the complete entailment library suite and strict clippy; use ghprsq for integration. This is a bounded producer repair within #558; it does not claim to complete that broader structured-error inventory. The concurrently owned #558 worktree must remain untouched.

## Comments (0)

