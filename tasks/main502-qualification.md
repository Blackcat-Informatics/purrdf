# Actual combined tooling qualification

The detached candidate at /opt/purrdf-xpath-448-integration.4VgRKW/tree
used the normal no-commit merge of main b9463ff883d3313fae17eff6ad5dfde39ac87145
into head71cfecc6ea665722da7c7974e20d6682dde7b367. The clean index tree was
5336f9aa05d0e113423da652e0253df4eb811d5c, exactly the assessed candidate.

Actual root session6713: CARGO_BUILD_JOBS=8 make helpers-hygiene, terminal0.
Full output is main502-helpers-hygiene.log; all native/shared-helper and hash
domain self-tests/census completed successfully, including80 registered domains.

Actual root session94893: CARGO_BUILD_JOBS=8 cargo run -q --locked -p
helper-census -- --glossary-gate, terminal0 on the default candidate root.
Full output is main502-glossary-gate.log:69 rows,43 rejections,24 K tokens
respected by4313 translated units, three tracked translated Markdown documents,
1716 self-tests. These are the native gate's actual units, not a claimed rerun
of the historical render gate's3039-entry count.

No source fix was necessary. The normal uncommitted merge was aborted after
qualification; the detached candidate was clean before normal removal. These
two demonstrations close the two execution findings in the integration
assessment. Final independent adjudication, fresh forge gates and ghprsq remain
separate steps. Existing affected runtime/API/render evidence is reused only
for the unchanged implementation scope described in that assessment.
