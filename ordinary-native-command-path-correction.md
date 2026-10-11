# Native rerun command path correction

The second native attempt used Stage's Cargo wrapper rather than the managed
toolchain binary used for the first run. It selected a distinct managed slot and
started rebuilding unchanged dependencies. Root interrupted only that owned
Cargo process (terminal 130). This is not a validation result.

The third attempt restores the original managed-toolchain PATH, target/build
paths, warnings-as-errors flags, two-job limit, and bounded systemd scope. It
reuses compiled artifacts and collects independent runtime failures with
`--no-fail-fast`. Receipts remain separate; the original failed benchmark run
and the interrupted second attempt have not been relabeled as passes.
