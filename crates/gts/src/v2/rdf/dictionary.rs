// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::borrow::Cow;

use super::values::Candidate;
use super::{
    FoldError, Line, LiteralDirection, LiteralValue, RdfFold, Term, TermId, ValueName, ValueStore,
};
use crate::v2::fields::copy_bytes;
use crate::v2::{
    DecodedRows, DecodedTerms, LiteralForm, Row, TermDefinition, TermOrder, TermReservation,
    TermText, TypedValue,
};
use crate::{FastMap, FastSet};

mod by_reference;
mod order;
mod written;

pub(in crate::v2) use by_reference::ByReferenceTerms;
pub use by_reference::{ByReferenceForm, ByReferenceTerm};

/// Findings from the first failed condition of each definition in a terms frame.
#[derive(Debug, Default)]
#[non_exhaustive]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Independent findings can coexist across definitions in one frame"
)]
pub struct DefinitionFindings {
    /// One or more reconstructed terms contain invalid UTF-8.
    pub ill_formed_term: bool,
    /// One or more definitions refer to an ID at or after their own.
    pub forward_reference: bool,
    /// One or more definitions refer to a term of a disallowed kind.
    pub position_constraint: bool,
    /// New missing pairs, in order of first use.
    pub missing_values: Vec<ValueName>,
    /// The written terms violate the frame's canonical order declaration.
    pub declared_order_violated: bool,
    pub(in crate::v2) capacity_refused_value: bool,
}

/// A row's resolution under P3 §4.2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowDisposition {
    /// All terms resolve and meet the position constraints.
    Accepted(Line),
    /// At least one ID is reserved but unreadable.
    Held,
    /// At least one ID is not yet reserved.
    ForwardReference,
    /// A resolved term has the wrong kind for its position.
    PositionConstraint,
}

/// Findings to report once each for a row frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct RowFindings {
    /// A row names an undefined ID.
    pub forward_reference: bool,
    /// A row was held because one of its terms is unreadable.
    pub opaque_term: bool,
    /// A row violates a position constraint.
    pub position_constraint: bool,
}

/// The dictionary constraints at the original position of a row frame.
#[derive(Clone, Copy, Debug)]
pub(in crate::v2) struct RowScope {
    next: u128,
    broken_prefix: u128,
    in_doubt: bool,
}

impl RowScope {
    pub(in crate::v2) const EMPTY: Self = Self {
        next: 0,
        broken_prefix: 0,
        in_doubt: false,
    };

    pub(in crate::v2) const fn defines(self, id: u64) -> bool {
        (id as u128) < self.next
    }

    pub(in crate::v2) const fn admits(self, id: u64) -> bool {
        !self.in_doubt && (id as u128) >= self.broken_prefix
    }

    pub(in crate::v2) const fn waits_for_commit(self) -> bool {
        self.broken_prefix != 0 && !self.in_doubt
    }

    pub(in crate::v2) const fn below_commit(self) -> Self {
        Self {
            broken_prefix: 0,
            ..self
        }
    }
}

