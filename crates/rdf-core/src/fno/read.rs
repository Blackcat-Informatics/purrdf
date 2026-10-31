// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Strict native FnO row accounting under original construction controls.

use super::construction::{
    Context, FnoConstructionFailure, OrdinalKey, Result as ConstructionResult,
};
use super::read_error::{
    FnoFieldKind as Field, FnoReadError, FnoReadFailure, FnoReadSourceError,
    FnoStructureKind as Shape,
};
use super::{
    DCTERMS_FORMAT, DCTERMS_IS_PART_OF, FNO, FNOM, FnBlank, FnFunction, FnImpl, FnMapping,
    FnOutput, FnParam, FnParamMapping, FnReturnMapping, FnoCatalog, FnoCatalogInput, OWL_ONTOLOGY,
    RDF_TYPE, RDFS_COMMENT, RDFS_LABEL, RDFS_SEE_ALSO, SKOS_DEFINITION, TermValue,
};
use crate::{DatasetView, QuadIds, QuadValues, SoleObject, TermLookupError, ViewTermId};
use purrdf_iri::vocab::rdf::{FIRST, NIL, REST};
use purrdf_lex::allocation::{Admission, OrderedTable, Resident, StorageError};
use purrdf_lex::control::{ControlledError, Progress, Unmetered};

enum ReadFailure<R, E> {
    Construction(super::construction::Failure<FnoReadError, E>),
    Source(FnoReadSourceError<R>),
}
type ReadResult<T, R, E> = Result<T, ReadFailure<R, E>>;
impl<R, E> From<super::construction::Failure<FnoReadError, E>> for ReadFailure<R, E> {
    fn from(cause: super::construction::Failure<FnoReadError, E>) -> Self {
        Self::Construction(cause)
    }
}
fn domain<T, R, E>(result: ConstructionResult<T, FnoReadError, E>) -> ReadResult<T, R, E> {
    result.map_err(Into::into)
}
type Objects<'a> = OrderedTable<&'a TermValue, Vec<usize>>;
type Properties<'a> = OrderedTable<&'a str, Objects<'a>>;
type Index<'a> = OrderedTable<&'a TermValue, Properties<'a>>;

impl FnoCatalog {
    /// Read a complete default-graph FnO document for the explicit ontology IRI.
    ///
    /// Native blank identities, literal facets, ordered lists and complete row
    /// accounting share the controlled reader. Named graphs, reifiers, annotations
    /// and unrepresented facts are refused. Returned native errors retain their
    /// original immutable operands; physical refusal needs no error allocation.
    ///
    /// # Errors
    /// Returns provider failures inline without a reporting allocation, physical
    /// refusal, or immutable admitted semantic operands. A failed or truncated
    /// source never publishes a partial catalog.
    pub fn from_dataset<D: DatasetView>(
        view: &D,
        ontology_iri: &str,
    ) -> Result<Self, FnoReadFailure<D::ReadError>> {
        match Self::from_dataset_with_control(view, ontology_iri, Resident, Unmetered) {
            Ok(catalog) => Ok(catalog),
            Err(ControlledError::Native(cause)) => Err(cause),
            Err(ControlledError::Control(never)) => match never {},
        }
    }

