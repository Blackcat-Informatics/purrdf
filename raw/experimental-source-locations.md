<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Preserved diagnostic source projections

After their builds, measurements and review reads completed, both task-owned source projections were moved intact on 2026-10-07 outside the selected Stage evidence. No original main or sibling source was changed.

- Geometry control: `/home/paudley/Active/purrdf-recovery-artifacts/20261006-2253/governor-experimental-source/geometry-control-src`. This is unchanged main with only the governed nonempty-loop block selector disabled; `geometry-control.diff` is its exact experimental delta. It is a scheduling control, not historical pre-block source or a correctness baseline.
- BGP layout control: `/home/paudley/Active/purrdf-recovery-artifacts/20261006-2253/governor-experimental-source/bgp-layout-control-src`. This projects `candidate-empty-arena.diff` onto main and adds only the BGP layout filter in `bgp-layout-control.diff`. It attributes the cell-helper effect; a driver-specific exception is not the shipping design.

The Cargo source identities and original compilation paths remain recorded in the build logs. The already-built executables remain in their source-specific Cargo slots. All default-sampling logs and benchmark reports remain here; the projection trees are preserved externally to avoid including duplicate source or non-Rust tooling in Stage selection and working-tree hygiene.
