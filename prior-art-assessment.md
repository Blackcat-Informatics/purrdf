<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# #536 prior-art assessment

The captured `prior-art.md` searched the live issue/comment, ten linked issues, fourteen related forge items and two hundred recent commits; it found no prior #536 delivery. #531 and #533 own other failures from the same rejected PR #535 and do not reduce this issue's two requirements. No governing ADR is identified. Root `.baseline` forbids deferrals, and `.goals` favors Rust, hard failures and portability; repository AGENTS requires actual gates and protected integration.

Two donor fragments fit the current main source. Commit `7ee086ffd` contains a **single Shapes hunk** changing the call to `scc_component_index::<BTreeSet<usize>>(graph)`; all other commit changes are unrelated. The numeric checkout's retained `T3-536-numeric-registry.patch` changes only `crates/hash/src/dispatch.rs`: sorted registry/documentation plus a parser test accepting numeric portable/NEON and refusing an unknown numeric path. Neither donor's tests or newer source qualify the main-based #536 branch. Transfer these fragments by scope, not wholesale, then test the real consumers.

The issue's observed hosted failures are authoritative diagnosis for the old source, not evidence that either candidate passes the pinned MSRV or Arm runner. The source-level design matches the two failure sites; actual qualification remains mandatory.
