// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Emoji recognition checked against independent official Unicode input data.

#[path = "support/emoji_predicate_cases.rs"]
mod cases;

#[test]
fn every_emoji_spelling_and_protected_scalar_matches_the_input_data() {
    cases::every_emoji_spelling_and_protected_scalar_matches_the_input_data();
}