    /// Read the same strict catalog while retaining an original owned account.
    ///
    /// Source rows and terms, indexes, list edges, reconstructed fields, validation
    /// and canonicalization use the same native engine as the resident entry point.
    /// Dense local list edges retain the source's identifier distinctions and admit
    /// each constant-time lookup, including native cycle-entry recovery.
    /// `storage` starts with zero live bytes and carries no other live payload;
    /// the immutable catalog or error retains it until its last owner is dropped.
    ///
    /// # Errors
    /// Keeps the original progress error in Control and physical refusal in
    /// Storage. Provider failures move inline without an allocation or later
    /// callback poll; owned semantic operands retain the shared original account.
    /// No partial catalog or truncated list is published on refusal.
    ///
    /// ```
    /// use purrdf_core::{FnoCatalog, RdfDatasetBuilder};
    /// use purrdf_lex::{allocation::Resident, control::ControlledError};
    /// let source = RdfDatasetBuilder::new().freeze().unwrap();
    /// let refused = FnoCatalog::from_dataset_with_control(
    ///     &source, "urn:ontology", Resident, |_| Err("stopped"));
    /// assert!(matches!(refused, Err(ControlledError::Control("stopped"))));
    /// ```
    pub fn from_dataset_with_control<D: DatasetView, S, C>(
        view: &D,
        ontology_iri: &str,
        storage: S,
        progress: C,
    ) -> Result<Self, ControlledError<FnoReadFailure<D::ReadError>, C::Error>>
    where
        S: Admission + Send + Sync + 'static,
        C: Progress,
    {
        let mut context = Context::new(storage, progress);
        let result: ReadResult<_, D::ReadError, C::Error> = (|| {
            if let Some(cause) = view.read_error() {
                return Err(ReadFailure::Source(FnoReadSourceError::Source(cause)));
            }
            context.check(0)?;
            let result = read(&mut context, view, ontology_iri);
            if matches!(
                &result,
                Err(ReadFailure::Source(_))
                    | Err(ReadFailure::Construction(ControlledError::Control(_)))
                    | Err(ReadFailure::Construction(ControlledError::Native(
                        FnoConstructionFailure::Storage(_)
                    )))
            ) {
                return result;
            }
            if let Some(cause) = view.read_error() {
                drop(result);
                return Err(ReadFailure::Source(FnoReadSourceError::Source(cause)));
            }
            let input = result?;
            context.check(0)?;
            Ok(input)
        })();
        match result {
            Ok(input) => {
                let catalog = context
                    .publish(input)
                    .map(|data| Self { data })
                    .map_err(super::construction::with_source)?;
                if let Some(cause) = view.read_error() {
                    drop(catalog);
                    return Err(ControlledError::Native(FnoConstructionFailure::Source(
                        FnoReadSourceError::Source(cause),
                    )));
                }
                Ok(catalog)
            }
            Err(ReadFailure::Source(cause)) => Err(ControlledError::Native(
                FnoConstructionFailure::Source(cause),
            )),
            Err(ReadFailure::Construction(ControlledError::Native(
                FnoConstructionFailure::Semantic(cause),
            ))) => {
                let cause = context
                    .publish_refusal(cause)
                    .map_err(super::construction::with_source)?;
                Err(ControlledError::Native(FnoConstructionFailure::Semantic(
                    cause,
                )))
            }
            Err(ReadFailure::Construction(ControlledError::Control(cause))) => {
                Err(ControlledError::Control(cause))
            }
            Err(ReadFailure::Construction(ControlledError::Native(
                FnoConstructionFailure::Storage(cause),
            ))) => Err(ControlledError::Native(FnoConstructionFailure::Storage(
                cause,
            ))),
            Err(ReadFailure::Construction(ControlledError::Native(
                FnoConstructionFailure::Source(never),
            ))) => match never {},
        }
    }
}

fn collect<T, D: DatasetView, S: Admission, C: Progress>(
    context: &mut Context<S, C>,
    view: &D,
    mut input: impl Iterator<Item = T>,
) -> ReadResult<Vec<T>, D::ReadError, C::Error> {
    let mut rows = Vec::new();
    loop {
        if let Some(cause) = view.read_error() {
            return Err(ReadFailure::Source(FnoReadSourceError::Source(cause)));
        }
        context.check(1)?;
        let row = input.next();
        if let Some(cause) = view.read_error() {
            return Err(ReadFailure::Source(FnoReadSourceError::Source(cause)));
        }
        let Some(row) = row else { break };
        context.push(&mut rows, row)?;
    }
    drop(input);
    if let Some(cause) = view.read_error() {
        return Err(ReadFailure::Source(FnoReadSourceError::Source(cause)));
    }
    Ok(rows)
}

fn import<D: DatasetView, S: Admission, C: Progress>(
    context: &mut Context<S, C>,
    view: &D,
    id: D::Id,
    ids: &mut OrderedTable<OrdinalKey<D::Id>, usize>,
    terms: &mut Vec<TermValue>,
) -> ReadResult<usize, D::ReadError, C::Error> {
    let key = OrdinalKey(id);
    if let Some(at) = context.lookup(ids, &key)? {
        return Ok(*at);
    }
    enum Refusal<R> {
        Term(TermLookupError<R>),
        Storage(StorageError),
    }
    impl<R> From<StorageError> for Refusal<R> {
        fn from(cause: StorageError) -> Self {
            Self::Storage(cause)
        }
    }
    context.check(1)?;
    let value = context.native(|memory| {
        crate::dataset_view::term_value_with_memory_inner(
            view,
            id,
            memory,
            Refusal::Term,
            |cause, _| Refusal::Storage(cause),
            true,
        )
    });
    let value = match value {
        Ok(value) => {
            if let Some(cause) = view.read_error() {
                drop(value);
                return Err(ReadFailure::Source(FnoReadSourceError::Term(
                    TermLookupError::Read(cause),
                )));
            }
            value
        }
        Err(ControlledError::Native(Refusal::Term(cause))) => {
            return Err(ReadFailure::Source(FnoReadSourceError::Term(cause)));
        }
        Err(ControlledError::Native(Refusal::Storage(cause))) => {
            return Err(ReadFailure::Construction(ControlledError::Native(
                FnoConstructionFailure::Storage(cause),
            )));
        }
        Err(ControlledError::Control(cause)) => {
            return Err(ReadFailure::Construction(ControlledError::Control(cause)));
        }
    };
    let at = terms.len();
    context.push(terms, value)?;
    context.insert(ids, key, at)?;
    Ok(at)
}

