# Pinned sheval inventory

Read-only upstream evidence, not a vendored corpus or qualified execution.

Repository: https://github.com/cogsl/sheval
Revision: f48f9f693ff908bba63f04334cb023e0d1f0cd2c
Captured metadata/tree/manifest/README are beside this file. No engine code,
binaries or fixtures were copied into the shipping source tree.

The recursive_shapes manifest has 34 engine entries: 17 SHACL and 17 ShEx.
The SHACL entries are bsep1–4, reach1, reach2, safe1–2, nstrat1–2, cons1–2,
fresh, dual1–2 and norec1–2. reach2's name is shared between its two engine
entries, so a replay identity must include the engine. The bsep files are named
sa1–4; the nstrat files are nostratnocycle and nostratcycle; cons uses
consistency1–2. Do not infer filenames from case names.

The tree has 21 RDF/SHACL fixture pairs. Four are not manifest entries:
example21, isolation, resilience and resilience_fixed. The issue explicitly
requests the paper's Example 2 as well as every named manifest family; retain
that example in acceptance, rather than silently reducing coverage to 17.
Readback of example21 labels it "Example 2.1" and gives an acyclic T/S catalog
over a two-node p-cycle. It is not the workshop paper's Example 2, which defines
mutually recursive user/person shapes over the social-network data. Do not use
example21 as a substitute for the explicitly requested recursive example. Its
captured data/shapes files are evidence only, outside the shipping corpus.
The three other unregistered fixture pairs have no registered mode expectations
in the captured manifest and are not evidence of additional passing cases.

The API returns no repository license metadata; the tree's LICENSE files belong
to bundled third-party binaries, not this fixture population. No license or
permission claim is inferred from those bundled files. This inventory does not
establish a delivery blocker: continue the native semantic work, and author
specification-derived expectations/fixtures with the paper cited correctly.
Do not publish upstream implementation bodies or treat a missing license as one.

This is sheval-specific work for recursive semantics. It does not touch the
separately owned W3C corpus, EARL reports or submissions.
