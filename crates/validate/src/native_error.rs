// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact JSON rendering of the native error-inspection protocol for host errors.

use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError};
use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError, retain_native_cause};
use purrdf_lex::control::ControlledError;
use purrdf_lex::json::{Object, OutputLayout, Value};

/// Own an original native failure through the shared diagnostic boundary.
/// A supplied storage cause preserves its native classification. Failure to retain
/// the original owner returns that physical storage refusal as the diagnostic.
#[must_use]
pub fn diagnostic(
    code: &str,
    error: impl purrdf_hash::error::NativeError + 'static,
    storage: Option<StorageError>,
) -> purrdf_core::RdfDiagnostic {
    use purrdf_core::RdfDiagnostic;
    let result = || {
        let mut resident = Resident;
        let diagnostic = match storage {
            Some(storage) => RdfDiagnostic::from(storage),
            None => {
                RdfDiagnostic::try_error_with_memory(code, &error, &mut Memory::new(&mut resident))?
            }
        };
        let retained = diagnostic
            .retained_bytes()
            .ok_or(StorageError::SizeOverflow)?;
        let mut memory = Memory::resume(&mut resident, retained);
        diagnostic.with_native_cause(retain_native_cause(error)?, &mut memory)
    };
    result().unwrap_or_else(RdfDiagnostic::from)
}

/// A native inspection protocol or original physical storage refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InspectionError(InspectionFailure);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InspectionFailure {
    Protocol(&'static str),
    Storage(StorageError),
}

impl InspectionError {
    fn protocol(reason: &'static str) -> Self {
        Self(InspectionFailure::Protocol(reason))
    }

    /// The original physical refusal, without interpreting presentation text.
    #[must_use]
    pub const fn storage_error(self) -> Option<StorageError> {
        match self.0 {
            InspectionFailure::Storage(cause) => Some(cause),
            InspectionFailure::Protocol(_) => None,
        }
    }
}

impl From<StorageError> for InspectionError {
    fn from(cause: StorageError) -> Self {
        Self(InspectionFailure::Storage(cause))
    }
}
impl std::fmt::Display for InspectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            InspectionFailure::Protocol(reason) => {
                write!(f, "native error inspection protocol failed: {reason}")
            }
            InspectionFailure::Storage(cause) => cause.fmt(f),
        }
    }
}
purrdf_hash::error_sources!(InspectionError, patterns
    Self(InspectionFailure::Storage(cause)) => cause,
);
impl StructuredError for InspectionError {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        match &self.0 {
            InspectionFailure::Protocol(reason) => {
                visitor.variant("native-inspection-protocol");
                visitor.value(Some("reason"), ErrorValue::Text(reason));
            }
            InspectionFailure::Storage(cause) => {
                visitor.variant("native-inspection-storage");
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
        }
    }
}

enum Cell<'error> {
    Scalar {
        kind: &'static str,
        value: ErrorValue<'error>,
    },
    Child {
        kind: &'static str,
        index: usize,
    },
}

enum Node<'error> {
    Record {
        code: Option<&'static str>,
        fields: Vec<(&'static str, Cell<'error>)>,
        items: Vec<Cell<'error>>,
    },
    List {
        length: usize,
        items: Vec<Cell<'error>>,
    },
}
impl Node<'_> {
    fn record(code: Option<&'static str>) -> Self {
        Self::Record {
            code,
            fields: Vec::new(),
            items: Vec::new(),
        }
    }
}

