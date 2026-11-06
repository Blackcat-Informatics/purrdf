// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed inspection of original proof refusals and their native evidence.

use super::{
    DlProofError, NodeRef, ProofAlternative, ProofFact, ProofGround, ProofRole, ReservedRef,
};
use purrdf_hash::error::{ErrorValue, StructuredError};

impl StructuredError for ProofRole {
    purrdf_hash::error_visit! {
        @record self { property, inverse } => "owl-proof-role" {
            "property" => Unsigned(u128::from(self.property)),
            "inverse" => Boolean(self.inverse),
        }
    }
}

impl StructuredError for NodeRef {
    purrdf_hash::error_visit! {
        Individual(id) => "owl-proof-node-individual" { "id" => Unsigned(u128::from(*id)) },
        Reserved(root) => "owl-proof-node-reserved" { "root" => Record(root.as_ref()) },
        Anonymous(id) => "owl-proof-node-anonymous" { "id" => Unsigned(u128::from(*id)) },
    }
}

impl StructuredError for ReservedRef {
    purrdf_hash::error_visit! {
        @record self { origin, role, filler, index } => "owl-proof-reserved-root" {
            "origin" => Record(&self.origin),
            "role" => Record(&self.role),
            "filler" => Unsigned(u128::from(self.filler)),
            "index" => Unsigned(u128::from(self.index)),
        }
    }
}

impl StructuredError for ProofFact {
    purrdf_hash::error_visit! {
        Concept { node, concept } => "owl-proof-fact-concept" {
            "node" => Record(node), "concept" => Unsigned(u128::from(*concept)),
        },
        Edge { from, to, role } => "owl-proof-fact-edge" {
            "from" => Record(from), "to" => Record(to), "role" => Record(role),
        },
        Denotes { node, individual } => "owl-proof-fact-denotes" {
            "node" => Record(node), "individual" => Unsigned(u128::from(*individual)),
        },
    }
}

impl StructuredError for ProofGround {
    purrdf_hash::error_visit! {
        Concept { node, concept } => "owl-proof-ground-concept" {
            "node" => Record(node), "concept" => Unsigned(u128::from(*concept)),
        },
        SelfLoop { node, role } => "owl-proof-ground-self-loop" {
            "node" => Record(node), "role" => Record(role),
        },
        AtLeast { node, n, role, filler } => "owl-proof-ground-at-least" {
            "node" => Record(node), "n" => Unsigned(u128::from(*n)),
            "role" => Record(role), "filler" => Unsigned(u128::from(*filler)),
        },
        Equal { left, right } => "owl-proof-ground-equal" {
            "left" => Record(left), "right" => Record(right),
        },
        EqualIndividual { node, individual } => "owl-proof-ground-equal-individual" {
            "node" => Record(node), "individual" => Unsigned(u128::from(*individual)),
        },
        EqualReserved { node, root } => "owl-proof-ground-equal-reserved" {
            "node" => Record(node), "root" => Record(root),
        },
    }
}

impl StructuredError for ProofAlternative {
    purrdf_hash::error_visit! {
        @record self { atoms } => "owl-proof-alternative" {
            "atoms" => List(self.atoms.iter().map(|atom| ErrorValue::Record(atom))),
        }
    }
}