fn translate<I: ViewTermId, R, S: Admission, C: Progress>(
    context: &mut Context<S, C>,
    mut source: Vec<QuadIds<I>>,
    ids: &OrderedTable<OrdinalKey<I>, usize>,
) -> ReadResult<Vec<QuadIds<usize>>, R, C::Error> {
    let mut rows = Vec::new();
    for row in source.drain(..) {
        let mut at = |id| {
            context
                .lookup(ids, &OrdinalKey(id))
                .map(|at| *at.expect("the source term was imported"))
        };
        let row = QuadIds {
            s: at(row.s)?,
            p: at(row.p)?,
            o: at(row.o)?,
            g: row.g.map(&mut at).transpose()?,
        };
        context.push(&mut rows, row)?;
    }
    context.memory(|memory| memory.release_vec(source))?;
    Ok(rows)
}

fn read<D: DatasetView, S: Admission, C: Progress>(
    context: &mut Context<S, C>,
    view: &D,
    ontology: &str,
) -> ReadResult<FnoCatalogInput, D::ReadError, C::Error> {
    let baseline = context.live();
    let result = read_snapshot(context, view, ontology);
    let Ok(input) = &result else { return result };
    let retained = context.measure(input)?;
    let retained = baseline
        .checked_add(retained)
        .ok_or(ControlledError::Native(FnoConstructionFailure::Storage(
            StorageError::SizeOverflow,
        )))?;
    context.shrink(retained)?;
    result
}

