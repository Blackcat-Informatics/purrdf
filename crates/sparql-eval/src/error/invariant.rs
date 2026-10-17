// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native evaluator invariants, classified before presentation.

/// The original category expected by a VM stack operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum StackExpectation {
    /// A term value.
    Term,
    /// A term or a structural triple term.
    TermOrTriple,
    /// An effective Boolean value.
    Boolean,
    /// An unfinished membership operation.
    OpenIn,
    /// A string argument.
    String,
}

impl StackExpectation {
    const fn description(self) -> &'static str {
        match self {
            Self::Term => "a term",
            Self::TermOrTriple => "a term or a triple term value",
            Self::Boolean => "an effective boolean value",
            Self::OpenIn => "an open IN",
            Self::String => "a string argument",
        }
    }
}

impl purrdf_hash::error::StructuredError for StackExpectation {
    purrdf_hash::error_visit! {
        Term => "vm-stack-term" {},
        TermOrTriple => "vm-stack-term-or-triple" {},
        Boolean => "vm-stack-boolean" {},
        OpenIn => "vm-stack-open-in" {},
        String => "vm-stack-string" {},
    }
}

// Each static native reason has one declaration for its code and its original
// presentation. Repeated producer sites name that reason instead of allocating
// the same sentence or declaring their own visitor/presentation mappings.
macro_rules! invariants {
    ($($variant:ident => $code:literal, $message:literal;)+) => {
        /// A classified native evaluator invariant failure, without allocated text.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[non_exhaustive]
        pub enum EvaluatorInvariant {
            $(#[doc = concat!("Refusal with inspection code `", $code, "`.")]
            $variant,)+
            /// A VM operation found a stack entry of another category.
            UnexpectedStackValue {
                /// The original expected category, before any presentation.
                expected: StackExpectation,
            },
        }

        impl purrdf_hash::error::StructuredError for EvaluatorInvariant {
            purrdf_hash::error_visit! {
                $($variant => $code {},)+
                UnexpectedStackValue { expected } => "eval-invariant-stack-value" {
                    "expected" => Record(expected),
                },
            }
        }

        impl core::fmt::Display for EvaluatorInvariant {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                match self {
                    $(Self::$variant => formatter.write_str($message),)+
                    Self::UnexpectedStackValue { expected } => write!(
                        formatter,
                        "an expression program found a stack entry that is not {}",
                        expected.description(),
                    ),
                }
            }
        }

        impl std::error::Error for EvaluatorInvariant {}
    };
}

