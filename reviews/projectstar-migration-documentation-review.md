<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# `ProjectStar` migration documentation review — 2026-10-13

**Scoped source/documentation PASS for the two-file delta.** The new Unreleased breaking entry in `CHANGELOG.md` identifies the actual public source break, explicitly assigns it to a 4.0 major release, and links the new migration guide. This is consistent with the changelog and book policy that a breaking Rust API change bumps the major version. No registry, manifest version or release state is changed or claimed here.

`docs/MIGRATION-4.0.md` gives the necessary downstream adaptation: `GraphPattern` has no `#[non_exhaustive]` attribute, so exhaustive matches need a `ProjectStar` arm; both variants have the same `inner: Child<GraphPattern>` type; `GraphPattern::projected_names()` returns the existing `ProjectionNames` view for either form and `None` otherwise; `ProjectionNames::iter()` yields `&Variable` in order. The examples match the current signatures. The text correctly preserves explicit `Project`'s owned `Vec<Variable>` and warns that repeatedly collecting each nested star prefix would recreate the original cost. I found no inaccurate API example or remaining ProjectStar-specific migration-doc gap in this bounded delta.

This documentation resolves the release-classification gap identified in `projectstar-public-contract-disposition.md` for an already-authorized v4 delivery; it does not establish a 3.x source-compatible implementation. No Cargo/doc test was run for this read-only review, and the examples are deliberately marked `ignore`. The optimized package's two #531 stack-floor failures, final hosted checks, PR and integration remain open; no #365 completion is inferred.
