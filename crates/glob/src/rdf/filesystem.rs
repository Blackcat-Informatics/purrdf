// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RDF projections of complete rooted filesystem evaluations.

use super::{
    Account, Admission, Cell, DatasetHandle, Error, ExportError, FIRST, Memory, NIL, PreparedPlan,
    REST, RdfDatasetBuilder, Reason, Resident, StorageError, TYPE, membership_walk, objects,
};
use crate::filesystem::{Expansion, ExpansionError, FileMatch, Source};
use purrdf_core::BlankScope;
use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError};

/// The caller's logical directory identity and explicit filesystem projection predicates.
#[derive(Debug, Clone, Copy)]
pub struct Projection<'a> {
    /// The absolute IRI identifying this evaluation.
    pub evaluation: &'a str,
    /// The absolute IRI identifying the supplied directory authority.
    pub directory: &'a str,
    /// Predicate linking the evaluation to the directory.
    pub directory_predicate: &'a str,
    /// Predicate linking the evaluation to each blank file record.
    pub file_predicate: &'a str,
}

/// A plan boundary or the original typed filesystem expansion refusal.
#[non_exhaustive]
pub enum FileError<E, W: Admission = Resident> {
    /// The RDF plan did not select the literal surface for relative paths.
    Plan(Error),
    /// Traversal failed before producing a complete result.
    Traversal(ExpansionError<E, W>),
}

impl<E: core::fmt::Debug, W: Admission> core::fmt::Debug for FileError<E, W> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Plan(error) => f.debug_tuple("Plan").field(error).finish(),
            Self::Traversal(error) => f.debug_tuple("Traversal").field(error).finish(),
        }
    }
}

impl<E: core::fmt::Display, W: Admission> core::fmt::Display for FileError<E, W> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Plan(error) => error.fmt(f),
            Self::Traversal(error) => error.fmt(f),
        }
    }
}

impl<E: core::error::Error + 'static, W: Admission + 'static> core::error::Error
    for FileError<E, W>
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Plan(error) => Some(error),
            Self::Traversal(error) => Some(error),
        }
    }
}

impl<E: StructuredError, W: Admission> StructuredError for FileError<E, W> {
    fn visit<'e>(&'e self, visitor: &mut dyn ErrorVisitor<'e>) {
        visitor.variant("glob-rdf-file-refusal");
        match self {
            Self::Plan(error) => visitor.value(Some("cause"), ErrorValue::Cause(error)),
            Self::Traversal(error) => visitor.value(Some("cause"), ErrorValue::Cause(error)),
        }
    }
}

/// A complete rooted evaluation and the original plan owning its occurrences.
#[derive(Debug)]
pub struct FileExecution<'v, S: Admission = Resident, W: Admission = Resident> {
    plan: PreparedPlan<'v, S>,
    expansion: Expansion<W>,
}

impl<'v, S: Admission> PreparedPlan<'v, S> {
    /// Execute the original RDF plan against exact root-relative UTF-8 paths.
    ///
    /// # Errors
    /// Refuses incompatible plans and any source, work or storage failure.
    #[cfg_attr(
        target_pointer_width = "64",
        expect(
            clippy::result_large_err,
            reason = "The refusal retains original diagnostic and allocation owners inline"
        )
    )]
    pub fn execute_files<T: Source, W: Admission>(
        self,
        source: &mut T,
        max_work: u64,
        storage: W,
    ) -> Result<FileExecution<'v, S, W>, FileError<T::Error, W>> {
        if self.surface != 0 {
            return Err(FileError::Plan(Error {
                source: self.source,
                plan: self.schema.root,
                input: None,
                surface: Some(self.surface),
                reason: Reason::FileProjection {
                    role: "surface",
                    code: "filesystem paths require the literal surface",
                },
            }));
        }
        let expansion = self
            .compiled
            .expand_files(source, max_work, storage)
            .map_err(FileError::Traversal)?;
        Ok(FileExecution {
            plan: self,
            expansion,
        })
    }
}