#[derive(Debug)]
struct Run {
    end: u128,
    kind: u64,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum DefinitionKey {
    Atom(u64, TermId),
    Triple([u64; 3]),
    UnreadableIri(Vec<u8>),
    UnreadableBlank(Vec<u8>),
    UnreadableLiteral(Vec<u8>, ReferenceForm),
    UnreadableValue(LiteralValue),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ReferenceForm {
    Typed(u64),
    Language(usize, Option<LiteralDirection>),
    TypedValue,
}

/// A segment's reserved IDs and their readable definitions.
///
/// A run consumes one interval regardless of its declared term count. The
/// readable IDs name terms in the same file fold throughout the segment.
#[derive(Debug)]
pub struct SegmentDictionary {
    segment: u64,
    next: u128,
    runs: Vec<Run>,
    readable: FastMap<u64, TermId>,
    definitions: FastMap<DefinitionKey, u64>,
    references: FastMap<(ValueName, ReferenceForm), u64>,
    reference_languages: FastMap<String, usize>,
    aliases: FastMap<u64, u64>,
    first_forward_definition: Option<u64>,
    annotation_aliases: Option<(u128, FastMap<u64, u64>)>,
    order: order::History,
    in_doubt: bool,
    broken_prefix: u128,
}

impl SegmentDictionary {
    /// Begin the term scope at the given file-relative segment ordinal.
    #[must_use]
    pub fn new(segment: u64) -> Self {
        Self {
            segment,
            next: 0,
            runs: Vec::new(),
            readable: FastMap::default(),
            definitions: FastMap::default(),
            references: FastMap::default(),
            reference_languages: FastMap::default(),
            aliases: FastMap::default(),
            first_forward_definition: None,
            annotation_aliases: None,
            order: order::History::default(),
            in_doubt: false,
            broken_prefix: 0,
        }
    }

    /// The first unreserved ID. It can equal 2^64.
    #[must_use]
    pub const fn next_id(&self) -> u128 {
        self.next
    }

    /// Mark a recovery region or opaque non-frame item.
    pub const fn mark_doubt(&mut self) {
        self.in_doubt = true;
    }

    /// Apply the dictionary rule for an ordinary broken link at this position.
    ///
    /// Apply it before observing the broken frame's own reservation.
    pub const fn mark_broken_link(&mut self) {
        self.broken_prefix = self.next;
    }

    /// The readable definition of this ID before row-specific doubt rules.
    #[must_use]
    pub fn readable_term(&self, id: u64) -> Option<TermId> {
        self.readable.get(&id).copied()
    }

    pub(in crate::v2) fn forget_term_order(&mut self) {
        self.order.last = None;
    }

    /// Add a reservation and resolve definitions when the frame is folded.
    ///
    /// Supply the definitions from this reservation's admitted payload.
    /// A frame outside the fold supplies only its reservation.
    ///
    /// # Errors
    /// Returns a reservation or definition mismatch or a host resource refusal.
    pub fn observe_terms(
        &mut self,
        fold: &mut RdfFold,
        reservation: &TermReservation<'_>,
        decoded: Option<DecodedTerms<'_>>,
        values: &ValueStore,
    ) -> Result<DefinitionFindings, FoldError> {
        self.observe_definitions(
            fold,
            reservation,
            decoded,
            DefinitionValues::Rebuilt(values),
        )
    }

    pub(in crate::v2) fn observe_referenced_terms(
        &mut self,
        fold: &mut RdfFold,
        reservation: &TermReservation<'_>,
        decoded: DecodedTerms<'_>,
        terms: &mut ByReferenceTerms,
        names_value: impl Fn(ValueName) -> bool,
    ) -> Result<DefinitionFindings, FoldError> {
        self.observe_definitions(
            fold,
            reservation,
            Some(decoded),
            DefinitionValues::Referenced(terms, &names_value),
        )
    }

    fn observe_definitions(
        &mut self,
        fold: &mut RdfFold,
        reservation: &TermReservation<'_>,
        decoded: Option<DecodedTerms<'_>>,
        mut values: DefinitionValues<'_>,
    ) -> Result<DefinitionFindings, FoldError> {
        let start = reservation.start;
        let end = reservation.reserved_end();
        if start != self.next || end < start || end != reservation.end {
            return Err(FoldError::Reservation {
                expected: self.next,
                start,
                end,
            });
        }
        if let Some(decoded) = &decoded {
            if decoded.terms.len() as u128 != end - start
                || !same_run_kinds(reservation, &decoded.terms)
            {
                return Err(FoldError::Definitions);
            }
            self.readable.try_reserve(decoded.terms.len())?;
            self.definitions.try_reserve(decoded.terms.len())?;
        }
        self.runs.try_reserve(
            usize::try_from(reservation.run_count()).map_err(|_| FoldError::Definitions)?,
        )?;
        let runs_before = self.runs.len();
        let languages_before = self.reference_languages.len();
        let forward_before = self.first_forward_definition;
        let terms_before = fold.terms.checkpoint();
        let mut run_end = start;
        for (kind, count) in reservation.runs() {
            run_end += u128::from(count.get());
            self.runs.push(Run { end: run_end, kind });
        }
        let mut findings = DefinitionFindings::default();
        if let Some(decoded) = decoded {
            let canonical = reservation.content.as_ref().is_some_and(|content| {
                content
                    .as_ref()
                    .is_ok_and(|header| header.order == TermOrder::Canonical)
            });
            let result =
                self.check_order(fold, &decoded, canonical)
                    .and_then(|(violated, last)| {
                        findings.declared_order_violated = violated;
                        self.resolve_terms(fold, decoded, &mut values, &mut findings, canonical)?;
                        self.order.last = last;
                        Ok(())
                    });
            if let Err(error) = result {
                self.runs.truncate(runs_before);
                self.order.rollback(start);
                if let DefinitionValues::Referenced(terms, _) = &mut values {
                    terms.rollback(start);
                }
                self.readable.retain(|id, _| u128::from(*id) < start);
                self.definitions.retain(|_, id| u128::from(*id) < start);
                self.references.retain(|_, id| u128::from(*id) < start);
                self.reference_languages
                    .retain(|_, id| *id < languages_before);
                self.aliases.retain(|id, _| u128::from(*id) < start);
                self.first_forward_definition = forward_before;
                fold.terms.rollback(terms_before);
                for value in findings.missing_values {
                    fold.missing_values.remove(&value);
                }
                return Err(error);
            }
        } else {
            self.forget_term_order();
        }
        self.next = end;
        Ok(findings)
    }

    fn resolve_terms(
        &mut self,
        fold: &mut RdfFold,
        decoded: DecodedTerms<'_>,
        values: &mut DefinitionValues<'_>,
        findings: &mut DefinitionFindings,
        canonical: bool,
    ) -> Result<(), FoldError> {
        let empty = ValueStore::default();
        let (values, mut references) = match values {
            DefinitionValues::Rebuilt(values) => (*values, None),
            DefinitionValues::Referenced(terms, names) => (&empty, Some((&mut **terms, *names))),
        };
        let context = Definitions {
            languages: &decoded.languages,
            values,
        };
        let mut literals = FastMap::default();
        for (index, definition) in decoded.terms.into_iter().enumerate() {
            let id =
                u64::try_from(self.next + index as u128).expect("definition fits its admitted run");
            if self.first_forward_definition.is_none() && uses_later_id(&definition, id) {
                self.first_forward_definition = Some(id);
            }
            let kind = definition_kind(&definition);
            let components = match &definition {
                TermDefinition::Triple(ids) => Some(ids.map(|id| self.definition_id(id))),
                _ => None,
            };
            // Repeated annotation rows compare the written name and form even
            // when no declaration supplies bytes for the reference.
            let reference = self.reference_key(&definition, context.languages)?;
            let mut unreadable_atom = None;
            let by_reference = if let Some((terms, names)) = &mut references {
                terms.preserve(self, id, &definition, context.languages, *names, findings)?
            } else {
                false
            };
            let term = if by_reference {
                None
            } else {
                self.resolve_definition(
                    fold,
                    id,
                    definition,
                    &context,
                    findings,
                    &mut unreadable_atom,
                )?
            };
            if let Some(term) = term {
                self.readable.insert(id, term);
                if canonical && !findings.declared_order_violated && matches!(kind, 2 | 5 | 6 | 7) {
                    literals.try_reserve(1)?;
                    if let Some(before) = literals.insert(term, kind) {
                        findings.declared_order_violated = before != kind;
                    }
                }
            }
            let first = if let Some(reference) = reference {
                self.references.try_reserve(1)?;
                *self.references.entry(reference).or_insert(id)
            } else if let Some(key) = unreadable_atom
                .or_else(|| components.map(DefinitionKey::Triple))
                .or_else(|| term.map(|term| DefinitionKey::Atom(kind, term)))
            {
                *self.definitions.entry(key).or_insert(id)
            } else {
                id
            };
            if first != id {
                self.aliases.try_reserve(1)?;
                self.aliases.insert(id, first);
            }
        }
        Ok(())
    }

    fn definition_id(&self, id: u64) -> u64 {
        self.aliases.get(&id).copied().unwrap_or(id)
    }

    fn prior_atom(&self, kind: u64, handle: Option<TermId>) -> Option<DefinitionKey> {
        let key = DefinitionKey::Atom(kind, handle?);
        self.definitions.contains_key(&key).then_some(key)
    }

    fn scoped_definition_id(&self, raw: u64, scope: RowScope) -> u64 {
        if scope.defines(raw) {
            self.definition_id(raw)
        } else {
            raw
        }
    }

    fn reference_key(
        &mut self,
        definition: &TermDefinition<'_>,
        languages: &[&str],
    ) -> Result<Option<(ValueName, ReferenceForm)>, FoldError> {
        let key = match definition {
            TermDefinition::LiteralRef(reference, form) => {
                let form = self.written_form(LiteralFields::read(*form, languages)?)?;
                ((*reference).into(), form)
            }
            TermDefinition::ValueRef(reference) => ((*reference).into(), ReferenceForm::TypedValue),
            _ => return Ok(None),
        };
        Ok(Some(key))
    }

    fn written_form(&mut self, form: LiteralFields<'_>) -> Result<ReferenceForm, FoldError> {
        Ok(match form {
            LiteralFields::Typed(datatype) => ReferenceForm::Typed(self.definition_id(datatype)),
            LiteralFields::Language(text, direction) => {
                let id = if let Some(&id) = self.reference_languages.get(text) {
                    id
                } else {
                    let mut owned = String::new();
                    owned.try_reserve_exact(text.len())?;
                    owned.push_str(text);
                    self.reference_languages.try_reserve(1)?;
                    let id = self.reference_languages.len();
                    self.reference_languages.insert(owned, id);
                    id
                };
                ReferenceForm::Language(id, direction)
            }
        })
    }

    fn resolve_definition(
        &mut self,
        fold: &mut RdfFold,
        id: u64,
        definition: TermDefinition<'_>,
        context: &Definitions<'_>,
        findings: &mut DefinitionFindings,
        unreadable_atom: &mut Option<DefinitionKey>,
    ) -> Result<Option<TermId>, FoldError> {
        let term = match definition {
            TermDefinition::Iri(TermText::Text(text)) => Term::Iri(text),
            TermDefinition::Iri(TermText::Invalid(bytes)) => {
                findings.ill_formed_term = true;
                *unreadable_atom = Some(DefinitionKey::UnreadableIri(copy_bytes(&bytes)?));
                return Ok(None);
            }
            TermDefinition::Blank(label) => Term::Blank {
                segment: self.segment,
                label: Cow::Borrowed(label),
            },
            TermDefinition::Triple(ids) => {
                let required = [Position::Subject, Position::Iri, Position::Any];
                let Some(terms) = self.dependencies(id, ids, required, findings) else {
                    return Ok(None);
                };
                Term::Triple(terms)
            }
            TermDefinition::Literal(text, form) => {
                let fields = LiteralFields::read(form, context.languages)?;
                let TermText::Text(text) = text else {
                    findings.ill_formed_term = true;
                    *unreadable_atom = Some(DefinitionKey::UnreadableLiteral(
                        copy_bytes(text.bytes())?,
                        self.written_form(fields)?,
                    ));
                    return Ok(None);
                };
                let Some(form) = self.resolve_literal_form(id, fields, findings) else {
                    *unreadable_atom = Some(DefinitionKey::UnreadableLiteral(
                        copy_bytes(text.as_bytes())?,
                        self.written_form(fields)?,
                    ));
                    return Ok(None);
                };
                if self.in_doubt {
                    let term = form.term(Cow::Borrowed(&text));
                    *unreadable_atom = Some(
                        if let Some(key) = self.prior_atom(2, fold.terms.find(&term)) {
                            key
                        } else {
                            DefinitionKey::UnreadableLiteral(
                                copy_bytes(text.as_bytes())?,
                                self.written_form(fields)?,
                            )
                        },
                    );
                    return Ok(None);
                }
                form.term(text)
            }
            TermDefinition::LiteralRef(reference, form) => {
                let name = reference.into();
                let (candidate, state_refused) = context.values.get(name);
                let text = match candidate {
                    Candidate::Verified(bytes) => match core::str::from_utf8(bytes) {
                        Ok(text) => Some(text),
                        Err(_) => {
                            if state_refused {
                                // This lexical text comes from outside the
                                // run. Judge the scoped datatype separately.
                                findings.capacity_refused_value |= self
                                    .literal_form(
                                        id,
                                        form,
                                        context.languages,
                                        &mut DefinitionFindings::default(),
                                    )?
                                    .is_some();
                            }
                            findings.ill_formed_term = true;
                            return Ok(None);
                        }
                    },
                    _ => None,
                };
                let Some(form) = self.literal_form(id, form, context.languages, findings)? else {
                    return Ok(None);
                };
                findings.capacity_refused_value |= state_refused;
                let Some(text) = text else {
                    note_missing(fold, name, candidate, findings)?;
                    return Ok(None);
                };
                form.term(Cow::Borrowed(text))
            }
            TermDefinition::Value(value) => {
                if self.in_doubt {
                    *unreadable_atom = Some(
                        if let Some(key) = self.prior_atom(6, fold.terms.find_value(value)?) {
                            key
                        } else {
                            DefinitionKey::UnreadableValue(LiteralValue::copy(value)?)
                        },
                    );
                    return Ok(None);
                }
                return self.intern_value(fold, value);
            }
            TermDefinition::ValueRef(reference) => {
                let name = reference.into();
                let (candidate, state_refused) = context.values.get(name);
                findings.capacity_refused_value |= state_refused;
                let Candidate::Verified(bytes) = candidate else {
                    note_missing(fold, name, candidate, findings)?;
                    return Ok(None);
                };
                return self.intern_value(fold, TypedValue::Octets(bytes));
            }
        };
        if self.in_doubt {
            *unreadable_atom = match &term {
                Term::Iri(text) => Some(
                    if let Some(key) = self.prior_atom(1, fold.terms.find(&term)) {
                        key
                    } else {
                        DefinitionKey::UnreadableIri(copy_bytes(text.as_bytes())?)
                    },
                ),
                Term::Blank { label, .. } => Some(
                    if let Some(key) = self.prior_atom(3, fold.terms.find(&term)) {
                        key
                    } else {
                        DefinitionKey::UnreadableBlank(copy_bytes(label)?)
                    },
                ),
                _ => None,
            };
            Ok(None)
        } else {
            fold.terms.intern(term).map(Some)
        }
    }

    fn intern_value(
        &self,
        fold: &mut RdfFold,
        value: TypedValue<'_>,
    ) -> Result<Option<TermId>, FoldError> {
        if self.in_doubt {
            Ok(None)
        } else {
            fold.terms.intern_value(value).map(Some)
        }
    }

    fn literal_form<'a>(
        &self,
        id: u64,
        form: LiteralForm,
        languages: &'a [&'a str],
        findings: &mut DefinitionFindings,
    ) -> Result<Option<ResolvedForm<'a>>, FoldError> {
        let fields = LiteralFields::read(form, languages)?;
        Ok(self.resolve_literal_form(id, fields, findings))
    }

    fn resolve_literal_form<'a>(
        &self,
        id: u64,
        form: LiteralFields<'a>,
        findings: &mut DefinitionFindings,
    ) -> Option<ResolvedForm<'a>> {
        match form {
            LiteralFields::Typed(datatype) => self
                .dependencies(id, [datatype], [Position::Iri], findings)
                .map(|[datatype]| ResolvedForm::Typed(datatype)),
            LiteralFields::Language(language, direction) => {
                Some(ResolvedForm::Language(language, direction))
            }
        }
    }

    fn dependencies<const N: usize>(
        &self,
        definition_id: u64,
        ids: [u64; N],
        required: [Position; N],
        findings: &mut DefinitionFindings,
    ) -> Option<[TermId; N]> {
        if ids.iter().any(|id| *id >= definition_id) {
            findings.forward_reference = true;
            return None;
        }
        if ids
            .into_iter()
            .zip(required)
            .any(|(id, position)| self.wrong_kind(id, position))
        {
            findings.position_constraint = true;
            return None;
        }
        let terms = ids.map(|id| self.readable_term(id));
        if terms.iter().any(Option::is_none) {
            return None;
        }
        Some(terms.map(|id| id.expect("every dependency was readable")))
    }

    fn wrong_kind(&self, id: u64, position: Position) -> bool {
        let index = self.runs.partition_point(|run| run.end <= u128::from(id));
        self.runs
            .get(index)
            .is_some_and(|run| !position.admits(run.kind))
    }

    /// Resolve one row before applying line union.
    #[must_use]
    pub fn resolve_row(&self, row: Row) -> RowDisposition {
        self.resolve_scoped_row(row, self.row_scope())
    }

    pub(in crate::v2) const fn row_scope(&self) -> RowScope {
        RowScope {
            next: self.next,
            broken_prefix: self.broken_prefix,
            in_doubt: self.in_doubt,
        }
    }

    pub(in crate::v2) fn resolve_scoped_row(&self, row: Row, scope: RowScope) -> RowDisposition {
        if let Err(disposition) = self.admit_row(row_references(row), scope, |id| {
            self.readable_term(id).is_some()
        }) {
            return disposition;
        }
        let resolve = |id| {
            self.readable_term(id)
                .expect("row definitions were admitted")
        };
        RowDisposition::Accepted(match row {
            Row::Quad(ids, graph) => Line::Quad(ids.map(resolve), graph.map(resolve)),
            Row::ReifierBinding(ids, graph) => {
                Line::ReifierBinding(ids.map(resolve), graph.map(resolve))
            }
            Row::Annotation(ids, count, graph) => {
                Line::Annotation(ids.map(resolve), count, graph.map(resolve))
            }
            Row::EmptyGraph(id) => Line::EmptyGraph(resolve(id)),
        })
    }

    pub(in crate::v2) fn admit_referenced_row(
        &self,
        row: Row,
        scope: RowScope,
        terms: &ByReferenceTerms,
    ) -> Result<bool, RowDisposition> {
        let references = row_references(row);
        let by_reference = |id| terms.get(id, scope).is_some();
        self.admit_row(references, scope, |id| {
            self.readable_term(id).is_some() || by_reference(id)
        })?;
        Ok(references
            .into_iter()
            .flatten()
            .any(|(id, _)| by_reference(id)))
    }

    fn admit_row(
        &self,
        references: [Option<(u64, Position)>; 4],
        scope: RowScope,
        readable: impl Fn(u64) -> bool,
    ) -> Result<(), RowDisposition> {
        if references
            .iter()
            .flatten()
            .any(|(id, _)| !scope.defines(*id))
        {
            return Err(RowDisposition::ForwardReference);
        }
        if references
            .iter()
            .flatten()
            .any(|(id, _)| !scope.admits(*id) || !readable(*id))
        {
            return Err(RowDisposition::Held);
        }
        if references
            .into_iter()
            .flatten()
            .any(|(id, position)| self.wrong_kind(id, position))
        {
            return Err(RowDisposition::PositionConstraint);
        }
        Ok(())
    }

    /// Resolve the rows of a folded frame and merge the accepted lines.
    ///
    /// # Errors
    /// Returns a host reservation failure, a repeated annotation or a term
    /// handle missing from the fold.
    pub fn fold_rows(
        &self,
        fold: &mut RdfFold,
        decoded: &DecodedRows,
    ) -> Result<RowFindings, FoldError> {
        let scope = self.row_scope();
        if Self::has_annotations(&decoded.rows) {
            let aliases = self
                .compares_written_annotations(scope)
                .then(|| written::Aliases::new(self, fold, scope)?.representatives())
                .transpose()?;
            self.check_annotation_keys(&decoded.rows, scope, aliases.as_ref())?;
        }
        self.fold_scoped_rows(fold, &decoded.rows, scope)
    }

    pub(in crate::v2) fn fold_scoped_rows(
        &self,
        fold: &mut RdfFold,
        rows: &[Row],
        scope: RowScope,
    ) -> Result<RowFindings, FoldError> {
        let mut findings = RowFindings::default();
        let mut lines = Vec::new();
        for &row in rows {
            // Earlier reservations and readable IDs are immutable. The scope
            // excludes every definition that arrived after this frame.
            match self.resolve_scoped_row(row, scope) {
                RowDisposition::Accepted(line) => {
                    for (id, _) in row_references(row).into_iter().flatten() {
                        let term = self
                            .readable_term(id)
                            .expect("only admitted rows are merged");
                        if fold.term(term).is_none() {
                            return Err(FoldError::TermHandle(term));
                        }
                    }
                    lines.try_reserve(1)?;
                    lines.push(line);
                }
                RowDisposition::Held => findings.opaque_term = true,
                RowDisposition::ForwardReference => findings.forward_reference = true,
                RowDisposition::PositionConstraint => findings.position_constraint = true,
            }
        }
        fold.merge_lines(&lines, findings.opaque_term)?;
        Ok(findings)
    }

    pub(in crate::v2) fn check_annotations(
        &mut self,
        fold: &RdfFold,
        rows: &[Row],
    ) -> Result<(), FoldError> {
        self.check_scoped_annotations(fold, rows, self.row_scope())
    }

    pub(in crate::v2) fn check_scoped_annotations(
        &mut self,
        fold: &RdfFold,
        rows: &[Row],
        scope: RowScope,
    ) -> Result<(), FoldError> {
        if !Self::has_annotations(rows) {
            return Ok(());
        }
        let compare_written = self.compares_written_annotations(scope);
        if compare_written
            && self
                .annotation_aliases
                .as_ref()
                .is_none_or(|(end, _)| *end != scope.next)
        {
            let aliases = written::Aliases::new(self, fold, scope)?.representatives()?;
            self.annotation_aliases = Some((scope.next, aliases));
        }
        let aliases = self
            .annotation_aliases
            .as_ref()
            .filter(|_| compare_written)
            .map(|(_, aliases)| aliases);
        self.check_annotation_keys(rows, scope, aliases)
    }

    fn has_annotations(rows: &[Row]) -> bool {
        rows.len() >= 2 && matches!(rows.first(), Some(Row::Annotation(..)))
    }

    fn compares_written_annotations(&self, scope: RowScope) -> bool {
        scope.in_doubt
            || self
                .first_forward_definition
                .is_some_and(|id| scope.defines(id))
    }

    fn check_annotation_keys(
        &self,
        rows: &[Row],
        scope: RowScope,
        aliases: Option<&FastMap<u64, u64>>,
    ) -> Result<(), FoldError> {
        let mut keys = FastSet::default();
        keys.try_reserve(rows.len())?;
        let id = |raw| {
            let base = self.scoped_definition_id(raw, scope);
            aliases
                .and_then(|aliases| aliases.get(&base).copied())
                .unwrap_or(base)
        };
        for &row in rows {
            if let Row::Annotation(terms, _, graph) = row
                && !keys.insert((terms.map(id), graph.map(id)))
            {
                return Err(FoldError::RepeatedAnnotation);
            }
        }
        Ok(())
    }
}

