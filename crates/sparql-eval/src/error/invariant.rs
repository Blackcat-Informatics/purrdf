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
    CoordinateEnvironmentChanged => "eval-invariant-coordinate-environment-changed", "one execution changed its certified implementation identity";
    CoordinateIdentityFormatting => "eval-invariant-coordinate-identity-formatting", "an execution coordinate identity could not be formatted";
    CoordinateFocusIdentityMissing => "eval-invariant-coordinate-focus-identity-missing", "an immutable focus graph has no snapshot identity";
    CoordinateObserverPoisoned => "eval-invariant-coordinate-observer-poisoned", "the execution coordinate observer was poisoned";
    QueryClockNotDateTime => "eval-invariant-query-clock-not-datetime", "an admitted query context lost its dateTime clock coordinate";
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
}