impl<S: Admission, W: Admission> FileExecution<'_, S, W> {
    /// The complete native file matches, ordered by the traversal contract.
    pub fn files(&self) -> impl ExactSizeIterator<Item = FileMatch<'_>> {
        self.expansion.files()
    }

    /// The number of charged directory and matching operations.
    pub fn work(&self) -> u64 {
        self.expansion.work()
    }

    /// Import the plan once and attach each original membership triple to its file record.
    ///
    /// # Errors
    /// Refuses invalid or conflicting projection metadata and any native output failure.
    #[cfg_attr(
        target_pointer_width = "64",
        expect(
            clippy::result_large_err,
            reason = "The refusal retains original RDF and allocation owners inline"
        )
    )]
    pub fn export<O: Admission + Send + Sync + 'static>(
        self,
        projection: Projection<'_>,
        storage: O,
    ) -> Result<DatasetHandle, ExportError<O>> {
        let mut account = Account(storage);
        let result = self
            .validate(projection)
            .and_then(|()| self.build(projection, &mut Memory::new(&mut account.0)));
        let (reason, account) = match result {
            Ok(dataset) => match DatasetHandle::try_from_admitted(dataset, account) {
                Ok(handle) => return Ok(handle),
                Err(_) => (Reason::Storage(StorageError::AllocationFailed), None),
            },
            Err(reason) => (reason, Some(account)),
        };
        Err(ExportError {
            error: Error {
                source: self.plan.source,
                plan: self.plan.schema.root,
                input: None,
                surface: Some(self.plan.surface),
                reason,
            },
            _account: account,
        })
    }

    fn validate(&self, projection: Projection<'_>) -> Result<(), Reason> {
        self.plan.validate_evaluation(projection.evaluation)?;
        for (role, value) in [
            ("directory", projection.directory),
            ("directory predicate", projection.directory_predicate),
            ("file predicate", projection.file_predicate),
        ] {
            if purrdf_iri::absolute_verdict(value) != Some(true) {
                return Err(Reason::FileProjection {
                    role,
                    code: "expected an absolute IRI",
                });
            }
        }
        for (role, value) in [
            ("directory predicate", projection.directory_predicate),
            ("file predicate", projection.file_predicate),
        ] {
            if self.plan.vocab.terms.contains(&value) || [TYPE, FIRST, REST, NIL].contains(&value) {
                return Err(Reason::FileProjection {
                    role,
                    code: "IRI collides with a configured plan role",
                });
            }
        }
        if projection.directory == projection.evaluation
            || projection.directory_predicate == projection.file_predicate
        {
            return Err(Reason::FileProjection {
                role: "filesystem projection",
                code: "directory and evaluation identities or output predicates collide",
            });
        }
        let source = &*self.plan.source;
        if let Some(node) = source.term_id_by_iri(projection.evaluation) {
            for field in [
                projection.directory_predicate,
                projection.file_predicate,
                self.plan.vocab.terms[9],
                self.plan.vocab.terms[12],
                TYPE,
            ] {
                if let Some(predicate) = source.term_id_by_iri(field)
                    && objects(source, node, predicate).next().is_some()
                {
                    return Err(Reason::OccupiedFileEvaluation { node, predicate });
                }
            }
        }
        Ok(())
    }

    fn build<O: Admission>(
        &self,
        projection: Projection<'_>,
        memory: &mut Memory<'_, O>,
    ) -> Result<purrdf_core::RdfDataset, Reason> {
        let mut builder = RdfDatasetBuilder::new();
        let map = builder
            .push_dataset_with_mapping_with_memory(&self.plan.source, memory)
            .map_err(Reason::Storage)?;
        let result = (|| {
            let schema = self.plan.schema;
            let terms = self.plan.vocab.terms;
            let evaluation = builder.intern_iri_with_memory(projection.evaluation, memory)?;
            let root = builder.intern_iri_with_memory(projection.directory, memory)?;
            let root_predicate =
                builder.intern_iri_with_memory(projection.directory_predicate, memory)?;
            let file_predicate =
                builder.intern_iri_with_memory(projection.file_predicate, memory)?;
            let input = builder.intern_iri_with_memory(terms[10], memory)?;
            let matched = builder.intern_iri_with_memory(terms[11], memory)?;
            let ty = builder.intern_iri_with_memory(TYPE, memory)?;
            let class = builder.intern_iri_with_memory(terms[8], memory)?;
            let plan = builder.intern_iri_with_memory(terms[9], memory)?;
            let first = builder.intern_iri_with_memory(FIRST, memory)?;
            let work_predicate = builder.intern_iri_with_memory(terms[12], memory)?;
            let work_text = memory.format(&format_args!("{}", self.work()))?;
            let work = builder.intern_literal_parts_with_memory(
                &work_text,
                Some(purrdf_core::datatype::XSD_INTEGER),
                None,
                None,
                memory,
            )?;
            memory.release_string(work_text)?;
            for (predicate, object) in [
                (ty, class),
                (
                    plan,
                    map[schema.root.index()].expect("imported plan identity"),
                ),
                (root_predicate, root),
                (work_predicate, work),
            ] {
                builder.push_quad_with_memory(evaluation, predicate, object, None, memory)?;
            }
            let mut memberships = Vec::new();
            if self.files().len() != 0 {
                memory.reserve(&mut memberships, schema.count)?;
                let current = Cell::new(schema.head);
                for member in membership_walk(&self.plan.source, schema, &current) {
                    let member = member.expect("validated immutable membership");
                    memberships.push((
                        map[current.get().index()].expect("imported membership cell"),
                        map[member.index()].expect("imported pattern identity"),
                    ));
                }
            }
            let result = (|| {
                for (ordinal, file) in self.files().enumerate() {
                    let label = memory.format(&format_args!("file{ordinal}"))?;
                    // Imported source blanks have a fresh nondefault scope.
                    let record =
                        builder.intern_blank_with_memory(&label, BlankScope::DEFAULT, memory)?;
                    memory.release_string(label)?;
                    let path = builder
                        .intern_literal_parts_with_memory(file.path, None, None, None, memory)?;
                    builder.push_quad_with_memory(
                        evaluation,
                        file_predicate,
                        record,
                        None,
                        memory,
                    )?;
                    builder.push_quad_with_memory(record, input, path, None, memory)?;
                    for &index in file.indices {
                        let (cell, member) = memberships[index];
                        let quoted =
                            builder.intern_triple_with_memory(cell, first, member, memory)?;
                        builder.push_quad_with_memory(record, matched, quoted, None, memory)?;
                    }
                }
                Ok(())
            })();
            memory.release_vec_after(memberships, result)
        })();
        memory
            .release_vec_after(map, result)
            .map_err(Reason::Storage)?;
        let dataset = builder.freeze_with_memory(memory).map_err(Reason::Build)?;
        memory
            .add_bytes(DatasetHandle::allocation_layout::<Account<O>>().size())
            .map_err(Reason::Storage)?;
        Ok(dataset)
    }
}