fn uses_later_id(definition: &TermDefinition<'_>, id: u64) -> bool {
    match definition {
        TermDefinition::Triple(ids) => ids.iter().any(|&component| component >= id),
        TermDefinition::Literal(_, LiteralForm::Typed(datatype))
        | TermDefinition::LiteralRef(_, LiteralForm::Typed(datatype)) => *datatype >= id,
        _ => false,
    }
}

enum DefinitionValues<'a> {
    Rebuilt(&'a ValueStore),
    Referenced(&'a mut ByReferenceTerms, &'a dyn Fn(ValueName) -> bool),
}

struct Definitions<'a> {
    languages: &'a [&'a str],
    values: &'a ValueStore,
}

#[derive(Clone, Copy)]
enum LiteralFields<'a> {
    Typed(u64),
    Language(&'a str, Option<LiteralDirection>),
}

impl<'a> LiteralFields<'a> {
    fn read(form: LiteralForm, languages: &'a [&'a str]) -> Result<Self, FoldError> {
        let (index, direction) = match form {
            LiteralForm::Typed(datatype) => return Ok(Self::Typed(datatype)),
            LiteralForm::Language(index) => (index, None),
            LiteralForm::LeftToRight(index) => (index, Some(LiteralDirection::LeftToRight)),
            LiteralForm::RightToLeft(index) => (index, Some(LiteralDirection::RightToLeft)),
        };
        let language = usize::try_from(index)
            .ok()
            .and_then(|index| languages.get(index))
            .ok_or(FoldError::Definitions)?;
        Ok(Self::Language(language, direction))
    }
}

