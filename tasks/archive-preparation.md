# Archive preparation during qualification

The selected Stage currently contains 293 regular files, 1,528,838 bytes, with
no symlinks. Source patches, manifests, native/portable/conformance logs, actual
large-run receipt, all 90 instruction CSV/log pairs and the method/results are
inside this directory. Standard configured build artifacts remain outside the
archive; producing source and commands are retained for replay. No whole build
tree or generated large corpus is copied here.

The current selected files have no credential-shaped GitHub bytes or unredacted
scope query parameters. This is an inventory snapshot, not stable final capture:
the full gate and forge review are still producing evidence. Refresh material
review/CI inputs and credential handling, stop writers, then capture through
ghprsq only after all required prerequisites pass. Verify essential archived
content before cleanup. No signing/publication/capture was attempted here.