struct Inspector<'a, 'error, 'memory, S: Admission + ?Sized> {
    arena: &'a mut Vec<Node<'error>>,
    jobs: &'a mut Vec<(&'error dyn StructuredError, usize)>,
    stack: Vec<usize>,
    memory: &'a mut Memory<'memory, S>,
    error: Option<InspectionError>,
}
impl<'error, S: Admission + ?Sized> Inspector<'_, 'error, '_, S> {
    fn attempt(&mut self, body: impl FnOnce(&mut Self) -> Result<(), InspectionError>) {
        if self.error.is_none()
            && let Err(error) = body(self)
        {
            self.error = Some(error);
        }
    }
    fn append(
        &mut self,
        name: Option<&'static str>,
        cell: Cell<'error>,
    ) -> Result<(), InspectionError> {
        let index = *self
            .stack
            .last()
            .ok_or_else(|| InspectionError::protocol("a field has no enclosing container"))?;
        match &mut self.arena[index] {
            Node::Record { fields, items, .. } => match name {
                Some(name) if fields.iter().any(|(previous, _)| *previous == name) => {
                    return Err(InspectionError::protocol("a record repeats a field name"));
                }
                Some(name) => self.memory.push(fields, (name, cell))?,
                None => self.memory.push(items, cell)?,
            },
            Node::List { items, .. } if name.is_none() => self.memory.push(items, cell)?,
            Node::List { .. } => {
                return Err(InspectionError::protocol("a list entry has a field name"));
            }
        }
        Ok(())
    }
    fn child(
        &mut self,
        name: Option<&'static str>,
        kind: &'static str,
        source: &'error dyn StructuredError,
    ) -> Result<(), InspectionError> {
        let index = self.arena.len();
        self.memory.push(self.arena, Node::record(None))?;
        self.memory.push(self.jobs, (source, index))?;
        self.append(name, Cell::Child { kind, index })
    }
    fn open(
        &mut self,
        name: Option<&'static str>,
        kind: &'static str,
        node: Node<'error>,
    ) -> Result<(), InspectionError> {
        let index = self.arena.len();
        self.memory.push(self.arena, node)?;
        self.append(name, Cell::Child { kind, index })?;
        self.memory.push(&mut self.stack, index)?;
        Ok(())
    }
    fn close(&mut self, list: bool) -> Result<(), InspectionError> {
        if self.stack.len() <= 1 {
            return Err(InspectionError::protocol(
                "an end event has no matching begin event",
            ));
        }
        let index = self.stack.pop().expect("the stack has a nested container");
        if matches!(self.arena[index], Node::List { .. }) != list {
            return Err(InspectionError::protocol(
                "a container has the wrong end event",
            ));
        }
        Ok(())
    }
}
impl<'error, S: Admission + ?Sized> ErrorVisitor<'error> for Inspector<'_, 'error, '_, S> {
    fn variant(&mut self, code: &'static str) {
        self.attempt(|this| {
            match this.stack.last().map(|&index| &mut this.arena[index]) {
                Some(Node::Record { code: previous, .. }) if previous.is_none() => {
                    *previous = Some(code);
                }
                _ => {
                    return Err(InspectionError::protocol(
                        "a variant is outside a record or repeats its code",
                    ));
                }
            }
            Ok(())
        });
    }
    fn value(&mut self, name: Option<&'static str>, value: ErrorValue<'error>) {
        self.attempt(|this| {
            let kind = match value {
                ErrorValue::Cause(source) => return this.child(name, "cause", source),
                ErrorValue::Record(source) => return this.child(name, "record", source),
                ErrorValue::Term(source) => return this.child(name, "term", source),
                ErrorValue::Absent => "absent",
                ErrorValue::Unsigned(_) => "unsigned",
                ErrorValue::Signed(_) => "signed",
                ErrorValue::ByteCount(_) => "byte-count",
                ErrorValue::Float32Bits(_) => "float32-bits",
                ErrorValue::Float64Bits(_) => "float64-bits",
                ErrorValue::Boolean(_) => "boolean",
                ErrorValue::Character(_) => "character",
                ErrorValue::Code(_) => "code",
                ErrorValue::Text(_) => "text",
                ErrorValue::Message(_) => "message",
                ErrorValue::Iri(_) => "iri",
                ErrorValue::Bytes(_) => "bytes",
            };
            this.append(name, Cell::Scalar { kind, value })
        });
    }
    fn begin_list(&mut self, name: Option<&'static str>, length: usize) {
        self.attempt(|this| {
            this.open(
                name,
                "list",
                Node::List {
                    length,
                    items: Vec::new(),
                },
            )
        });
    }
    fn end_list(&mut self) {
        self.attempt(|this| this.close(true));
    }
    fn begin_record(&mut self, name: Option<&'static str>, code: &'static str) {
        self.attempt(|this| this.open(name, "record", Node::record(Some(code))));
    }
    fn end_record(&mut self) {
        self.attempt(|this| this.close(false));
    }
}

