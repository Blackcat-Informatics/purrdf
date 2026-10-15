# Remaining original-home carrier consumers

The public RL native batch19430 is still compiling; its Shapes/core/Datalog/RDF
source graph is frozen. These independent source changes do not affect that graph.

## Phrase occurrence
crates/text/src/dictionary_relation.rs now propagates CorpusProfile through
PhraseOccurrenceRelation, Results, PhraseBatch, PhraseCursor and literal_matches.
All existing match/admission/row bodies are retained; default GraphLanguage
preserves the old public spelling and keys. The original dictionary implementation
was already profile-parametric. The existing graph_free physical fixture registers
the same phrase PF, compares quad/triple rows, and pins the independent first
document phrase span [1,3). Compiler/runtime pending.

## Official query corpus
crates/sparql-conformance/src/lib.rs moves original verdict/xfail accounting into
Summary::record_verdict and exposes record_outcome for carrier executions using
the same comparator. Original run_manifest still performs the same source
construction and verdict sequence, including unmodeled/service failures.

tests/triple_carrier.rs discovers all three official W3C manifest families.
Each QueryEval uses the original dataset loader, parser/environment/options,
in-memory service source, actual TripleDataset conversion and TripleRead fallible
engine. Only exact NamedGraphsUnsupported is classified as unsupported; all
other failures go to the original result/error comparator and xfail law.
The golden inventory names every case and classification, including non-query
cases; it is not yet generated, reviewed or accepted. A case moving between
supported and named-refusal categories changes the exact inventory, so neither
silent exclusion nor a count-only substitution is accepted. No broad corpus
run has been launched.

## Facade
The original explicit rdf/src/lib.rs core export list still lacks new Triple*
names. Add original-type exports and an actual umbrella caller after19430
terminal; do not mutate that active dependency graph.


## Current readback after public RL terminal
Session19430 terminal0,4/4 selected controls PASS. RDF now reexports the original Triple* types, and base_surface includes one actual facade parse→compact→public RL→compact publication control. These and the profile-parametric phrase path are in current selected warning-denied Clippy40697 (not yet terminal).

The corpus adapter now constructs SERVICE before any data conversion, records setup failures via original Verdict::Fail/xfail accounting (never compare_failure), and preserves original read-query/evaluate case framing. Only an actual typed NamedGraphsUnsupported enters the exclusion category. Its inventory remains ungenerated/unqualified.
