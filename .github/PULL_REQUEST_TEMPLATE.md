<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
## Summary

<!-- What does this change, and why? Link the issue if there is one. -->

## Checklist

- [ ] `cargo fmt --all` has been run and `make check` passes (fmt + clippy + build + tests + hygiene)
- [ ] No new Cargo features introduced anywhere in the workspace
- [ ] No hand-edits to `generated/` or `vectors/` (regenerate via `make metadata` instead)
- [ ] If touching a release crate: wasm32 build stays clean (`make wasm`)
- [ ] Docs / CHANGELOG updated where the change is user-visible
- [ ] If claiming a performance improvement: criterion benches extended to cover it