struct Captured<'error> {
    arena: Vec<Node<'error>>,
    scratch_bytes: usize,
}
fn capture<'error, S: Admission + ?Sized>(
    source: &'error dyn StructuredError,
    memory: &mut Memory<'_, S>,
) -> Result<Captured<'error>, InspectionError> {
    let baseline = memory.admitted_bytes();
    let mut arena = Vec::new();
    let mut jobs = Vec::new();
    memory.push(&mut arena, Node::record(None))?;
    memory.push(&mut jobs, (source, 0))?;
    while let Some((source, root)) = jobs.pop() {
        let mut stack = Vec::new();
        memory.push(&mut stack, root)?;
        let mut inspector = Inspector {
            arena: &mut arena,
            jobs: &mut jobs,
            stack,
            memory,
            error: None,
        };
        source.visit(&mut inspector);
        if inspector.stack.len() != 1 && inspector.error.is_none() {
            inspector.error = Some(InspectionError::protocol("a visit left a container open"));
        }
        let Inspector { stack, error, .. } = inspector;
        let refund = memory.release_vec(stack);
        if let Some(error) = error {
            return Err(error);
        }
        refund?;
    }
    memory.release_vec(jobs)?;
    let scratch_bytes = memory
        .admitted_bytes()
        .checked_sub(baseline)
        .ok_or(StorageError::SizeOverflow)?;
    Ok(Captured {
        arena,
        scratch_bytes,
    })
}
fn bounded<T>(value: Option<T>) -> Result<T, InspectionError> {
    value.ok_or_else(|| StorageError::SizeOverflow.into())
}
fn record_layout(members: usize, text: usize) -> Result<OutputLayout, InspectionError> {
    bounded(OutputLayout::record(members as u64, text as u64))
}
fn tagged_layout(kind: &str, value: OutputLayout) -> Result<OutputLayout, InspectionError> {
    bounded(record_layout(2, kind.len() + 9)?.with_child(value))
}
fn scalar_layout(value: ErrorValue<'_>) -> Result<OutputLayout, InspectionError> {
    let text = match value {
        ErrorValue::Absent | ErrorValue::Boolean(_) => return Ok(OutputLayout::scalar()),
        ErrorValue::Unsigned(_) | ErrorValue::ByteCount(_) => 39,
        ErrorValue::Signed(_) => 40,
        ErrorValue::Float32Bits(_) => 10,
        ErrorValue::Float64Bits(_) => 20,
        ErrorValue::Character(value) => value.len_utf8(),
        ErrorValue::Code(value)
        | ErrorValue::Text(value)
        | ErrorValue::Message(value)
        | ErrorValue::Iri(value) => value.len(),
        ErrorValue::Bytes(value) => bounded(value.len().checked_mul(2))?,
        _ => {
            return Err(InspectionError::protocol(
                "a compound value has no child node",
            ));
        }
    };
    record_layout(0, text)
}
fn cell_layout(
    cell: &Cell<'_>,
    completed: &[Option<OutputLayout>],
) -> Result<OutputLayout, InspectionError> {
    let (kind, child) = match cell {
        Cell::Scalar { kind, value } => (*kind, scalar_layout(*value)?),
        Cell::Child { kind, index } => (
            *kind,
            completed.get(*index).copied().flatten().ok_or_else(|| {
                InspectionError::protocol("a child record is absent or was consumed twice")
            })?,
        ),
    };
    tagged_layout(kind, child)
}
impl Captured<'_> {
    fn layout<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> Result<OutputLayout, InspectionError> {
        let mut completed = Vec::new();
        memory.reserve(&mut completed, self.arena.len())?;
        completed.resize(self.arena.len(), None);
        for (index, node) in self.arena.iter().enumerate().rev() {
            let layout = match node {
                Node::Record {
                    code,
                    fields,
                    items,
                } => {
                    let code = code
                        .ok_or_else(|| InspectionError::protocol("a record has no variant code"))?;
                    let mut text = 0_usize;
                    for (name, _) in fields {
                        text = bounded(text.checked_add(name.len()))?;
                    }
                    let mut field_layout = record_layout(fields.len(), text)?;
                    for (_, cell) in fields {
                        field_layout =
                            bounded(field_layout.with_child(cell_layout(cell, &completed)?))?;
                    }
                    let mut item_layout = bounded(OutputLayout::array(
                        items.len() as u64,
                        OutputLayout::scalar(),
                    ))?;
                    for cell in items {
                        item_layout =
                            bounded(item_layout.with_child(cell_layout(cell, &completed)?))?;
                    }
                    bounded(
                        record_layout(3, bounded(code.len().checked_add(15))?)?
                            .with_child(field_layout)
                            .and_then(|layout| layout.with_child(item_layout)),
                    )?
                }
                Node::List { length, items } => {
                    if *length != items.len() {
                        return Err(InspectionError::protocol(
                            "a list length disagrees with its entries",
                        ));
                    }
                    let mut layout = bounded(OutputLayout::array(
                        items.len() as u64,
                        OutputLayout::scalar(),
                    ))?;
                    for cell in items {
                        layout = bounded(layout.with_child(cell_layout(cell, &completed)?))?;
                    }
                    layout
                }
            };
            completed[index] = Some(layout);
        }
        let layout = completed[0]
            .take()
            .ok_or_else(|| InspectionError::protocol("the root record is absent"))?;
        memory.release_vec(completed)?;
        // Both assembly tables coexist with the borrowed capture; record/array
        // layouts separately include the owned output and native writer frames.
        let slots = bounded(self.arena.len().checked_mul(2))?.max(4);
        let tables = bounded(
            slots.checked_mul(size_of::<Option<Value>>() + size_of::<Option<OutputLayout>>()),
        )?;
        let scratch = bounded(self.scratch_bytes.checked_add(tables))?;
        bounded(layout.with_formatting(
            bounded((self.arena.len() as u64).checked_mul(4))?,
            scratch as u64,
        ))
    }
    fn into_json<S: Admission + ?Sized>(
        mut self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Value, InspectionError> {
        let mut completed = Vec::new();
        memory.reserve(&mut completed, self.arena.len())?;
        completed.resize_with(self.arena.len(), || None);
        while let Some(node) = self.arena.pop() {
            let index = self.arena.len();
            let value = match node {
                Node::Record {
                    code,
                    fields,
                    items,
                } => {
                    let code = code
                        .ok_or_else(|| InspectionError::protocol("a record has no variant code"))?;
                    let mut output_fields = Object::new();
                    for (name, cell) in &fields {
                        output_fields = output_fields.with_memory(
                            name,
                            finish_cell(cell, &mut completed, memory)?,
                            memory,
                        )?;
                    }
                    memory.release_vec(fields)?;
                    let mut output_items = Vec::new();
                    for cell in &items {
                        let value = finish_cell(cell, &mut completed, memory)?;
                        memory.push(&mut output_items, value)?;
                    }
                    memory.release_vec(items)?;
                    Object::new()
                        .with_memory("code", code, memory)?
                        .with_memory("fields", Value::Object(output_fields), memory)?
                        .with_memory("items", Value::Array(output_items), memory)?
                        .into()
                }
                Node::List { length, items } => {
                    if length != items.len() {
                        return Err(InspectionError::protocol(
                            "a list length disagrees with its entries",
                        ));
                    }
                    let mut output = Vec::new();
                    for cell in &items {
                        let value = finish_cell(cell, &mut completed, memory)?;
                        memory.push(&mut output, value)?;
                    }
                    memory.release_vec(items)?;
                    Value::Array(output)
                }
            };
            completed[index] = Some(value);
        }
        let value = completed[0]
            .take()
            .ok_or_else(|| InspectionError::protocol("the root record is absent"))?;
        memory.release_vec(completed)?;
        memory.release_vec(self.arena)?;
        Ok(value)
    }
}
fn tagged<S: Admission + ?Sized>(
    kind: &str,
    value: Value,
    memory: &mut Memory<'_, S>,
) -> Result<Value, StorageError> {
    Ok(Object::new()
        .with_memory("kind", kind, memory)?
        .with_memory("value", value, memory)?
        .into())
}
fn scalar_value<S: Admission + ?Sized>(
    value: ErrorValue<'_>,
    memory: &mut Memory<'_, S>,
) -> Result<Value, InspectionError> {
    let text = match value {
        ErrorValue::Absent => return Ok(Value::Null),
        ErrorValue::Boolean(value) => return Ok(Value::Bool(value)),
        ErrorValue::Unsigned(value) | ErrorValue::ByteCount(value) => memory.format(&value)?,
        ErrorValue::Signed(value) => memory.format(&value)?,
        ErrorValue::Float32Bits(value) => memory.format(&value)?,
        ErrorValue::Float64Bits(value) => memory.format(&value)?,
        ErrorValue::Character(value) => memory.format(&value)?,
        ErrorValue::Code(value)
        | ErrorValue::Text(value)
        | ErrorValue::Message(value)
        | ErrorValue::Iri(value) => memory.string(value)?,
        ErrorValue::Bytes(value) => {
            let mut text = String::new();
            memory.reserve_string(&mut text, bounded(value.len().checked_mul(2))?)?;
            purrdf_hash::hex::encode_into(value, &mut text);
            text
        }
        _ => {
            return Err(InspectionError::protocol(
                "a compound value has no child node",
            ));
        }
    };
    Ok(Value::String(text))
}
fn finish_cell<S: Admission + ?Sized>(
    cell: &Cell<'_>,
    completed: &mut [Option<Value>],
    memory: &mut Memory<'_, S>,
) -> Result<Value, InspectionError> {
    let (kind, value) = match cell {
        Cell::Scalar { kind, value } => (*kind, scalar_value(*value, memory)?),
        Cell::Child { kind, index } => (
            *kind,
            completed
                .get_mut(*index)
                .and_then(Option::take)
                .ok_or_else(|| {
                    InspectionError::protocol("a child record is absent or was consumed twice")
                })?,
        ),
    };
    Ok(tagged(kind, value, memory)?)
}

