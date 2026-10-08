# Issue #475: Benchmarks: replace the Java LUBM generator with a deterministic Rust generator

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

Owner rule: running Java on this host is forbidden, and PurRDF exists to replace Java RDF tooling. The LUBM lane still depends on the Java UBA generator. Replace it with a deterministic Rust LUBM generator that is byte-identical for a given seed and university count, matching the UBA output's statistics and schema. Then remove every Java and JRE dependency from the scripts, tests, CI and docs.

## Comments (0)