fn read_snapshot<D: DatasetView, S: Admission, C: Progress>(
    context: &mut Context<S, C>,
    view: &D,
    ontology: &str,
) -> ReadResult<FnoCatalogInput, D::ReadError, C::Error> {
    let rows = collect(context, view, view.quads())?;
    let reifiers = collect(context, view, view.reifier_quads())?;
    let annotations = collect(context, view, view.annotation_quads())?;
    let mut graphs = if view.supports_named_graphs() {
        collect(context, view, view.named_graphs())?
    } else {
        Vec::new()
    };
    if let Some(cause) = view.read_error() {
        return Err(ReadFailure::Source(FnoReadSourceError::Source(cause)));
    }
    let mut ids = OrderedTable::default();
    let mut terms = Vec::new();
    for row in rows.iter().chain(&reifiers).chain(&annotations) {
        for id in [Some(row.s), Some(row.p), Some(row.o), row.g]
            .into_iter()
            .flatten()
        {
            import(context, view, id, &mut ids, &mut terms)?;
        }
    }
    for &id in &graphs {
        import(context, view, id, &mut ids, &mut terms)?;
    }
    let rows = translate(context, rows, &ids)?;
    let reifiers = translate(context, reifiers, &ids)?;
    let annotations = translate(context, annotations, &ids)?;
    let mut graph_indices = Vec::new();
    for graph in graphs.drain(..) {
        let at = *context
            .lookup(&ids, &OrdinalKey(graph))?
            .expect("the graph term was imported");
        context.push(&mut graph_indices, at)?;
    }
    context.memory(|memory| memory.release_vec(graphs))?;
    context.release(ids)?;
    let mut claimed = Vec::new();
    context.check(u64::try_from(rows.len()).map_err(|_| {
        ControlledError::Native(FnoConstructionFailure::Storage(StorageError::SizeOverflow))
    })?)?;
    context.memory(|memory| memory.reserve(&mut claimed, rows.len()))?;
    claimed.resize(rows.len(), false);
    let mut reader = Reader {
        context,
        rows: &rows,
        terms: &terms,
        index: OrderedTable::default(),
        claimed,
        cells: OrderedTable::default(),
        edges: Vec::new(),
        nil: None,
        marker: core::marker::PhantomData,
    };
    if let Some(row) = reifiers.first() {
        return reader.statement(*row, Shape::Reifier);
    }
    if let Some(row) = annotations.first() {
        return reader.statement(*row, Shape::Annotation);
    }
    for (at, row) in rows.iter().enumerate() {
        if row.g.is_some() {
            return reader.statement(*row, Shape::NamedGraph);
        }
        let TermValue::Iri(predicate) = &terms[row.p] else {
            return reader.statement(*row, Shape::Unclaimed);
        };
        let subject = &terms[row.s];
        let object = &terms[row.o];
        let predicate = predicate.as_str();
        let mut props = reader
            .context
            .insert(&mut reader.index, subject, OrderedTable::default())?
            .unwrap_or_default();
        let mut objects = reader
            .context
            .insert(&mut props, predicate, OrderedTable::default())?
            .unwrap_or_default();
        let mut occurrences = reader
            .context
            .insert(&mut objects, object, Vec::new())?
            .unwrap_or_default();
        reader.context.push(&mut occurrences, at)?;
        reader.context.insert(&mut objects, object, occurrences)?;
        reader.context.insert(&mut props, predicate, objects)?;
        reader.context.insert(&mut reader.index, subject, props)?;
    }
    if let Some(&graph) = graph_indices.first() {
        let graph = reader.context.term(&terms[graph])?;
        return domain(reader.context.refuse(FnoReadError::Graph { graph }));
    }
    for (at, value) in terms.iter().enumerate() {
        reader.context.check(1)?;
        if let TermValue::Iri(iri) = value {
            if reader.context.compare(iri.as_str(), NIL)?.is_eq() {
                reader.nil = Some(at);
                break;
            }
        }
    }
    reader
        .context
        .check(u64::try_from(terms.len()).map_err(|_| {
            ControlledError::Native(FnoConstructionFailure::Storage(StorageError::SizeOverflow))
        })?)?;
    reader
        .context
        .memory(|memory| memory.reserve(&mut reader.edges, terms.len()))?;
    reader.edges.resize(terms.len(), [SoleObject::None; 2]);
    for row in &rows {
        let TermValue::Iri(predicate) = &terms[row.p] else {
            unreachable!("predicates were checked")
        };
        let slot = if reader.context.compare(predicate.as_str(), FIRST)?.is_eq() {
            Some(0)
        } else if reader.context.compare(predicate.as_str(), REST)?.is_eq() {
            Some(1)
        } else {
            None
        };
        if let Some(slot) = slot {
            reader.context.check(1)?;
            reader.edges[row.s][slot] = reader.edges[row.s][slot].and(row.o);
        }
    }
    reader.catalog(ontology)
}

struct Reader<'a, 'm, R, S: Admission, C> {
    context: &'m mut Context<S, C>,
    rows: &'a [QuadIds<usize>],
    terms: &'a [TermValue],
    index: Index<'a>,
    claimed: Vec<bool>,
    cells: OrderedTable<&'a TermValue, ()>,
    edges: Vec<[SoleObject<usize>; 2]>,
    nil: Option<usize>,
    marker: core::marker::PhantomData<R>,
}