/// Render native fields through their original physical account. The admission
/// callback sees the complete output/work/scratch bound before JSON construction;
/// its original refusal and every physical failure are returned unchanged.
/// The returned value retains its original grant. Keep the account alive until
/// that value is destroyed; scratch is destroyed before its admission is refunded.
///
/// # Errors
/// Returns native protocol/storage refusal or the callback's control refusal.
pub fn to_json_with_admission<S: Admission + ?Sized, E>(
    source: &dyn StructuredError,
    memory: &mut Memory<'_, S>,
    admit: impl FnOnce(OutputLayout) -> Result<(), E>,
) -> Result<Value, ControlledError<InspectionError, E>> {
    memory.try_scope(|memory| {
        let captured = capture(source, memory).map_err(ControlledError::Native)?;
        let layout = captured.layout(memory).map_err(ControlledError::Native)?;
        admit(layout).map_err(ControlledError::Control)?;
        captured.into_json(memory).map_err(ControlledError::Native)
    })
}

/// Render native fields using the original physical account and no extra policy.
///
/// # Errors
/// Returns original storage refusal or an actionable inspection protocol failure.
pub fn to_json_with_memory<S: Admission + ?Sized>(
    source: &dyn StructuredError,
    memory: &mut Memory<'_, S>,
) -> Result<Value, InspectionError> {
    match to_json_with_admission(source, memory, |_| Ok::<_, core::convert::Infallible>(())) {
        Ok(value) => Ok(value),
        Err(ControlledError::Native(error)) => Err(error),
        Err(ControlledError::Control(never)) => match never {},
    }
}

