// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//! Official grapheme/emoji vectors and integrated source-aligned analysis.
#[path = "support/analysis_cases.rs"]
mod cases;
#[test]
fn unicode17_grapheme_and_all_emoji() {
    cases::official_grapheme_and_emoji_cases();
}
#[test]
fn ordered_laws_and_source_alignment() {
    cases::ordered_analysis_and_alignment_cases();
}