enum ResolvedForm<'a> {
    Typed(TermId),
    Language(&'a str, Option<LiteralDirection>),
}

impl<'a> ResolvedForm<'a> {
    fn term(self, lexical: Cow<'a, str>) -> Term<'a> {
        match self {
            Self::Typed(datatype) => Term::Typed { lexical, datatype },
            Self::Language(language, direction) => Term::Language {
                lexical,
                language: Cow::Borrowed(language),
                direction,
            },
        }
    }
}

fn note_missing(
    fold: &mut RdfFold,
    name: ValueName,
    candidate: Candidate<'_>,
    findings: &mut DefinitionFindings,
) -> Result<(), FoldError> {
    if matches!(candidate, Candidate::Missing) && !fold.missing_values.contains(&name) {
        fold.missing_values.try_reserve(1)?;
        findings.missing_values.try_reserve(1)?;
        fold.missing_values.insert(name);
        findings.missing_values.push(name);
    }
    Ok(())
}

fn same_run_kinds(reservation: &TermReservation<'_>, definitions: &[TermDefinition<'_>]) -> bool {
    let mut terms = definitions.iter();
    for (kind, count) in reservation.runs() {
        for _ in 0..count.get() {
            if terms
                .next()
                .is_none_or(|term| definition_kind(term) != kind)
            {
                return false;
            }
        }
    }
    terms.next().is_none()
}