impl<E: StructuredError> StructuredError for DlProofError<E> {
    purrdf_hash::error_visit! {
        UnsaturatedIntroduction { clause, node, predecessor } => "owl-proof-unsaturated-introduction" {
            "clause" => Unsigned(*clause as u128), "node" => Record(node.as_ref()),
            "predecessor" => Record(predecessor.as_ref()),
        },
        CausalSupport { run, clash, kind, detail } => "owl-proof-causal-support" {
            "run" => Value(run.map_or(ErrorValue::Absent, |run| ErrorValue::Unsigned(run as u128))),
            "clash" => Value(clash.map_or(ErrorValue::Absent, |clash| ErrorValue::Unsigned(clash as u128))),
            "kind" => Record(kind), "detail" => Message(detail),
        },
        Stopped => "owl-proof-stopped" {},
        SchemaRecording { obstruction } => "owl-proof-schema-recording" { "obstruction" => Record(obstruction) },
        SchemaPreparation { obstruction } => "owl-proof-schema-preparation" { "obstruction" => Record(obstruction) },
        Storage(cause) => "owl-proof-storage" { "cause" => Cause(cause) },
        Malformed { detail } => "owl-proof-malformed" { "detail" => Message(detail) },
        InputMismatch { expected, stated } => "owl-proof-input-mismatch" {
            "expected" => Text(expected), "stated" => Text(stated),
        },
        ContractMismatch { expected, stated } => "owl-proof-contract-mismatch" {
            "expected" => Text(expected), "stated" => Text(stated),
        },
        UnknownClause { clause, clauses } => "owl-proof-unknown-clause" {
            "clause" => Unsigned(*clause as u128), "clauses" => Unsigned(*clauses as u128),
        },
        NotARefutation { clause, form } => "owl-proof-not-a-refutation" {
            "clause" => Unsigned(*clause as u128), "form" => Record(form),
        },
        FrameTooShort { clause, variable, frame } => "owl-proof-frame-too-short" {
            "clause" => Unsigned(*clause as u128), "variable" => Unsigned(u128::from(*variable)),
            "frame" => Unsigned(*frame as u128),
        },
        WitnessLengthMismatch { clause, derived, stated } => "owl-proof-witness-length-mismatch" {
            "clause" => Unsigned(*clause as u128), "derived" => Unsigned(*derived as u128),
            "stated" => Unsigned(*stated as u128),
        },
        WitnessMismatch { clause, position, derived, stated } => "owl-proof-witness-mismatch" {
            "clause" => Unsigned(*clause as u128), "position" => Unsigned(*position as u128),
            "derived" => Record(derived.as_ref()), "stated" => Record(stated.as_ref()),
        },
        NodeMismatch { clause, derived, stated } => "owl-proof-node-mismatch" {
            "clause" => Unsigned(*clause as u128),
            "derived" => Record(derived.as_ref()), "stated" => Record(stated.as_ref()),
        },
        Ontology { detail } => "owl-proof-ontology" { "detail" => Message(detail) },
        SourceRead(cause) => "owl-proof-source-read" { "cause" => Cause(cause) },
        TrustBaseMismatch { expected, stated } => "owl-proof-trust-base-mismatch" {
            "expected" => Text(expected), "stated" => Text(stated),
        },
        NotADisjunction { clause, form } => "owl-proof-not-a-disjunction" {
            "clause" => Unsigned(*clause as u128), "form" => Record(form),
        },
        AlternativeCountMismatch { branch, clause, derived, stated } => "owl-proof-alternative-count-mismatch" {
            "branch" => Unsigned(*branch as u128), "clause" => Unsigned(*clause as u128),
            "derived" => Unsigned(*derived as u128), "stated" => Unsigned(*stated as u128),
        },
        AlternativeMismatch { branch, position, derived, stated } => "owl-proof-alternative-mismatch" {
            "branch" => Unsigned(*branch as u128), "position" => Unsigned(*position as u128),
            "derived" => Record(derived.as_ref()), "stated" => Record(stated.as_ref()),
        },
        IllFormedIntroduction { branch, position, detail } => "owl-proof-ill-formed-introduction" {
            "branch" => Unsigned(*branch as u128), "position" => Unsigned(*position as u128),
            "detail" => Message(detail),
        },
        OutcomeCountMismatch { branch, derived, stated } => "owl-proof-outcome-count-mismatch" {
            "branch" => Unsigned(*branch as u128), "derived" => Unsigned(*derived as u128),
            "stated" => Unsigned(*stated as u128),
        },
        DanglingOutcome { branch, ordinal, detail } => "owl-proof-dangling-outcome" {
            "branch" => Unsigned(*branch as u128), "ordinal" => Unsigned(*ordinal as u128),
            "detail" => Message(detail),
        },
        BranchNotClosed { branch, ordinal } => "owl-proof-branch-not-closed" {
            "branch" => Unsigned(*branch as u128), "ordinal" => Unsigned(*ordinal as u128),
        },
        WrongAnswer { expected, stated } => "owl-proof-wrong-answer" {
            "expected" => Code(expected.as_str()), "stated" => Code(stated.as_str()),
        },
        Truncated => "owl-proof-truncated" {},
        NoCompletion => "owl-proof-no-completion" {},
        MalformedCompletion { detail } => "owl-proof-malformed-completion" { "detail" => Message(detail) },
        ClashInCompletion { clause, node } => "owl-proof-clash-in-completion" {
            "clause" => Unsigned(*clause as u128), "node" => Record(node.as_ref()),
        },
        ClauseNotSatisfied { clause, node } => "owl-proof-clause-not-satisfied" {
            "clause" => Unsigned(*clause as u128), "node" => Record(node.as_ref()),
        },
        WrongQuestion { expected, stated } => "owl-proof-wrong-question" {
            "expected" => Text(expected), "stated" => Text(stated),
        },
        AnswerNotCovered { detail } => "owl-proof-answer-not-covered" { "detail" => Message(detail) },
        ReceiptMismatch { detail } => "owl-proof-receipt-mismatch" { "detail" => Message(detail) },
        UnlicensedMerge { merge, detail } => "owl-proof-unlicensed-merge" {
            "merge" => Unsigned(*merge as u128), "detail" => Message(detail),
        },
        NotAnIdentification { merge, derived } => "owl-proof-not-an-identification" {
            "merge" => Unsigned(*merge as u128), "derived" => Record(derived.as_ref()),
        },
        MergeCauseMismatch { merge, derived, stated } => "owl-proof-merge-cause-mismatch" {
            "merge" => Unsigned(*merge as u128), "derived" => Code(derived.as_str()),
            "stated" => Code(stated.as_str()),
        },
        MergeNodeMismatch { merge, derived, stated } => "owl-proof-merge-node-mismatch" {
            "merge" => Unsigned(*merge as u128),
            "derived" => List([&derived.0, &derived.1].into_iter().map(|node| ErrorValue::Record(node))),
            "stated" => List([&stated.0, &stated.1].into_iter().map(|node| ErrorValue::Record(node))),
        },
        BlockingSignatureMismatch { blocked, blocker, detail } => "owl-proof-blocking-signature-mismatch" {
            "blocked" => Record(blocked.as_ref()), "blocker" => Record(blocker.as_ref()),
            "detail" => Message(detail),
        },
        Canonicalization(cause) => "owl-proof-canonicalization" { "cause" => Cause(cause) },
    }
}
