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

/// The fewest bytes an emitter may write for one artifact, whatever its input.
pub const MIN_EMITTED_BYTES: usize = 16 * 1024 * 1024;

/// The most bytes an emitter may write per byte of the compiled schema it
/// reads. Each emitter writes a bounded rendering of each schema node it
/// reads, so its output grows with its input; more than this is amplification,
/// which is what an emitter's output bound guards against.
pub const EMITTED_BYTES_PER_INPUT_BYTE: usize = 4;

/// The most bytes one artifact emitted from an `input_bytes`-byte compiled
/// schema may hold: [`EMITTED_BYTES_PER_INPUT_BYTE`] per input byte, and at
/// least [`MIN_EMITTED_BYTES`].
///
/// The emitters bound what they write, not what they read. Their input is a
/// compiled schema already held in memory, which they walk once under the
/// shared depth, definition, field and enumeration ceilings, so their work is
/// linear in it; a fixed input ceiling would refuse a schema as large as the
/// compiler legitimately produces (QUDT's is 55 MB) while bounding nothing the
/// input does not already bound.
#[must_use]
pub const fn emitted_bytes_bound(input_bytes: usize) -> usize {
    let scaled = input_bytes.saturating_mul(EMITTED_BYTES_PER_INPUT_BYTE);
    if scaled > MIN_EMITTED_BYTES {
        scaled
    } else {
        MIN_EMITTED_BYTES
    }
}

/// Refuse an emitted artifact larger than [`emitted_bytes_bound`] of its
/// input.
///
/// `what` names the artifact in the message (`"GraphQL SDL"`). The caller
/// lifts the message into its own error type.
///
/// # Errors
///
/// Returns the refusal message when `output_bytes` exceeds the bound.
pub fn ensure_emitted(input_bytes: usize, output_bytes: usize, what: &str) -> Result<(), String> {
    let bound = emitted_bytes_bound(input_bytes);
    if output_bytes > bound {
        Err(format!(
            "generated {what} uses {output_bytes} bytes, more than the {bound}-byte bound for a \
             {input_bytes}-byte compiled schema"
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_at_the_derived_bound_is_accepted_and_one_past_it_is_refused() {
        // A small input is bounded by the floor.
        assert!(ensure_emitted(10, MIN_EMITTED_BYTES, "SDL").is_ok());
        assert!(ensure_emitted(10, MIN_EMITTED_BYTES + 1, "SDL").is_err());
        // A large input scales the bound: a 55 MB schema may emit 220 MB.
        let input = 55_352_357;
        let bound = input * EMITTED_BYTES_PER_INPUT_BYTE;
        assert_eq!(emitted_bytes_bound(input), bound);
        assert!(ensure_emitted(input, bound, "SDL").is_ok());
        assert!(
            ensure_emitted(input, bound + 1, "SDL")
                .unwrap_err()
                .contains("more than the 221409428-byte bound")
        );
        assert_eq!(emitted_bytes_bound(usize::MAX), usize::MAX);
    }

    #[test]
    fn depth_at_the_bound_is_accepted_and_one_past_it_is_refused() {
        assert!(ensure_depth(MAX_SCHEMA_DEPTH, "/a", "TypeScript").is_ok());
        assert_eq!(
            ensure_depth(MAX_SCHEMA_DEPTH + 1, "/a", "GraphQL").unwrap_err(),
            "GraphQL schema expression at /a exceeds depth 128"
        );
    }
}
