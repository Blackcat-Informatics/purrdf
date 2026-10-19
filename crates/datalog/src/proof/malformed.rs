// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original graph and wire operands of malformed proof refusals.

/// A native defect in the proof graph or its binary encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofMalformed {
    /// The checked root was not established.
    RootUnestablished {
        /// Original root handle.
        root: u64,
    },
    /// A root is outside its original arena.
    RootOutside {
        /// Original root handle.
        root: u64,
        /// Actual arena length.
        nodes: usize,
    },
    /// A stored premise is not earlier than its use.
    PremiseOrder {
        /// Original premise handle.
        premise: u64,
        /// Using node position.
        position: usize,
    },
    /// An earlier premise has not been established.
    PremiseUnestablished {
        /// Original premise handle.
        premise: u64,
        /// Using node position.
        position: usize,
    },
    /// No immutable base was supplied for a negative premise.
    MissingBase {
        /// Cited rule.
        rule: usize,
        /// Original body position.
        body_position: usize,
    },
    /// The wire names a different proof encoding.
    Tag {
        /// Original tag bytes.
        actual: Vec<u8>,
        /// Required encoding tag.
        expected: purrdf_hash::Domain,
    },
    /// No proof node was encoded.
    Empty,
    /// A decoded premise does not precede its use.
    ForwardReference {
        /// Original wire reference.
        premise: usize,
        /// Using node position.
        position: usize,
    },
    /// A wire node has an unknown kind byte.
    Kind {
        /// Original byte.
        kind: u8,
        /// Original node position.
        position: usize,
    },
    /// Bytes remain after the declared final node.
    Trailing {
        /// Remaining original bytes.
        bytes: usize,
    },
    /// A field extends beyond the available input.
    Truncated {
        /// Required byte count.
        required: usize,
        /// Available byte count.
        remaining: usize,
    },
    /// A wire count cannot be addressed on this target.
    Length {
        /// Original wire count.
        value: u64,
        /// Target address limit.
        maximum: usize,
    },
    /// A term's original lexical bytes are not UTF-8.
    Utf8 {
        /// Rejected original bytes.
        bytes: Vec<u8>,
        /// Original decoding cause.
        cause: core::str::Utf8Error,
    },
}

impl purrdf_hash::error::StructuredError for ProofMalformed {
    purrdf_hash::error_visit! {
        RootUnestablished { root } => "proof-root-unestablished" { "root" => Unsigned(u128::from(*root)) },
        RootOutside { root, nodes } => "proof-root-outside" { "root" => Unsigned(u128::from(*root)), "nodes" => Unsigned(*nodes as u128) },
        PremiseOrder { premise, position } => "proof-premise-order" { "premise" => Unsigned(u128::from(*premise)), "position" => Unsigned(*position as u128) },
        PremiseUnestablished { premise, position } => "proof-premise-unestablished" { "premise" => Unsigned(u128::from(*premise)), "position" => Unsigned(*position as u128) },
        MissingBase { rule, body_position } => "proof-missing-base" { "rule" => Unsigned(*rule as u128), "body_position" => Unsigned(*body_position as u128) },
        Tag { actual, expected } => "proof-encoding-tag" { "actual" => Bytes(actual), "expected" => Bytes(expected.as_bytes()) },
        Empty => "proof-empty" {},
        ForwardReference { premise, position } => "proof-forward-reference" { "premise" => Unsigned(*premise as u128), "position" => Unsigned(*position as u128) },
        Kind { kind, position } => "proof-node-kind" { "kind" => Unsigned(u128::from(*kind)), "position" => Unsigned(*position as u128) },
        Trailing { bytes } => "proof-trailing" { "bytes" => ByteCount(*bytes as u128) },
        Truncated { required, remaining } => "proof-truncated" { "required" => ByteCount(*required as u128), "remaining" => ByteCount(*remaining as u128) },
        Length { value, maximum } => "proof-length" { "value" => Unsigned(u128::from(*value)), "maximum" => Unsigned(*maximum as u128) },
        Utf8 { bytes, cause } => "proof-utf8" { "bytes" => Bytes(bytes), "cause" => Cause(cause) },
    }
}

impl core::fmt::Display for ProofMalformed {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::RootUnestablished { .. } => {
                f.write_str("the root of a proof is reachable from itself")
            }
            Self::RootOutside { .. } => f.write_str("the root is outside this arena"),
            Self::PremiseOrder { .. } => {
                f.write_str("a premise must be interned before the step it justifies")
            }
            Self::PremiseUnestablished { .. } => {
                f.write_str("a premise was not established before the step that uses it")
            }
            Self::MissingBase { .. } => {
                f.write_str("a base premise requires an explicit immutable base context")
            }
            Self::Tag { .. } => {
                f.write_str("the proof encoding tag is absent or from another layout")
            }
            Self::Empty => f.write_str("a proof has at least one node"),
            Self::ForwardReference { .. } => {
                f.write_str("a premise reference must point at an earlier node")
            }
            Self::Kind { kind, .. } => write!(f, "unknown proof node kind {kind}"),
            Self::Trailing { .. } => f.write_str("trailing bytes after the proof's last node"),
            Self::Truncated { .. } => f.write_str("the proof encoding ends inside a field"),
            Self::Length { .. } => {
                f.write_str("a length in the proof encoding exceeds this target's usize")
            }
            Self::Utf8 { .. } => f.write_str("a term surface in the proof encoding is not UTF-8"),
        }
    }
}

impl std::error::Error for ProofMalformed {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::Utf8 { cause, .. } = self {
            Some(cause)
        } else {
            None
        }
    }
}
