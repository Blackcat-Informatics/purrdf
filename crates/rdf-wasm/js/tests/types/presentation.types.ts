// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Compile-only checks for the typed presentation a failure carries (`npm run typecheck`).
// Nothing here runs.
// Why not Rust: checks the published TypeScript declarations that npm consumers compile against, which only tsc can type-check

import type {
  AsyncJobError,
  DiagnosticParameterValue,
  DiagnosticPresentation,
  PurrdfError,
} from "@blackcatinformatics/purrdf";

declare const thrown: PurrdfError;
declare const rejected: AsyncJobError;

const presentation: DiagnosticPresentation | undefined = thrown.presentation;
const messageId: string | undefined = presentation?.messageId;
const at: DiagnosticParameterValue | undefined = presentation?.parameters["at"];
if (at?.kind === "unsigned") {
  const exact: string = at.value;
  void exact;
}
if (at?.kind === "boolean") {
  const flag: boolean = at.value;
  void flag;
}
const cause: DiagnosticPresentation | undefined = rejected.presentation?.detail;
// @ts-expect-error the presentation is read-only
thrown.presentation = undefined;
// @ts-expect-error an exact integer is a decimal string, never a binary64 number
const lossy: number | undefined = presentation?.parameters["at"]?.value;
void messageId;
void cause;
void lossy;
