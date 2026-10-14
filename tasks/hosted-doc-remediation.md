# Hosted documentation remediation

PR544 original doc job115487437495 failed on two unresolved IriError links in
global.rs at505 and656. Raw job output is hosted-doc-first.log. Actual methods
return imported IriReadError; their two error paragraphs now identify Lexical
and Storage variants. No runtime body changed.

Standard make doc28542 cleared core and exposed one previously unreached
Datalog plan.rs TermId link whose import is test-only. Qualifying its link to
crate::id::TermId matches its actual home and adjacent PartitionId link.
The original failed local log is hosted-doc-corrected-local.log. Standard
make doc48944 with one Cargo job then passed all49 documentation outputs in1m31;
log hosted-doc-corrected-final.log. Root independently reviewed the three
paragraph/link changes; no runtime retest or full make check was needed.

Actual index10076 reports40 stale proofs and retains all149 closure paths.
Only source identities for global.rs and Datalog plan.rs changed. The reviewed
ledger update changes40 constructor aggregates60cd43cc38bd7131 to49c7c48bf647382f,
seven global authority references ee49b9f69b8a6429 tobbf736546130634d, and the
unchanged value_index sink921e8adc54613316 line888 to890. Its other exact sink
find remains432. Datalog plan.rs is not a proof authority. All theorem reasons,
classes, sink hashes, constructor/mint bodies and owner boundaries are unchanged.
The original stale JSON is retained in hosted-doc-stale-index.json.

Final index/helper24646 is terminal0: errors[], aggregate49c7c48bf647382f and
unchanged helper82/118 PASS. Format and diff checks pass; normal hook78382 and push76582 passed at10316f2c1. Hosted doc remains failed on03f46 until a
new current-head run qualifies the correction; original CI is not relabeled.