invariants! {
    LayeredLateralBodyMissing => "eval-invariant-layered-lateral-body-missing", "layered LATERAL has no copied body";
    LateralPlaceholderOutsideCopy => "eval-invariant-lateral-placeholder-outside-copy", "a nested LATERAL placeholder was evaluated outside the substituted copy that left it";
    UngovernedBgpTruncated => "eval-invariant-ungoverned-bgp-truncated", "an ungoverned uncapped BGP driver unexpectedly truncated";
    UngovernedBgpCapped => "eval-invariant-ungoverned-bgp-capped", "an ungoverned uncapped BGP driver unexpectedly reached a cap";
    EmptyFoldArguments => "eval-invariant-empty-fold-arguments", "FOLD reached evaluation with an empty exprlist; AggregateExpression::new \
                 admits only one argument (the cdt:List form) or two (the cdt:Map form)";
    EvidenceOwnerPoisoned => "eval-invariant-evidence-owner-poisoned", "poisoned native evidence owner";
    BorrowedChildOwnerMissing => "eval-invariant-borrowed-child-owner-missing", "a borrowed child needs its original owner";
    CompletedLateralReentered => "eval-invariant-completed-lateral-reentered", "completed LATERAL entered another row";
    ExistsFilterMissing => "eval-invariant-exists-filter-missing", "EXISTS parent lost its FILTER";
    FilterCompletionMissing => "eval-invariant-filter-completion-missing", "missing FILTER completion";
    LateralParentPatternMissing => "eval-invariant-lateral-parent-pattern-missing", "LATERAL parent lost its pattern";
    LateralCompletionMissing => "eval-invariant-lateral-completion-missing", "missing LATERAL completion";
    OptionalPatternMissing => "eval-invariant-optional-pattern-missing", "OPTIONAL frame lost its pattern";
    FilterPatternMissing => "eval-invariant-filter-pattern-missing", "FILTER frame lost its pattern";
    BorrowedFilterExpressionMissing => "eval-invariant-borrowed-filter-expression-missing", "borrowed FILTER frame lost its expression";
    LateralPatternMissing => "eval-invariant-lateral-pattern-missing", "LATERAL frame lost its pattern";
    CompletionMissing => "eval-invariant-completion-missing", "missing ordinary completion";
    JoinSeedOutsidePlan => "eval-invariant-join-seed-outside-plan", "a native join seed was not produced by its positive plan";
    SeededRelationOutsidePlan => "eval-invariant-seeded-relation-outside-plan", "a seeded relation is outside its positive plan";
    RetainedBgpNotLeaf => "eval-invariant-retained-bgp-not-leaf", "a retained BGP must be a BGP leaf";
    EvidenceSearchFrameMissing => "eval-invariant-evidence-search-frame-missing", "original evidence search frame disappeared";
    VerdictEvidenceSourceMissing => "eval-invariant-verdict-evidence-source-missing", "verdict lost its original evidence source";
    FilterExpressionOwnerChanged => "eval-invariant-filter-expression-owner-changed", "FILTER changed its expression owner";
    ProbePatternMissing => "eval-invariant-probe-pattern-missing", "a probe child lost its prepared pattern";
    DefinitionPatternMissing => "eval-invariant-definition-pattern-missing", "a definition child lost its prepared pattern";
    DeferredPatternMissing => "eval-invariant-deferred-pattern-missing", "a deferred child lost its prepared pattern";
    StringPredicateAfterEvaluation => "eval-invariant-string-predicate-after-evaluation", "a string predicate was applied to evaluated argument values";
    CopiedPatternChildMissing => "eval-invariant-copied-pattern-child-missing", "a copied node's child is absent from its owner";
    MembershipTermMissing => "eval-invariant-membership-term-missing", "native membership candidate lost its term";
    RankedCandidateTermMissing => "eval-invariant-ranked-candidate-term-missing", "native scoped ranking candidate lost its term";
    ScopedRowDeclarationMissing => "eval-invariant-scoped-row-declaration-missing", "native scoped row lost its original declaration";
    CustomAggregateInBuiltinDispatch => "eval-invariant-custom-aggregate-in-builtin-dispatch", "AggregateFunction::Custom reached the built-in dispatch arm; the Custom \
                 short-circuit at the top of eval_aggregate should have already handled it";
    FoldAggregateInBuiltinDispatch => "eval-invariant-fold-aggregate-in-builtin-dispatch", "AggregateFunction::Fold reached the built-in dispatch arm; the FOLD \
                 short-circuit at the top of eval_aggregate should have already handled it";
    ProducerInvocationScopeMissing => "eval-invariant-producer-invocation-scope-missing", "lossy producer has no original invocation search scope";
    ServiceEvidenceFrameMissing => "eval-invariant-service-evidence-frame-missing", "SERVICE has no original evidence search frame";
    FunctionEvidenceRowsMissing => "eval-invariant-function-evidence-rows-missing", "SPARQL function lost its original evidence rows";
    VmEvidenceOperandsOutsideStack => "eval-invariant-vm-evidence-operands-outside-stack", "VM evidence operands exceed the original value stack";
    VmEvidenceResultSlotMissing => "eval-invariant-vm-evidence-result-slot-missing", "VM evidence result has no original value slot";
    VmConditionalEvidenceMissing => "eval-invariant-vm-conditional-evidence-missing", "VM evidence lost its original conditional record";
    ExistsPatternMissing => "eval-invariant-exists-pattern-missing", "an EXISTS instruction was run by a link that holds no pattern for it";
    ExistsExpressionMissing => "eval-invariant-exists-expression-missing", "an EXISTS site no longer names its linked expression";
    CopiedExpressionChildMissing => "eval-invariant-copied-expression-child-missing", "copied expression path no longer names a child";
    CopiedExpressionMissing => "eval-invariant-copied-expression-missing", "copied expression path no longer names an expression";
    ExistsExpressionOwnerChanged => "eval-invariant-exists-expression-owner-changed", "an EXISTS link was run with a different expression owner";
    ExistsExpressionOwnerMissing => "eval-invariant-exists-expression-owner-missing", "an EXISTS link was run without its expression owner";
    CopiedFilterExpressionMissing => "eval-invariant-copied-filter-expression-missing", "a copied FILTER expression is absent from its owner";
    VmExtraStackValues => "eval-invariant-vm-extra-stack-values", "an expression program left more than its value on the stack";
    CustomInstructionBuiltinFunction => "eval-invariant-custom-instruction-builtin-function", "a custom-call instruction names a built-in function";
    SuspendedCallBuiltinFunction => "eval-invariant-suspended-call-builtin-function", "a suspended function call names a built-in function";
    VmEvidencePoisoned => "eval-invariant-vm-evidence-poisoned", "poisoned original VM evidence records";
    VmStackUnderflow => "eval-invariant-vm-stack-underflow", "an expression program popped an empty stack";
    CandidateAccountingAborted => "eval-invariant-candidate-accounting-aborted", "candidate constraint accounting was aborted; partial counts cannot be published";
    CandidateAccountingPoisoned => "eval-invariant-candidate-accounting-poisoned", "poisoned candidate constraint accounting";
    FoldPartialTypeMismatch => "eval-invariant-fold-partial-type-mismatch", "FOLD combined two partial accumulators built for different composite datatypes; every partial of one fold comes from the same init factory";
    CompletedEvidenceRowsMissing => "eval-invariant-completed-evidence-rows-missing", "a completed evidence query lost its original WHERE rows";
    EvidenceDeclarationsMissing => "eval-invariant-evidence-declarations-missing", "an evidence query lost its original declarations";
    DiagnosticTextMissing => "eval-invariant-diagnostic-text-missing", "native diagnostic publication has no original text";
    DiagnosticStorageMismatch => "eval-invariant-diagnostic-storage-mismatch", "native diagnostic publication retains unmatched construction storage";
    EvidenceInputPoisoned => "eval-invariant-evidence-input-poisoned", "poisoned original evidence input";
    EvidenceAdmissionPoisoned => "eval-invariant-evidence-admission-poisoned", "poisoned native evidence admission";
    ReasonOwnerMissing => "eval-invariant-reason-owner-missing", "ordinary evidence has no reason owner";
    ExistsPlaceholderOutsideCopy => "eval-invariant-exists-placeholder-outside-copy", "a nested EXISTS placeholder was evaluated outside the substituted copy that \
             left it";
    RelationPairSourceMismatch => "eval-invariant-relation-pair-source-mismatch", "geographic relation pair does not belong to its immutable source";
    HeldInListDispatch => "eval-invariant-held-in-list-dispatch", "heldIn is not an rdf:List function";
    ForeignDatasetTerm => "eval-invariant-foreign-dataset-term", "foreign dataset term id";
    ServiceSubstitutionUnresolved => "eval-invariant-service-substitution-unresolved", "substituting an IRI endpoint into a variable-endpoint SERVICE did not \
                     resolve its endpoint";
    ServiceColumnMissing => "eval-invariant-service-column-missing", "SERVICE block column is absent from its union schema";
    TrackedRowControlShared => "eval-invariant-tracked-row-control-shared", "fresh tracked row controls are unexpectedly shared";
    NativeTextMissing => "eval-invariant-native-text-missing", "native text publication has no payload";
    NativeTextStorageMismatch => "eval-invariant-native-text-storage-mismatch", "native text publication retains unmatched working storage";
    LedgerEvidenceDictionaryPoisoned => "eval-invariant-ledger-evidence-dictionary-poisoned", "poisoned original ledger evidence dictionary";
    LedgerReasonDictionaryPoisoned => "eval-invariant-ledger-reason-dictionary-poisoned", "poisoned original ledger reason dictionary";
    LedgerRowForeignQuery => "eval-invariant-ledger-row-foreign-query", "ledger row belongs to another query";
    LedgerAdmissionPoisoned => "eval-invariant-ledger-admission-poisoned", "poisoned native ledger admission";
    AcceptanceEvidencePoisoned => "eval-invariant-acceptance-evidence-poisoned", "poisoned original acceptance evidence dictionary";
    AcceptedPayloadForeignQuery => "eval-invariant-accepted-payload-foreign-query", "accepted payload belongs to another query";
    EvidenceDictionaryPoisoned => "eval-invariant-evidence-dictionary-poisoned", "poisoned query evidence dictionary";
    ReasonDictionaryPoisoned => "eval-invariant-reason-dictionary-poisoned", "poisoned original reason dictionary";
    UnknownClaimReason => "eval-invariant-unknown-claim-reason", "claim names an unknown reason identity";
    ReasonIdentityMismatch => "eval-invariant-reason-identity-mismatch", "source reason identity disagrees with its canonical description";
    ReasonAdmissionPoisoned => "eval-invariant-reason-admission-poisoned", "poisoned original reason admission";
    EvidenceRowsForeignOwners => "eval-invariant-evidence-rows-foreign-owners", "evidence rows belong to different query owners";
    EvidenceRowForeignOwner => "eval-invariant-evidence-row-foreign-owner", "evidence row belongs to another query owner";
    ExistentialSearchForeignOwner => "eval-invariant-existential-search-foreign-owner", "existential search belongs to another query owner";
    ExistentialWitnessForeignOwner => "eval-invariant-existential-witness-foreign-owner", "existential witness belongs to another query owner";

}
