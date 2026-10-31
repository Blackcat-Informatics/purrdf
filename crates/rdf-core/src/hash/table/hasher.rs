// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

/// The workspace fixed-key native default for maps and sets.
/// It performs no runtime seeding and uses the same framing as explicit callers.
pub type DefaultHashBuilder = purrdf_hash::fixed::FixedState;