const fn definition_kind(term: &TermDefinition<'_>) -> u64 {
    match term {
        TermDefinition::Iri(_) => 1,
        TermDefinition::Literal(_, _) => 2,
        TermDefinition::Blank(_) => 3,
        TermDefinition::Triple(_) => 4,
        TermDefinition::LiteralRef(_, _) => 5,
        TermDefinition::Value(_) => 6,
        TermDefinition::ValueRef(_) => 7,
    }
}

#[derive(Clone, Copy)]
pub(super) enum Position {
    Any,
    Iri,
    Subject,
    Triple,
}

impl Position {
    pub(super) const fn admits(self, kind: u64) -> bool {
        // A newer storage form has no known position class. Readability
        // decides a dependency whose kind this reader cannot interpret.
        kind > 7
            || match self {
                Self::Any => true,
                Self::Iri => kind == 1,
                Self::Subject => kind == 1 || kind == 3,
                Self::Triple => kind == 4,
            }
    }
}

fn row_references(row: Row) -> [Option<(u64, Position)>; 4] {
    let subject = |id| Some((id, Position::Subject));
    let predicate = |id| Some((id, Position::Iri));
    let object = |id| Some((id, Position::Any));
    let graph = |id: Option<u64>| id.map(|id| (id, Position::Subject));
    match row {
        Row::Quad([s, p, o], g) | Row::Annotation([s, p, o], _, g) => {
            [subject(s), predicate(p), object(o), graph(g)]
        }
        Row::ReifierBinding([r, t], g) => [subject(r), Some((t, Position::Triple)), graph(g), None],
        Row::EmptyGraph(g) => [subject(g), None, None, None],
    }
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU64;
    use std::borrow::Cow;

    use crate::v2::fields::Fields;
    use crate::v2::{
        DecodedTerms, FoldError, LiteralForm, RdfFold, Row, Term, TermDefinition, TermText,
        TypedValue, ValueStore,
    };

    use super::SegmentDictionary;

    #[test]
    fn annotation_repetition_compares_unreadable_iri_bytes() {
        let content = [0x84, 0, 0x81, 0x82, 1, 4, 1, 1];
        let reservation =
            crate::v2::terms::reserve_verified(Fields::new(&content), 0, NonZeroU64::MIN).unwrap();
        let annotations = [
            Row::Annotation([0, 1, 2], NonZeroU64::MIN, None),
            Row::Annotation([0, 1, 3], NonZeroU64::new(2).unwrap(), None),
        ];
        for (other, repeated) in [(&[0xff][..], true), (&[0xfe][..], false)] {
            let terms = DecodedTerms {
                terms: vec![
                    TermDefinition::Iri(TermText::Text(Cow::Borrowed("https://example.org/r"))),
                    TermDefinition::Iri(TermText::Text(Cow::Borrowed("https://example.org/p"))),
                    TermDefinition::Iri(TermText::Invalid(Cow::Borrowed(&[0xff]))),
                    TermDefinition::Iri(TermText::Invalid(Cow::Borrowed(other))),
                ],
                languages: Vec::new(),
            };
            let mut dictionary = SegmentDictionary::new(0);
            let mut fold = RdfFold::default();
            let findings = dictionary
                .observe_terms(&mut fold, &reservation, Some(terms), &ValueStore::default())
                .unwrap();
            assert!(findings.ill_formed_term);
            let duplicates = dictionary.check_annotations(&fold, &annotations);
            if repeated {
                assert!(matches!(duplicates, Err(FoldError::RepeatedAnnotation)));
            } else {
                duplicates.unwrap();
            }
        }
    }

    #[test]
    fn annotation_repetition_compares_written_atoms_during_dictionary_doubt() {
        let record = |start, count| {
            [
                0x84, start, 0x84, 0x82, 1, count, 0x82, 3, count, 0x82, 2, count, 0x82, 6, count,
                1, 1,
            ]
        };
        let iri = |text| TermDefinition::Iri(TermText::Text(Cow::Borrowed(text)));
        let literal = |text| {
            TermDefinition::Literal(
                TermText::Text(Cow::Borrowed(text)),
                LiteralForm::Language(0),
            )
        };
        let mut dictionary = SegmentDictionary::new(0);
        let mut fold = RdfFold::default();
        let first = record(0, 1);
        let later = record(4, 3);
        let values = [
            (
                &first[..],
                vec![
                    iri("https://example.org/a"),
                    TermDefinition::Blank(b"a"),
                    literal("a"),
                    TermDefinition::Value(TypedValue::Octets(b"a")),
                ],
            ),
            (
                &later[..],
                vec![
                    iri("https://example.org/a"),
                    iri("https://example.org/b"),
                    iri("https://example.org/b"),
                    TermDefinition::Blank(b"a"),
                    TermDefinition::Blank(b"b"),
                    TermDefinition::Blank(b"b"),
                    literal("a"),
                    literal("b"),
                    literal("b"),
                    TermDefinition::Value(TypedValue::Octets(b"a")),
                    TermDefinition::Value(TypedValue::Octets(b"b")),
                    TermDefinition::Value(TypedValue::Octets(b"b")),
                ],
            ),
        ];
        let mut arena_size = 0;
        for (index, (content, terms)) in values.into_iter().enumerate() {
            if index == 1 {
                arena_size = fold.terms().len();
                dictionary.mark_doubt();
            }
            let reservation = crate::v2::terms::reserve_verified(
                Fields::new(content),
                dictionary.next_id(),
                NonZeroU64::MIN,
            )
            .unwrap();
            dictionary
                .observe_terms(
                    &mut fold,
                    &reservation,
                    Some(DecodedTerms {
                        terms,
                        languages: vec!["en"],
                    }),
                    &ValueStore::default(),
                )
                .unwrap();
        }
        assert_eq!(fold.terms().len(), arena_size);
        let row = |object| Row::Annotation([0, 0, object], NonZeroU64::MIN, None);
        for (prior, equal) in (0..4).zip((4..16).step_by(3)) {
            assert!(dictionary.readable_term(equal).is_none());
            for (left, right) in [(prior, equal), (equal + 1, equal + 2)] {
                assert!(matches!(
                    dictionary.check_annotations(&fold, &[row(left), row(right)]),
                    Err(FoldError::RepeatedAnnotation)
                ));
            }
            dictionary
                .check_annotations(&fold, &[row(prior), row(equal + 1)])
                .unwrap();
        }
    }

    #[test]
    fn written_doubt_aliases_survive_external_fold_interning() {
        let mut dictionary = SegmentDictionary::new(0);
        let mut fold = RdfFold::default();
        dictionary.mark_doubt();
        let first = [0x84, 0, 0x81, 0x82, 1, 1, 1, 1];
        let later = [0x84, 1, 0x81, 0x82, 1, 2, 1, 1];
        let spelling = "https://example.org/a";
        let iri = |text| TermDefinition::Iri(TermText::Text(Cow::Borrowed(text)));
        for (index, (content, terms)) in [
            (&first[..], vec![iri(spelling)]),
            (
                &later[..],
                vec![iri(spelling), iri("https://example.org/b")],
            ),
        ]
        .into_iter()
        .enumerate()
        {
            if index == 1 {
                fold.intern_term(Term::Iri(Cow::Borrowed(spelling)))
                    .unwrap();
            }
            let reservation = crate::v2::terms::reserve_verified(
                Fields::new(content),
                dictionary.next_id(),
                NonZeroU64::MIN,
            )
            .unwrap();
            dictionary
                .observe_terms(
                    &mut fold,
                    &reservation,
                    Some(DecodedTerms {
                        terms,
                        languages: Vec::new(),
                    }),
                    &ValueStore::default(),
                )
                .unwrap();
        }
        let row = |object| Row::Annotation([0, 0, object], NonZeroU64::MIN, None);
        assert!(matches!(
            dictionary.check_annotations(&fold, &[row(0), row(1)]),
            Err(FoldError::RepeatedAnnotation)
        ));
        dictionary
            .check_annotations(&fold, &[row(0), row(2)])
            .unwrap();
        assert!(dictionary.readable_term(1).is_none());
    }

    #[test]
    fn annotation_repetition_uses_later_written_aliases_only_once_they_are_defined() {
        let first = [
            0x84, 0, 0x84, 0x82, 1, 2, 0x82, 4, 1, 0x82, 1, 1, 0x82, 4, 3, 1, 1,
        ];
        let later = [0x84, 7, 0x81, 0x82, 1, 1, 1, 1];
        let annotations = [
            Row::Annotation([0, 1, 5], NonZeroU64::MIN, None),
            Row::Annotation([0, 1, 6], NonZeroU64::MIN, None),
        ];
        let iri = |text| TermDefinition::Iri(TermText::Text(Cow::Borrowed(text)));
        let mut fold = RdfFold::default();
        let mut dictionary = SegmentDictionary::new(0);
        let reservation =
            crate::v2::terms::reserve_verified(Fields::new(&first), 0, NonZeroU64::MIN).unwrap();
        let findings = dictionary
            .observe_terms(
                &mut fold,
                &reservation,
                Some(DecodedTerms {
                    terms: vec![
                        iri("https://example.org/a"),
                        iri("https://example.org/p"),
                        TermDefinition::Triple([7, 0, 1]),
                        iri("https://example.org/s"),
                        TermDefinition::Triple([3, 0, 1]),
                        TermDefinition::Triple([0, 1, 2]),
                        TermDefinition::Triple([0, 1, 4]),
                    ],
                    languages: Vec::new(),
                }),
                &ValueStore::default(),
            )
            .unwrap();
        assert!(findings.forward_reference);
        let before = dictionary.row_scope();
        dictionary.check_annotations(&fold, &annotations).unwrap();

        let reservation =
            crate::v2::terms::reserve_verified(Fields::new(&later), 7, NonZeroU64::MIN).unwrap();
        dictionary
            .observe_terms(
                &mut fold,
                &reservation,
                Some(DecodedTerms {
                    terms: vec![iri("https://example.org/s")],
                    languages: Vec::new(),
                }),
                &ValueStore::default(),
            )
            .unwrap();
        dictionary
            .check_scoped_annotations(&fold, &annotations, before)
            .unwrap();
        assert!(matches!(
            dictionary.check_annotations(&fold, &annotations),
            Err(FoldError::RepeatedAnnotation)
        ));
    }
}
