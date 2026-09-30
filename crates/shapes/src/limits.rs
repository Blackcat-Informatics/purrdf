// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one home of the nesting bound every JSON Schema surface enforces.
//!
//! The JSON Schema importer, the TypeScript and GraphQL emitters and the Pydantic
//! package generator all walk caller-supplied schema trees recursively. They share
//! one depth ceiling so the surfaces agree on which schema is too deeply nested to
//! process, and one guard so each reports the refusal the same way.

/// The deepest schema nesting any JSON Schema surface accepts. A tree nested past
/// it is refused with an error instead of recursing without bound.
pub const MAX_SCHEMA_DEPTH: usize = 128;

/// Refuse a schema expression nested deeper than [`MAX_SCHEMA_DEPTH`].
///
/// `surface` names the emitter in the message (`"TypeScript"`, `"GraphQL"`), and
/// `path` is the JSON pointer of the offending expression. The caller lifts the
/// message into its own error type.
///
/// # Errors
///
/// Returns the refusal message when `depth` exceeds [`MAX_SCHEMA_DEPTH`].
pub fn ensure_depth(depth: usize, path: &str, surface: &str) -> Result<(), String> {
    if depth > MAX_SCHEMA_DEPTH {
        Err(format!(
            "{surface} schema expression at {path} exceeds depth {MAX_SCHEMA_DEPTH}"
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_at_the_bound_is_accepted_and_one_past_it_is_refused() {
        assert!(ensure_depth(MAX_SCHEMA_DEPTH, "/a", "TypeScript").is_ok());
        assert_eq!(
            ensure_depth(MAX_SCHEMA_DEPTH + 1, "/a", "GraphQL").unwrap_err(),
            "GraphQL schema expression at /a exceeds depth 128"
        );
    }
}
