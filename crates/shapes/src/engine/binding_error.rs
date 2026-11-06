// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original binding and changed-row operands refused by prepared validation.

/// Why a prepared binding cannot admit the requested focus or change scope.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BindingRefusal {
    /// The validator has no mutation snapshot to expand.
    MissingMutationBinding,
    /// Its view omits the RDF 1.2 statement layer named by changes.
    UnprojectedStatements,
    /// The supplied mutation snapshot is not the bound one.
    WrongSnapshot,
    /// A retained focus set addresses another dataset.
    ForeignFocusSet,
    /// A focus id carries another binding's stamp.
    ForeignFocus {
        /// Original caller-supplied native term id.
        id: purrdf_core::TermId,
    },
    /// A stamped focus id does not address the bound term table.
    FocusOutOfRange {
        /// Original caller-supplied native term id.
        id: purrdf_core::TermId,
        /// Actual bound term-table size.
        terms: u64,
    },
    /// The bound view cannot map an original changed row.
    UnmappedChangedRow {
        /// Original mutation snapshot row.
        row: purrdf_core::ir::QuadIds<purrdf_core::ir::DeltaViewId>,
        /// Actual bound term-table size.
        terms: u64,
    },
    /// A retained cross-focus read names no indexed shape.
    UnknownBroadcast {
        /// Original top-level shape position.
        shape: usize,
        /// Number of shapes actually indexed by the binding.
        shapes: usize,
    },
}

impl std::fmt::Display for BindingRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingMutationBinding => f.write_str(
                "affected_focus_node_ids: this validator is not bound to a mutation snapshot, so it \
                 has no change to expand; bind through PreparedShapes::bind_delta_with_shapes_graph"),
            Self::UnprojectedStatements => f.write_str(
                "affected_focus_node_ids: this validator is bound to a mutation snapshot through a \
                 view that does not project the RDF 1.2 statement layer, so it reads the plain \
                 table alone; a row the overlay demotes onto the annotation table would leave that \
                 read surface without appearing in the change set, and the expansion would silently \
                 omit the focus nodes it moves; bind through \
                 PreparedShapes::bind_delta_with_shapes_graph, which projects statements"),
            Self::WrongSnapshot => f.write_str(
                "affected_focus_node_ids: the supplied mutation snapshot is not the one this \
                 validator was bound to, and the ids it would produce would index this \
                 validator's term table while naming the other snapshot's terms"),
            Self::ForeignFocusSet => f.write_str(
                "focus node TermIds were resolved against a different dataset than the one this \
                 validator is bound to; TermIds are dataset-local and an in-range id from another \
                 dataset addresses a different term"),
            Self::ForeignFocus { id } => write!(f,
                "focus node TermId {} was minted against a different dataset binding than the \
                 one this validator is bound to; TermIds are dataset-local, so an in-range id \
                 from another binding resolves to a different term", id.index()),
            Self::FocusOutOfRange { id, terms } => write!(f,
                "focus node TermId {} is outside the prepared dataset's {terms}-term table", id.index()),
            Self::UnmappedChangedRow { terms, .. } => write!(f,
                "internal change-path defect: a changed row of the bound mutation snapshot \
                 names a term the binding's own {terms}-term view does not map"),
            Self::UnknownBroadcast { shape, shapes } => write!(f,
                "internal change-path defect: a cross-focus read names top-level shape {shape} of \
                 a binding indexing {shapes}"),
        }
    }
}

impl std::error::Error for BindingRefusal {}

impl purrdf_hash::error::StructuredError for BindingRefusal {
    purrdf_hash::error_visit! {
        MissingMutationBinding => "shacl-binding-missing-mutation" {},
        UnprojectedStatements => "shacl-binding-unprojected-statements" {},
        WrongSnapshot => "shacl-binding-wrong-snapshot" {},
        ForeignFocusSet => "shacl-binding-foreign-focus-set" {},
        ForeignFocus { id } => "shacl-binding-foreign-focus" { "id" => Unsigned(id.index() as u128) },
        FocusOutOfRange { id, terms } => "shacl-binding-focus-out-of-range" {
            "id" => Unsigned(id.index() as u128), "terms" => Unsigned(u128::from(*terms)),
        },
        UnmappedChangedRow { row, terms } => "shacl-binding-unmapped-changed-row" {
            "subject" => Record(&row.s), "predicate" => Record(&row.p), "object" => Record(&row.o),
            "graph" => Value(row.g.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent,
                |id| purrdf_hash::error::ErrorValue::Record(id))),
            "terms" => Unsigned(u128::from(*terms)),
        },
        UnknownBroadcast { shape, shapes } => "shacl-binding-unknown-broadcast" {
            "shape" => Unsigned(*shape as u128), "shapes" => Unsigned(*shapes as u128),
        },
    }
}