/// Render the original closed fields without parsing Display text. Integer and
/// float-bit payloads remain decimal strings and bytes remain lowercase hex.
///
/// # Errors
/// Returns actionable protocol failure or the original physical refusal.
pub fn to_json(source: &dyn StructuredError) -> Result<Value, InspectionError> {
    to_json_with_memory(source, &mut Memory::new(&mut Resident))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every binding publishes a canonical refusal through `to_json` and finds it
    /// again through `native_canonical_cause`; a source that spells a term with a
    /// different length on a second read must arrive as its own code, and an
    /// ordinary refusal must keep its own.
    #[test]
    fn source_inconsistent_is_published_with_its_own_code_and_found_again() {
        let refusal = purrdf_core::CanonError::<std::convert::Infallible>::SourceInconsistent;
        let fields = to_json(&refusal).unwrap().to_string();
        assert!(fields.contains("canonical-source-inconsistent"), "{fields}");
        assert!(matches!(
            crate::canonical::native_canonical_cause(&refusal),
            Some(purrdf_core::CanonError::<std::convert::Infallible>::SourceInconsistent)
        ));
        let neighbour = to_json(&purrdf_core::CanonError::<std::convert::Infallible>::Stopped)
            .unwrap()
            .to_string();
        assert!(
            !neighbour.contains("canonical-source-inconsistent"),
            "{neighbour}"
        );
    }

    struct ListRefusal(usize);

    impl StructuredError for ListRefusal {
        fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
            visitor.variant("native-list-refusal");
            visitor.begin_list(Some("coordinates"), self.0);
            visitor.value(None, ErrorValue::Unsigned(u128::MAX));
            visitor.end_list();
        }
    }

    #[test]
    fn malformed_native_list_refuses_instead_of_publishing_partial_fields() {
        assert!(to_json(&ListRefusal(2)).is_err());
        let admitted = to_json(&ListRefusal(1)).unwrap().to_string();
        assert!(admitted.contains("native-list-refusal"));
        assert!(
            admitted.contains(&format!("\"{}\"", u128::MAX)),
            "{admitted}"
        );
    }

    struct NativeFields<'a> {
        text: &'a str,
        cause: purrdf_core::LiteralComponentError,
    }
    impl StructuredError for NativeFields<'_> {
        fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
            visitor.variant("native-json-fields");
            for (name, value) in [
                ("unsigned", ErrorValue::Unsigned(u128::MAX)),
                ("signed", ErrorValue::Signed(i128::MIN)),
                ("bytes", ErrorValue::Bytes(&[0, 255])),
                ("count", ErrorValue::ByteCount(u128::MAX)),
                ("f32", ErrorValue::Float32Bits(u32::MAX)),
                ("f64", ErrorValue::Float64Bits(u64::MAX)),
                ("character", ErrorValue::Character('é')),
                ("flag", ErrorValue::Boolean(true)),
                ("text", ErrorValue::Text(self.text)),
                ("code", ErrorValue::Code("example")),
                ("iri", ErrorValue::Iri("https://example.org/")),
                ("message", ErrorValue::Message("authored")),
                ("absent", ErrorValue::Absent),
                ("cause", ErrorValue::Cause(&self.cause)),
            ] {
                visitor.value(Some(name), value);
            }
        }
    }

    #[derive(Default)]
    struct Account {
        live: usize,
        peak: usize,
        growths: usize,
        refuse: Option<usize>,
        requested: Option<usize>,
    }
    impl Admission for Account {
        fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
            if bytes > self.live {
                let event = self.growths;
                self.growths += 1;
                if self.refuse == Some(event) {
                    self.requested = Some(bytes);
                    return Err(StorageError::AdmissionFailed);
                }
            }
            self.live = bytes;
            self.peak = self.peak.max(bytes);
            Ok(())
        }
    }

    #[test]
    fn native_json_keeps_original_fields_and_admits_every_owned_buffer() {
        let text = "é\n\"".to_owned();
        let source = NativeFields {
            text: &text,
            cause: purrdf_core::LiteralComponentError::LanguageTag(
                purrdf_iri::langtag::LanguageTagError::TerminalPrimaryNotAlpha,
            ),
        };
        let mut account = Account::default();
        let mut memory = Memory::new(&mut account);
        let mut original = Vec::<u8>::new();
        memory.reserve(&mut original, 17).unwrap();
        let baseline = memory.admitted_bytes();
        let bound = core::cell::Cell::new(None);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let value = to_json_with_admission(&source, &mut memory, |layout| {
            bound.set(Some(layout));
            Ok::<_, usize>(())
        })
        .unwrap();
        let measured = window.close();
        let bytes = memory.admitted_bytes() - baseline;
        assert_eq!(bytes, usize::try_from(measured.retained_bytes).unwrap());
        assert!(
            bound.get().unwrap().workspace_bytes().unwrap()
                >= u64::try_from(memory.admission().peak - baseline).unwrap()
        );
        assert_eq!(
            value["fields"]["unsigned"]["value"].as_str(),
            Some(u128::MAX.to_string().as_str())
        );
        assert_eq!(
            value["fields"]["signed"]["value"].as_str(),
            Some(i128::MIN.to_string().as_str())
        );
        assert_eq!(value["fields"]["bytes"]["value"].as_str(), Some("00ff"));
        assert_eq!(
            value["fields"]["count"]["value"].as_str(),
            Some(u128::MAX.to_string().as_str())
        );
        assert_eq!(
            value["fields"]["f32"]["value"].as_str(),
            Some(u32::MAX.to_string().as_str())
        );
        assert_eq!(
            value["fields"]["f64"]["value"].as_str(),
            Some(u64::MAX.to_string().as_str())
        );
        assert_eq!(value["fields"]["character"]["value"].as_str(), Some("é"));
        assert_eq!(value["fields"]["flag"]["value"], Value::Bool(true));
        assert_eq!(
            value["fields"]["cause"]["value"]["fields"]["cause"]["value"]["code"].as_str(),
            Some("langtag-terminal-primary-not-alpha")
        );
        assert_eq!(
            value["fields"]["text"]["value"].as_str(),
            Some(text.as_str())
        );
        drop(text);
        assert_eq!(value["fields"]["text"]["value"].as_str(), Some("é\n\""));
        drop(value);
        memory.release_bytes(bytes).unwrap();
        assert_eq!(memory.admitted_bytes(), baseline);
        memory.release_vec(original).unwrap();
        let growths = account.growths;
        for refuse in 1..growths {
            let source = NativeFields {
                text: "kept",
                cause: purrdf_core::LiteralComponentError::LanguageTag(
                    purrdf_iri::langtag::LanguageTagError::TerminalPrimaryNotAlpha,
                ),
            };
            let mut account = Account {
                refuse: Some(refuse),
                ..Account::default()
            };
            let mut memory = Memory::new(&mut account);
            let mut original = Vec::<u8>::new();
            memory.reserve(&mut original, 17).unwrap();
            let baseline = memory.admitted_bytes();
            let failure = to_json_with_memory(&source, &mut memory).unwrap_err();
            assert_eq!(failure.storage_error(), Some(StorageError::AdmissionFailed));
            assert_eq!(
                std::error::Error::source(&failure)
                    .and_then(|cause| cause.downcast_ref::<StorageError>()),
                Some(&StorageError::AdmissionFailed),
            );
            assert_eq!(memory.admitted_bytes(), baseline);
            memory.release_vec(original).unwrap();
            assert_eq!(account.live, 0);
            assert!(account.requested.unwrap() > baseline);
        }
        let mut account = Account::default();
        let mut memory = Memory::new(&mut account);
        let refused = to_json_with_admission(&ListRefusal(1), &mut memory, |_| Err(73_usize));
        assert!(matches!(refused, Err(ControlledError::Control(73))));
        assert_eq!(memory.admitted_bytes(), 0);
    }
}
