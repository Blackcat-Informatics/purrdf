# Qualified shared i586 fixture applicability

Root requested a source-only applicability check while the542 mandatory gate
continues. No542 source file was edited.

The comparison from PR541's pre-correction55e9dc650 to this checkout's HEAD is
empty for distance/mod.rs, distance/binary64.rs, distance/binary64_tests.rs and
xsd/ieee/environment.rs. Thus the same faulty test expectation, test-only core
bypass, public Exact/Reassociated admission and XSD scope/probe homes are
present. A check-only application of the single-file27e9c2c7b patch succeeds.
The current542 mutable/test delta does not touch those homes.

Actual evidence: PR541 run38353988504 cross-arch job115152530564 passes the
i586 distance family39/39, including the corrected faulty-token test, and the
subsequent i586 geographic replay. Root already source-reviewed the correction:
existing probe_with receives the deliberately faulty token, exact failed rows
and bits remain, and public guarded paths work and restore the full caller word.
There is no new bypass or production change. The exact one-file correction is
therefore applicable without repeating the previous source diagnosis or a broad
local suite; it must still be transferred through a normal hook after the live
gate/source checkpoint permits, and current542 hosted results remain required.

Original raw receipt is retained in PR541 Stage/tasks/hosted-27e9-cross-arch.log
and copied to539 Stage/tasks/shared-i586-27e9-cross-arch.log. This assessment
does not claim542's unchanged fixture is already fixed or its hosted job passes.

The raw original log is also copied into this Stage as
shared-i586-27e9-cross-arch.log for independent durable capture. The exact
single-file correction is now standalone published commit2d01ba1b5 on539,
normal hook/push terminal0, so no mixed documentation cherry-pick is needed.

Root retained the exact single-file diff from that commit as
shared-i586-fixture.patch. The installed Git emits prefix-free file paths in this
diff; `git apply -p0 --check` passes on the current source. No patch has been
applied while the original full gate is running.