impl<'a, R, S: Admission, C: Progress> Reader<'a, '_, R, S, C> {
    fn statement<T>(&mut self, row: QuadIds<usize>, reason: Shape) -> ReadResult<T, R, C::Error> {
        let row = QuadValues {
            s: self.context.term(&self.terms[row.s])?,
            p: self.context.term(&self.terms[row.p])?,
            o: self.context.term(&self.terms[row.o])?,
            g: row
                .g
                .map(|id| self.context.term(&self.terms[id]))
                .transpose()?,
        };
        domain(self.context.refuse(FnoReadError::Statement { row, reason }))
    }
    fn objects(
        &mut self,
        subject: &TermValue,
        predicate: &str,
    ) -> ReadResult<Vec<&'a TermValue>, R, C::Error> {
        let mut values = Vec::new();
        if let Some(props) = self.context.lookup(&self.index, &subject)? {
            if let Some(objects) = self.context.lookup(props, &predicate)? {
                for (&value, _) in objects.ordered() {
                    self.context.push(&mut values, value)?;
                }
            }
        }
        Ok(values)
    }
    fn field<T>(
        &mut self,
        subject: &TermValue,
        predicate: &str,
        expected: Field,
    ) -> ReadResult<T, R, C::Error> {
        let mut sources = self.objects(subject, predicate)?;
        let mut objects = Vec::new();
        for value in sources.drain(..) {
            let value = self.context.term(value)?;
            self.context.push(&mut objects, value)?;
        }
        self.context.release(sources)?;
        let subject = self.context.term(subject)?;
        let predicate = self.context.string(predicate)?;
        domain(self.context.refuse(FnoReadError::Field {
            subject,
            predicate,
            objects,
            expected,
        }))
    }
    fn claim(
        &mut self,
        subject: &TermValue,
        predicate: &str,
        object: &TermValue,
    ) -> ReadResult<(), R, C::Error> {
        if let Some(props) = self.context.lookup(&self.index, &subject)? {
            if let Some(objects) = self.context.lookup(props, &predicate)? {
                if let Some(rows) = self.context.lookup(objects, &object)? {
                    for &row in rows {
                        self.context.check(1)?;
                        self.claimed[row] = true;
                    }
                }
            }
        }
        Ok(())
    }
    fn scalar(
        &mut self,
        subject: &TermValue,
        predicate: &str,
        required: bool,
    ) -> ReadResult<Option<TermValue>, R, C::Error> {
        let mut values = self.objects(subject, predicate)?;
        if values.len() > 1 || required && values.is_empty() {
            self.context.release(values)?;
            return self.field(
                subject,
                predicate,
                if required {
                    Field::ExactlyOne
                } else {
                    Field::AtMostOne
                },
            );
        }
        let source = values.pop();
        self.context.release(values)?;
        if let Some(source) = source {
            self.claim(subject, predicate, source)?;
            Ok(Some(self.context.term(source)?))
        } else {
            Ok(None)
        }
    }
    fn iri(&mut self, subject: &TermValue, predicate: &str) -> ReadResult<String, R, C::Error> {
        match self.scalar(subject, predicate, true)? {
            Some(TermValue::Iri(value)) => Ok(value),
            value => {
                self.context.release(value)?;
                self.field(subject, predicate, Field::Iri)
            }
        }
    }
    fn optional_iri(
        &mut self,
        subject: &TermValue,
        predicate: &str,
    ) -> ReadResult<Option<String>, R, C::Error> {
        match self.scalar(subject, predicate, false)? {
            None => Ok(None),
            Some(TermValue::Iri(value)) => Ok(Some(value)),
            value => {
                self.context.release(value)?;
                self.field(subject, predicate, Field::Iri)
            }
        }
    }
    fn literal(
        &mut self,
        subject: &TermValue,
        predicate: &str,
        required: bool,
    ) -> ReadResult<Option<TermValue>, R, C::Error> {
        let value = self.scalar(subject, predicate, required)?;
        if value
            .as_ref()
            .is_some_and(|value| !matches!(value, TermValue::Literal { .. }))
        {
            self.context.release(value)?;
            return self.field(subject, predicate, Field::Literal);
        }
        Ok(value)
    }
    fn text(&mut self, subject: &TermValue, predicate: &str) -> ReadResult<TermValue, R, C::Error> {
        Ok(self
            .literal(subject, predicate, true)?
            .expect("required scalar was checked"))
    }
    fn plain(&mut self, subject: &TermValue, predicate: &str) -> ReadResult<String, R, C::Error> {
        let value = self.scalar(subject, predicate, true)?;
        if let Some(TermValue::Literal {
            lexical_form,
            datatype,
            language: None,
            direction: None,
        }) = &value
        {
            if self
                .context
                .compare(datatype.as_str(), purrdf_xsd::datatype::XSD_STRING)?
                .is_eq()
            {
                let result = self.context.string(lexical_form)?;
                self.context.release(value)?;
                return Ok(result);
            }
        }
        self.context.release(value)?;
        self.field(subject, predicate, Field::PlainString)
    }
    fn boolean(&mut self, subject: &TermValue, predicate: &str) -> ReadResult<bool, R, C::Error> {
        let value = self.scalar(subject, predicate, true)?;
        if let Some(TermValue::Literal {
            lexical_form,
            datatype,
            language: None,
            direction: None,
        }) = &value
        {
            if self
                .context
                .compare(datatype.as_str(), purrdf_xsd::datatype::XSD_BOOLEAN)?
                .is_eq()
            {
                let boolean = if self.context.compare(lexical_form.as_str(), "true")?.is_eq() {
                    Some(true)
                } else if self
                    .context
                    .compare(lexical_form.as_str(), "false")?
                    .is_eq()
                {
                    Some(false)
                } else {
                    None
                };
                if let Some(boolean) = boolean {
                    self.context.release(value)?;
                    return Ok(boolean);
                }
            }
        }
        self.context.release(value)?;
        self.field(subject, predicate, Field::Boolean)
    }
    fn subjects(&mut self, class: &str) -> ReadResult<Vec<&'a TermValue>, R, C::Error> {
        let selector = self.context.iri(class)?;
        let mut subjects = Vec::new();
        for (&subject, props) in self.index.ordered() {
            self.context.check(1)?;
            if let Some(types) = self.context.lookup(props, &RDF_TYPE)? {
                if self.context.lookup(types, &&selector)?.is_some() {
                    self.context.push(&mut subjects, subject)?;
                }
            }
        }
        self.context.release(selector)?;
        Ok(subjects)
    }
    fn typed(&mut self, subject: &TermValue, class: &str) -> ReadResult<(), R, C::Error> {
        let value = self.context.iri(class)?;
        let mut found = false;
        if let Some(props) = self.context.lookup(&self.index, &subject)? {
            if let Some(types) = self.context.lookup(props, &RDF_TYPE)? {
                found = self.context.lookup(types, &&value)?.is_some();
            }
        }
        if found {
            self.claim(subject, RDF_TYPE, &value)?;
            domain(self.context.release(value))
        } else {
            self.context.release(value)?;
            let class = self.context.string(class)?;
            self.field(subject, RDF_TYPE, Field::Role { class })
        }
    }
    fn subject_iri(&mut self, subject: &TermValue) -> ReadResult<String, R, C::Error> {
        if let TermValue::Iri(value) = subject {
            domain(self.context.string(value))
        } else {
            self.field(subject, RDF_TYPE, Field::Iri)
        }
    }
    fn blank(&mut self, subject: &TermValue) -> ReadResult<FnBlank, R, C::Error> {
        if let TermValue::Blank { label, scope } = subject {
            Ok(FnBlank {
                label: self.context.string(label)?,
                scope: *scope,
            })
        } else {
            self.field(subject, RDF_TYPE, Field::Blank)
        }
    }
    fn list(
        &mut self,
        subject: &TermValue,
        predicate: &str,
    ) -> ReadResult<Vec<&'a TermValue>, R, C::Error> {
        let mut heads = self.objects(subject, predicate)?;
        if heads.len() != 1 {
            self.context.release(heads)?;
            return self.field(subject, predicate, Field::ExactlyOne);
        }
        let head = heads.pop().expect("the required head was checked");
        self.context.release(heads)?;
        self.claim(subject, predicate, head)?;
        let props = self
            .context
            .lookup(&self.index, &subject)?
            .expect("the property was checked");
        let objects = self
            .context
            .lookup(props, &predicate)?
            .expect("the property was checked");
        let occurrences = self
            .context
            .lookup(objects, &head)?
            .expect("the object was checked");
        let head_id = self.rows[occurrences[0]].o;
        let edges = &self.edges;
        let walked = self.context.list(
            head_id,
            self.nil,
            |cell| edges[cell][0],
            |cell| edges[cell][1],
        )?;
        let members = match walked {
            Ok(members) => members,
            Err(cause) => {
                let mut members = Vec::new();
                for &at in &cause.members {
                    let value = self.context.term(&self.terms[at])?;
                    self.context.push(&mut members, value)?;
                }
                let node = self.context.term(&self.terms[cause.node])?;
                let kind = cause.kind;
                self.context.release(cause.members)?;
                let subject = self.context.term(subject)?;
                let predicate = self.context.string(predicate)?;
                return domain(self.context.refuse(FnoReadError::List {
                    subject,
                    predicate,
                    cause: crate::ListError {
                        kind,
                        members,
                        node,
                    },
                }));
            }
        };
        // Keep native fault precedence: check all members before cell ownership.
        let mut cell = head_id;
        for &member in &members {
            self.context.check(1)?;
            if !matches!(self.terms[member], TermValue::Iri(_)) {
                return self.field(&self.terms[cell], FIRST, Field::Iri);
            }
            let SoleObject::One(next) = self.edges[cell][1] else {
                unreachable!("the strict walk checked every rest edge")
            };
            cell = next;
        }
        cell = head_id;
        for _ in &members {
            self.context.check(1)?;
            let source = &self.terms[cell];
            let props = self
                .context
                .lookup(&self.index, &source)?
                .expect("the list cell was checked");
            let first = self
                .context
                .lookup(props, &FIRST)?
                .expect("the first edge was checked");
            let at = *first
                .ordered()
                .next()
                .and_then(|(_, rows)| rows.first())
                .expect("the first edge was checked");
            let row = self.rows[at];
            if !matches!(source, TermValue::Blank { .. }) {
                return self.statement(row, Shape::NonblankCell);
            }
            let reused = self.context.insert(&mut self.cells, source, ())?.is_some();
            if reused || self.context.lookup(props, &RDF_TYPE)?.is_some() {
                return self.statement(row, Shape::SharedCell);
            }
            for predicate in [FIRST, REST] {
                let mut objects = self.objects(source, predicate)?;
                for object in objects.drain(..) {
                    self.claim(source, predicate, object)?;
                }
                self.context.release(objects)?;
            }
            let SoleObject::One(next) = self.edges[cell][1] else {
                unreachable!("the strict walk checked every rest edge")
            };
            cell = next;
        }
        let mut values = Vec::new();
        for &at in &members {
            self.context.push(&mut values, &self.terms[at])?;
        }
        self.context.release(members)?;
        Ok(values)
    }
    fn catalog(&mut self, ontology: &str) -> ReadResult<FnoCatalogInput, R, C::Error> {
        let selector = self.context.iri(ontology)?;
        let mut documents = Vec::new();
        for (&subject, props) in self.index.ordered() {
            self.context.check(1)?;
            if let Some(values) = self.context.lookup(props, &DCTERMS_IS_PART_OF)? {
                if self.context.lookup(values, &&selector)?.is_some() {
                    self.context.push(&mut documents, subject)?;
                }
            }
        }
        self.context.release(selector)?;
        if documents.len() != 1 {
            let mut candidates = Vec::new();
            for subject in documents.drain(..) {
                let value = self.context.term(subject)?;
                self.context.push(&mut candidates, value)?;
            }
            self.context.release(documents)?;
            let ontology_iri = self.context.string(ontology)?;
            return domain(self.context.refuse(FnoReadError::Document {
                ontology_iri,
                candidates,
            }));
        }
        let doc = documents.pop().expect("one document was checked");
        self.context.release(documents)?;
        self.typed(doc, OWL_ONTOLOGY)?;
        let mut input = FnoCatalogInput {
            document_iri: self.subject_iri(doc)?,
            ontology_iri: self.iri(doc, DCTERMS_IS_PART_OF)?,
            doc_label: self.text(doc, RDFS_LABEL)?,
            banner: self.text(doc, RDFS_COMMENT)?,
            functions: Vec::new(),
            params: Vec::new(),
            implementations: Vec::new(),
            mappings: Vec::new(),
        };
        let mut subjects = self.subjects(fno!("Parameter"))?;
        for subject in subjects.drain(..) {
            self.typed(subject, fno!("Parameter"))?;
            let param = FnParam {
                iri: self.subject_iri(subject)?,
                predicate: self.optional_iri(subject, fno!("predicate"))?,
                r#type: self.iri(subject, fno!("type"))?,
                required: self.boolean(subject, fno!("required"))?,
                label: self.literal(subject, RDFS_LABEL, false)?,
                description: self.literal(subject, SKOS_DEFINITION, false)?,
            };
            self.context.push(&mut input.params, param)?;
        }
        self.context.release(subjects)?;
        let mut subjects = self.subjects(fno!("Implementation"))?;
        for subject in subjects.drain(..) {
            self.typed(subject, fno!("Implementation"))?;
            let implementation = FnImpl {
                iri: self.subject_iri(subject)?,
                format: self.plain(subject, DCTERMS_FORMAT)?,
                see_also: self.iri(subject, RDFS_SEE_ALSO)?,
            };
            self.context
                .push(&mut input.implementations, implementation)?;
        }
        self.context.release(subjects)?;
        let mut subjects = self.subjects(fno!("Function"))?;
        for subject in subjects.drain(..) {
            self.typed(subject, fno!("Function"))?;
            let mut outputs = self.list(subject, fno!("returns"))?;
            if outputs.len() != 1 {
                let mut heads = self.objects(subject, fno!("returns"))?;
                let head = self
                    .context
                    .term(heads.pop().expect("the list head was checked"))?;
                self.context.release(heads)?;
                let mut members = Vec::new();
                for output in outputs.drain(..) {
                    let value = self.context.term(output)?;
                    self.context.push(&mut members, value)?;
                }
                self.context.release(outputs)?;
                let subject = self.context.term(subject)?;
                let predicate = self.context.string(fno!("returns"))?;
                return domain(self.context.refuse(FnoReadError::Returns {
                    subject,
                    predicate,
                    head,
                    members,
                }));
            }
            let output = outputs.pop().expect("one output was checked");
            self.context.release(outputs)?;
            self.typed(output, fno!("Output"))?;
            let output = FnOutput {
                iri: self.subject_iri(output)?,
                predicate: self.optional_iri(output, fno!("predicate"))?,
                r#type: self.iri(output, fno!("type"))?,
                label: self.literal(output, RDFS_LABEL, false)?,
                description: self.literal(output, SKOS_DEFINITION, false)?,
            };
            let mut values = self.list(subject, fno!("expects"))?;
            let mut expects = Vec::new();
            for value in values.drain(..) {
                let TermValue::Iri(iri) = value else {
                    return self.field(subject, fno!("expects"), Field::Iri);
                };
                let iri = self.context.string(iri)?;
                self.context.push(&mut expects, iri)?;
            }
            self.context.release(values)?;
            let mut kinds = self.objects(subject, RDF_TYPE)?;
            let mut kind_types = Vec::new();
            for kind in kinds.drain(..) {
                let TermValue::Iri(iri) = kind else {
                    return self.field(subject, RDF_TYPE, Field::Iri);
                };
                let structural = iri.starts_with(FNO)
                    || iri.starts_with(FNOM)
                    || self.context.compare(subject, doc)?.is_eq()
                        && self.context.compare(iri.as_str(), OWL_ONTOLOGY)?.is_eq();
                if !structural {
                    self.claim(subject, RDF_TYPE, kind)?;
                    let kind = self.context.string(iri)?;
                    self.context.push(&mut kind_types, kind)?;
                }
            }
            self.context.release(kinds)?;
            let function = FnFunction {
                iri: self.subject_iri(subject)?,
                label: self.text(subject, RDFS_LABEL)?,
                description: self.literal(subject, SKOS_DEFINITION, false)?,
                kind_types,
                see_also: self.optional_iri(subject, RDFS_SEE_ALSO)?,
                expects,
                output,
            };
            self.context.push(&mut input.functions, function)?;
        }
        self.context.release(subjects)?;
        let mut subjects = self.subjects(fno!("Mapping"))?;
        for subject in subjects.drain(..) {
            self.typed(subject, fno!("Mapping"))?;
            let mut mapping = FnMapping {
                node: self.blank(subject)?,
                label: self.text(subject, RDFS_LABEL)?,
                function: self.iri(subject, fno!("function"))?,
                implementation: self.iri(subject, fno!("implementation"))?,
                parameter_mappings: Vec::new(),
                return_mappings: Vec::new(),
            };
            let mut children = self.objects(subject, fno!("parameterMapping"))?;
            for child in children.drain(..) {
                self.claim(subject, fno!("parameterMapping"), child)?;
                self.typed(child, fnom!("PropertyParameterMapping"))?;
                let child = FnParamMapping {
                    node: self.blank(child)?,
                    label: self.text(child, RDFS_LABEL)?,
                    function_parameter: self.iri(child, fnom!("functionParameter"))?,
                    implementation_property: self.plain(child, fnom!("implementationProperty"))?,
                };
                self.context.push(&mut mapping.parameter_mappings, child)?;
            }
            self.context.release(children)?;
            let mut children = self.objects(subject, fno!("returnMapping"))?;
            for child in children.drain(..) {
                self.claim(subject, fno!("returnMapping"), child)?;
                self.typed(child, fnom!("DefaultReturnMapping"))?;
                let child = FnReturnMapping {
                    node: self.blank(child)?,
                    label: self.text(child, RDFS_LABEL)?,
                    function_output: self.iri(child, fnom!("functionOutput"))?,
                    implementation_property: self.plain(child, fnom!("implementationProperty"))?,
                };
                self.context.push(&mut mapping.return_mappings, child)?;
            }
            self.context.release(children)?;
            self.context.push(&mut input.mappings, mapping)?;
        }
        self.context.release(subjects)?;
        for (at, &claimed) in self.claimed.iter().enumerate() {
            self.context.check(1)?;
            if !claimed {
                return self.statement(self.rows[at], Shape::Unclaimed);
            }
        }
        match super::admission::construct(self.context, &input) {
            Ok(staged) => {
                self.context.release(input)?;
                Ok(staged)
            }
            Err(ControlledError::Native(FnoConstructionFailure::Semantic(reason))) => domain(
                self.context
                    .refuse(FnoReadError::Admission(super::FnoRejectedCatalog {
                        input,
                        reason,
                    })),
            ),
            Err(ControlledError::Native(FnoConstructionFailure::Storage(cause))) => {
                Err(ReadFailure::Construction(ControlledError::Native(
                    FnoConstructionFailure::Storage(cause),
                )))
            }
            Err(ControlledError::Control(cause)) => {
                Err(ReadFailure::Construction(ControlledError::Control(cause)))
            }
            Err(ControlledError::Native(FnoConstructionFailure::Source(never))) => match never {},
        }
    }
}
