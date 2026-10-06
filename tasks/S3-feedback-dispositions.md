The final review summary's workflow-separation warning is declined. This branch's
required ordinary assembly gate failed generated-document parity. The G3 changes
provide the supported hosted compiler execution and exact evidence needed to
diagnose and repair that failure; repository ownership requires fixing failures
the change depends on. The optional projection invokes the existing make gate,
and failed-shard diagnostics preserve its actual inputs. Normal seven-shard
qualification, selectors, instruction counting, floors and aggregate are unchanged.
Separating this repair would leave this issue's required gate unsatisfied.

The summary's claim that full-matrix qualification is still needed is superseded
by completed projection run 37532305791 and final ordinary CI run 37537944878.
All 110 sites pass on all seven configurations; each ordinary report matches the
qualified source, manifest, compiler and generated cells, and the ordinary
aggregate passes. Final CI and Docs both completed successfully at head
d48be7c960adfeaba44d642630cde4a66d7e86e7, testing candidate tree
5d150fa4076cc75e293b517f79dd080b6ea807ed.

The updated 40.49%/80% documentation warning remains declined for the previously
posted reason: the default bot percentage is not a repository contract. Public
APIs, error behavior and costs are documented, and the actual warnings-denied
documentation/lint gates pass. The actionable descriptor finding is repaired and
its review thread resolved. No source finding remains open in the complete
captured review surface. Independent final acceptance and merge/archive gates
are being completed separately.
