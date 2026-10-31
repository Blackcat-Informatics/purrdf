// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native construction storage, work and retained publication.

use super::{
    FnBlank, FnFunction, FnImpl, FnMapping, FnOutput, FnParam, FnParamMapping, FnReturnMapping,
    FnoAdmissionReason, FnoCatalogInput,
};
use crate::{TermValue, ir::OwnedTermFoldError};
use purrdf_lex::allocation::{Admission, Memory, OwnedAdmission, SharedOwned, StorageError};
use purrdf_lex::control::{ControlledError, Progress, Unmetered};

/// A native catalog construction failure, separate from caller progress refusal.
#[derive(Clone, Debug)]
pub enum FnoConstructionFailure<N, R = core::convert::Infallible> {
    /// Original independently owned provider failure; reporting allocates nothing.
    Source(R),
    /// Checked layout, admission or physical allocator refusal.
    Storage(StorageError),
    /// The original native semantic failure, retaining its original allocation owner.
    Semantic(N),
}

impl<N, R> From<StorageError> for FnoConstructionFailure<N, R> {
    fn from(cause: StorageError) -> Self {
        Self::Storage(cause)
    }
}

impl<N: core::fmt::Display, R: core::fmt::Display> core::fmt::Display
    for FnoConstructionFailure<SharedOwned<N>, R>
{
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Storage(cause) => cause.fmt(out),
            Self::Semantic(cause) => core::fmt::Display::fmt(&**cause, out),
            Self::Source(cause) => cause.fmt(out),
        }
    }
}
impl<N: std::error::Error + 'static, R: std::error::Error + 'static> std::error::Error
    for FnoConstructionFailure<SharedOwned<N>, R>
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Storage(cause) => Some(cause),
            Self::Semantic(cause) => Some(&**cause),
            Self::Source(cause) => Some(cause),
        }
    }
}
impl<N: PartialEq, R: PartialEq> PartialEq for FnoConstructionFailure<SharedOwned<N>, R> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Storage(a), Self::Storage(b)) => a == b,
            (Self::Semantic(a), Self::Semantic(b)) => **a == **b,
            (Self::Source(a), Self::Source(b)) => a == b,
            _ => false,
        }
    }
}
impl<N: Eq, R: Eq> Eq for FnoConstructionFailure<SharedOwned<N>, R> {}
impl<N: purrdf_hash::error::StructuredError, R: purrdf_hash::error::StructuredError>
    purrdf_hash::error::StructuredError for FnoConstructionFailure<SharedOwned<N>, R>
{
    fn visit<'a>(&'a self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'a>) {
        use purrdf_hash::error::ErrorValue;
        match self {
            Self::Source(cause) => cause.visit(visitor),
            Self::Storage(cause) => {
                visitor.variant("fno-construction-storage");
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
            Self::Semantic(cause) => {
                visitor.variant("fno-construction-semantic");
                visitor.value(Some("cause"), ErrorValue::Cause(&**cause));
            }
        }
    }
}

/// Original caller fields and a controlled constructor's first refusal.
///
/// The input is returned with its original capacities, records and ordering;
/// neither validation nor interrupted canonicalization mutates it.
#[derive(Debug)]
pub struct FnoConstructorRefusal<E> {
    /// The original caller input, unchanged.
    pub input: FnoCatalogInput,
    /// Original progress/storage failure or an admitted native semantic reason.
    pub cause: ControlledError<FnoConstructionFailure<SharedOwned<FnoAdmissionReason>>, E>,
}

impl<E: core::fmt::Display> core::fmt::Display for FnoConstructorRefusal<E> {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.cause.fmt(out)
    }
}
impl<E: std::error::Error + 'static> std::error::Error for FnoConstructorRefusal<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}

pub(super) type Failure<N, E, R = core::convert::Infallible> =
    ControlledError<FnoConstructionFailure<N, R>, E>;
pub(super) type Result<T, N, E> = core::result::Result<T, Failure<N, E>>;

pub(super) fn with_source<N, E, R>(cause: Failure<N, E>) -> Failure<N, E, R> {
    match cause {
        ControlledError::Control(cause) => ControlledError::Control(cause),
        ControlledError::Native(FnoConstructionFailure::Storage(cause)) => {
            ControlledError::Native(FnoConstructionFailure::Storage(cause))
        }
        ControlledError::Native(FnoConstructionFailure::Semantic(cause)) => {
            ControlledError::Native(FnoConstructionFailure::Semantic(cause))
        }
        ControlledError::Native(FnoConstructionFailure::Source(never)) => match never {},
    }
}

/// One operation's original account. Local payloads are declared after this
/// context, so unsuccessful construction destroys them before the account.
pub(super) struct Context<S: Admission, C> {
    pub(super) progress: C,
    live: usize,
    storage: OwnedAdmission<S>,
}

impl<S: Admission, C: Progress> Context<S, C> {
    pub(super) fn new(storage: S, progress: C) -> Self {
        Self {
            progress,
            live: 0,
            storage: OwnedAdmission::new(storage),
        }
    }

    pub(super) fn check<N>(&mut self, units: u64) -> Result<(), N, C::Error> {
        self.progress
            .charge(units)
            .map_err(ControlledError::Control)
    }

    pub(super) fn native<T, N: From<StorageError>>(
        &mut self,
        body: impl FnOnce(
            &mut Memory<'_, purrdf_lex::control::ControlledAdmission<'_, OwnedAdmission<S>, C>>,
        ) -> core::result::Result<T, N>,
    ) -> core::result::Result<T, ControlledError<N, C::Error>> {
        let mut memory = if self.live == 0 {
            Memory::new(&mut self.storage)
        } else {
            Memory::resume(&mut self.storage, self.live)
        };
        let result =
            purrdf_lex::control::with_memory_control(&mut memory, &mut self.progress, body);
        self.live = memory.admitted_bytes();
        result
    }

    pub(super) fn memory<T, N>(
        &mut self,
        body: impl FnOnce(
            &mut Memory<'_, purrdf_lex::control::ControlledAdmission<'_, OwnedAdmission<S>, C>>,
        ) -> core::result::Result<T, StorageError>,
    ) -> Result<T, N, C::Error> {
        self.native(body).map_err(|cause| match cause {
            ControlledError::Native(cause) => {
                ControlledError::Native(FnoConstructionFailure::Storage(cause))
            }
            ControlledError::Control(cause) => ControlledError::Control(cause),
        })
    }

    pub(super) fn string<N>(&mut self, source: &str) -> Result<String, N, C::Error> {
        self.memory(|memory| memory.string_with_progress(source))
    }

    pub(super) fn iri<N>(&mut self, source: &str) -> Result<TermValue, N, C::Error> {
        self.string(source).map(TermValue::Iri)
    }

    pub(super) fn absolute<N>(
        &mut self,
        source: &str,
    ) -> Result<core::result::Result<(), purrdf_iri::IriError>, N, C::Error> {
        match self.native(|memory| crate::ir::absolute::check_absolute_with_memory(source, memory))
        {
            Ok(()) => Ok(Ok(())),
            Err(ControlledError::Native(purrdf_iri::IriReadError::Lexical(cause))) => {
                Ok(Err(cause))
            }
            Err(ControlledError::Native(purrdf_iri::IriReadError::Storage(cause))) => Err(
                ControlledError::Native(FnoConstructionFailure::Storage(cause)),
            ),
            Err(ControlledError::Control(cause)) => Err(ControlledError::Control(cause)),
        }
    }

    pub(super) fn literal<N>(
        &mut self,
        value: &TermValue,
    ) -> Result<core::result::Result<(), crate::RdfDiagnostic>, N, C::Error> {
        let TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } = value
        else {
            unreachable!("the FnO literal-kind boundary was checked")
        };
        match self.native(|memory| {
            crate::ir::validate::validate_literal_with_memory(
                lexical_form,
                datatype,
                language.as_deref(),
                *direction,
                memory,
            )
        }) {
            Ok(()) => Ok(Ok(())),
            Err(ControlledError::Native(crate::ir::builder::NativeBuildError::Diagnostic(
                cause,
            ))) => Ok(Err(cause)),
            Err(ControlledError::Native(crate::ir::builder::NativeBuildError::Storage(cause))) => {
                Err(ControlledError::Native(FnoConstructionFailure::Storage(
                    cause,
                )))
            }
            Err(ControlledError::Control(cause)) => Err(ControlledError::Control(cause)),
        }
    }

    pub(super) fn push<T, N>(&mut self, values: &mut Vec<T>, value: T) -> Result<(), N, C::Error> {
        self.check(1)?;
        self.memory(|memory| memory.push(values, value))
    }

    pub(super) fn term<N>(&mut self, source: &TermValue) -> Result<TermValue, N, C::Error> {
        self.memory(|memory| {
            source
                .try_clone_with_memory(memory, |_| Ok::<(), core::convert::Infallible>(()))
                .map_err(|cause| match cause {
                    OwnedTermFoldError::Storage(cause) => cause,
                    OwnedTermFoldError::Visitor(never) => match never {},
                })
        })
    }

    pub(super) fn list<Id: Copy + Eq, N>(
        &mut self,
        head: Id,
        nil: Option<Id>,
        first: impl FnMut(Id) -> crate::SoleObject<Id>,
        rest: impl FnMut(Id) -> crate::SoleObject<Id>,
    ) -> Result<core::result::Result<Vec<Id>, crate::ListError<Id>>, N, C::Error> {
        self.physical(|memory, progress| {
            crate::collections::walk_rdf_list_with_control(
                head,
                nil,
                first,
                rest,
                memory,
                &mut |units| progress.charge(units),
            )
        })
    }

    pub(super) fn copy<T: CopyFields, N>(&mut self, value: &T) -> Result<T, N, C::Error> {
        value.copy_fields(self)
    }

    pub(super) fn refuse<T, N>(&mut self, reason: N) -> Result<T, N, C::Error> {
        Err(ControlledError::Native(FnoConstructionFailure::Semantic(
            reason,
        )))
    }

    fn physical<T, N>(
        &mut self,
        body: impl FnOnce(&mut Memory<'_, OwnedAdmission<S>>, &mut C) -> PhysicalResult<T, C::Error>,
    ) -> Result<T, N, C::Error> {
        let mut memory = if self.live == 0 {
            Memory::new(&mut self.storage)
        } else {
            Memory::resume(&mut self.storage, self.live)
        };
        let result = body(&mut memory, &mut self.progress);
        self.live = memory.admitted_bytes();
        result.map_err(|cause| match cause {
            ControlledError::Native(cause) => {
                ControlledError::Native(FnoConstructionFailure::Storage(cause))
            }
            ControlledError::Control(cause) => ControlledError::Control(cause),
        })
    }

    pub(super) fn compare<T: CompareFields + ?Sized, N>(
        &mut self,
        a: &T,
        b: &T,
    ) -> Result<core::cmp::Ordering, N, C::Error> {
        self.physical(|memory, progress| a.compare_fields(b, memory, progress))
    }

    pub(super) fn insert<K: CompareFields + RetainedFields, V, N>(
        &mut self,
        table: &mut purrdf_lex::allocation::OrderedTable<K, V>,
        key: K,
        value: V,
    ) -> Result<Option<V>, N, C::Error> {
        let key_bytes = self.measure(&key)?;
        let (previous, discarded) = self.physical(|memory, progress| {
            let entry = table.prepare_insert_with_memory(key, memory, |a, b, memory| {
                a.compare_fields(b, memory, progress)
            })?;
            let discarded = entry.is_occupied();
            Ok((entry.publish(value).1, discarded))
        })?;
        if discarded && key_bytes != 0 {
            self.memory(|memory| memory.release_bytes(key_bytes))?;
        }
        Ok(previous)
    }

    pub(super) fn lookup<'a, K: CompareFields, V, N>(
        &mut self,
        table: &'a purrdf_lex::allocation::OrderedTable<K, V>,
        key: &K,
    ) -> Result<Option<&'a V>, N, C::Error> {
        self.physical(|memory, progress| {
            table.try_get_by(|candidate| key.compare_fields(candidate, memory, progress))
        })
    }

    pub(super) fn sort<T: CompareFields, N>(
        &mut self,
        values: &mut [T],
    ) -> Result<(), N, C::Error> {
        self.physical(|memory, progress| {
            purrdf_lex::walk::try_sort_unstable_by(values, |a, b| {
                a.compare_fields(b, memory, progress)
            })
        })
    }

    pub(super) fn dedup<T: CompareFields + RetainedFields, N>(
        &mut self,
        values: &mut Vec<T>,
    ) -> Result<(), N, C::Error> {
        let kept = self.physical(|memory, progress| {
            purrdf_lex::walk::try_partition_unique_by(values, |a, b| {
                a.compare_fields(b, memory, progress)
                    .map(core::cmp::Ordering::is_eq)
            })
        })?;
        while values.len() > kept {
            let value = values.pop().expect("the duplicate suffix is nonempty");
            self.release(value)?;
        }
        Ok(())
    }

    pub(super) const fn live(&self) -> usize {
        self.live
    }

    pub(super) fn measure<T: RetainedFields + ?Sized, N>(
        &mut self,
        value: &T,
    ) -> Result<usize, N, C::Error> {
        self.physical(|memory, progress| value.retained_fields(memory, progress))
    }

    pub(super) fn shrink<N>(&mut self, retained: usize) -> Result<(), N, C::Error> {
        let released = self
            .live
            .checked_sub(retained)
            .ok_or(ControlledError::Native(FnoConstructionFailure::Storage(
                StorageError::SizeOverflow,
            )))?;
        if released != 0 {
            self.memory(|memory| memory.release_bytes(released))?;
        }
        Ok(())
    }

    pub(super) fn release<T: RetainedFields, N>(&mut self, value: T) -> Result<(), N, C::Error> {
        let bytes = self.measure(&value)?;
        drop(value);
        if bytes != 0 {
            self.memory(|memory| memory.release_bytes(bytes))?;
        }
        Ok(())
    }

    /// A native error is already fully owned. Its destruction census and
    /// reporting header must not poll a later caller stop or replace its cause.
    pub(super) fn publish_refusal<T: RetainedFields + Send + Sync + 'static, N>(
        mut self,
        value: T,
    ) -> Result<SharedOwned<T>, N, C::Error>
    where
        S: Send + Sync + 'static,
    {
        let refusal = |cause| ControlledError::Native(FnoConstructionFailure::Storage(cause));
        let mut memory = if self.live == 0 {
            Memory::new(&mut self.storage)
        } else {
            Memory::resume(&mut self.storage, self.live)
        };
        let retained = value
            .retained_fields(&mut memory, &mut Unmetered)
            .map_err(|cause| match cause {
                ControlledError::Native(cause) => refusal(cause),
                ControlledError::Control(never) => match never {},
            })?;
        let released = memory
            .admitted_bytes()
            .checked_sub(retained)
            .ok_or_else(|| refusal(StorageError::SizeOverflow))?;
        // A refused refund keeps its original grant. Preserve the first native
        // cause and use that still-admitted credit before asking for growth.
        if released != 0 {
            let _ = memory.release_bytes(released);
        }
        let target = retained
            .checked_add(SharedOwned::<T>::allocation_layout::<OwnedAdmission<S>>().size())
            .ok_or_else(|| refusal(StorageError::SizeOverflow))?;
        if target > memory.admitted_bytes() {
            let growth = target - memory.admitted_bytes();
            memory.add_bytes(growth).map_err(refusal)?;
        }
        self.live = memory.admitted_bytes();
        drop(memory);
        SharedOwned::try_from_admitted(value, self.storage)
            .map_err(|_| refusal(StorageError::AllocationFailed))
    }

    pub(super) fn publish<T: RetainedFields + Send + Sync + 'static, N>(
        mut self,
        value: T,
    ) -> Result<SharedOwned<T>, N, C::Error>
    where
        S: Send + Sync + 'static,
    {
        let retained = self.measure(&value)?;
        self.shrink(retained)?;
        self.check(0)?;
        self.memory(|memory| {
            memory.add_bytes(SharedOwned::<T>::allocation_layout::<OwnedAdmission<S>>().size())
        })?;
        SharedOwned::try_from_admitted(value, self.storage).map_err(|_| {
            ControlledError::Native(FnoConstructionFailure::Storage(
                StorageError::AllocationFailed,
            ))
        })
    }
}

pub(super) trait CopyFields: Sized {
    fn copy_fields<S: Admission, C: Progress, N>(
        &self,
        context: &mut Context<S, C>,
    ) -> Result<Self, N, C::Error>;
}

impl CopyFields for String {
    fn copy_fields<S: Admission, C: Progress, N>(
        &self,
        context: &mut Context<S, C>,
    ) -> Result<Self, N, C::Error> {
        context.string(self)
    }
}
impl CopyFields for TermValue {
    fn copy_fields<S: Admission, C: Progress, N>(
        &self,
        context: &mut Context<S, C>,
    ) -> Result<Self, N, C::Error> {
        context.term(self)
    }
}
impl<T: CopyFields> CopyFields for Option<T> {
    fn copy_fields<S: Admission, C: Progress, N>(
        &self,
        context: &mut Context<S, C>,
    ) -> Result<Self, N, C::Error> {
        self.as_ref().map(|value| context.copy(value)).transpose()
    }
}
impl<T: CopyFields> CopyFields for Vec<T> {
    fn copy_fields<S: Admission, C: Progress, N>(
        &self,
        context: &mut Context<S, C>,
    ) -> Result<Self, N, C::Error> {
        let mut result = Vec::new();
        for value in self {
            context.check(1)?;
            let value = context.copy(value)?;
            context.push(&mut result, value)?;
        }
        Ok(result)
    }
}

macro_rules! copy_fields {
    ($kind:ty { $($field:ident),* $(,)? } $(copy { $($plain:ident),* $(,)? })?) => {
        impl CopyFields for $kind {
            fn copy_fields<S: Admission, C: Progress, N>(&self, context: &mut Context<S, C>)
                -> Result<Self, N, C::Error> {
                context.check(1)?;
                Ok(Self { $($field: context.copy(&self.$field)?,)* $($($plain: self.$plain,)*)? })
            }
        }
        impl RetainedFields for $kind {
            fn retained_fields<S: Admission, C: Progress>(&self, memory: &mut Memory<'_,S>, progress: &mut C)
                -> PhysicalResult<usize,C::Error> {
                let mut bytes = 0usize;
                $(bytes = add_layout(bytes, self.$field.retained_fields(memory,progress)?)?;)*
                $($(let _ = self.$plain;)*)?
                Ok(bytes)
            }
        }
    };
}
copy_fields!(FnBlank { label } copy { scope });
copy_fields!(FnOutput {
    iri,
    predicate,
    r#type,
    label,
    description
});
copy_fields!(FnParam { iri, predicate, r#type, label, description } copy { required });
copy_fields!(FnImpl {
    iri,
    format,
    see_also
});
copy_fields!(FnParamMapping {
    node,
    label,
    function_parameter,
    implementation_property
});
copy_fields!(FnReturnMapping {
    node,
    label,
    function_output,
    implementation_property
});
copy_fields!(FnMapping {
    node,
    label,
    function,
    implementation,
    parameter_mappings,
    return_mappings
});
copy_fields!(FnFunction {
    iri,
    label,
    description,
    kind_types,
    see_also,
    expects,
    output
});
copy_fields!(FnoCatalogInput {
    ontology_iri,
    document_iri,
    doc_label,
    banner,
    functions,
    params,
    implementations,
    mappings
});

pub(super) type PhysicalResult<T, E> = core::result::Result<T, ControlledError<StorageError, E>>;
pub(super) trait CompareFields {
    fn compare_fields<S: Admission, C: Progress>(
        &self,
        other: &Self,
        memory: &mut Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<core::cmp::Ordering, C::Error>;
}

impl CompareFields for str {
    fn compare_fields<S: Admission, C: Progress>(
        &self,
        other: &Self,
        _: &mut Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<core::cmp::Ordering, C::Error> {
        purrdf_lex::control::compare_bytes(self.as_bytes(), other.as_bytes(), progress)
            .map_err(ControlledError::Control)
    }
}
impl CompareFields for String {
    fn compare_fields<S: Admission, C: Progress>(
        &self,
        other: &Self,
        memory: &mut Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<core::cmp::Ordering, C::Error> {
        self.as_str()
            .compare_fields(other.as_str(), memory, progress)
    }
}
impl<T: CompareFields + ?Sized> CompareFields for &T {
    fn compare_fields<S: Admission, C: Progress>(
        &self,
        other: &Self,
        memory: &mut Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<core::cmp::Ordering, C::Error> {
        (**self).compare_fields(*other, memory, progress)
    }
}
impl CompareFields for TermValue {
    fn compare_fields<S: Admission, C: Progress>(
        &self,
        other: &Self,
        memory: &mut Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<core::cmp::Ordering, C::Error> {
        self.try_cmp_with_control(other, memory, &mut |units| progress.charge(units))
    }
}
impl<T: CompareFields> CompareFields for Option<T> {
    fn compare_fields<S: Admission, C: Progress>(
        &self,
        other: &Self,
        memory: &mut Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<core::cmp::Ordering, C::Error> {
        progress.charge(1).map_err(ControlledError::Control)?;
        match (self, other) {
            (Some(a), Some(b)) => a.compare_fields(b, memory, progress),
            (None, None) => Ok(core::cmp::Ordering::Equal),
            (None, Some(_)) => Ok(core::cmp::Ordering::Less),
            (Some(_), None) => Ok(core::cmp::Ordering::Greater),
        }
    }
}
impl<T: CompareFields> CompareFields for Vec<T> {
    fn compare_fields<S: Admission, C: Progress>(
        &self,
        other: &Self,
        memory: &mut Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<core::cmp::Ordering, C::Error> {
        for (a, b) in self.iter().zip(other) {
            progress.charge(1).map_err(ControlledError::Control)?;
            let order = a.compare_fields(b, memory, progress)?;
            if !order.is_eq() {
                return Ok(order);
            }
        }
        progress.charge(1).map_err(ControlledError::Control)?;
        Ok(self.len().cmp(&other.len()))
    }
}
macro_rules! compare_atom {
    ($($kind:ty),* $(,)?) => { $(impl CompareFields for $kind {
        fn compare_fields<S: Admission, C: Progress>(&self, other: &Self,
            _: &mut Memory<'_, S>, progress: &mut C) -> PhysicalResult<core::cmp::Ordering, C::Error> {
            progress.charge(1).map_err(ControlledError::Control)?;
            Ok(self.cmp(other))
        }
    })* };
}
compare_atom!(bool, crate::BlankScope);
macro_rules! compare_fields {
    ($kind:ty { $($field:ident),* $(,)? }) => {
        impl CompareFields for $kind {
            fn compare_fields<S: Admission, C: Progress>(&self, other: &Self,
                memory: &mut Memory<'_, S>, progress: &mut C) -> PhysicalResult<core::cmp::Ordering, C::Error> {
                $(let order = self.$field.compare_fields(&other.$field, memory, progress)?;
                  if !order.is_eq() { return Ok(order); })*
                Ok(core::cmp::Ordering::Equal)
            }
        }
    };
}
compare_fields!(FnBlank { label, scope });
compare_fields!(FnOutput {
    iri,
    predicate,
    r#type,
    label,
    description
});
compare_fields!(FnParam {
    iri,
    predicate,
    r#type,
    required,
    label,
    description
});
compare_fields!(FnImpl {
    iri,
    format,
    see_also
});
compare_fields!(FnParamMapping {
    node,
    label,
    function_parameter,
    implementation_property
});
compare_fields!(FnReturnMapping {
    node,
    label,
    function_output,
    implementation_property
});
compare_fields!(FnMapping {
    node,
    label,
    function,
    implementation,
    parameter_mappings,
    return_mappings
});
compare_fields!(FnFunction {
    iri,
    label,
    description,
    kind_types,
    see_also,
    expects,
    output
});

/// A destruction census over storage already admitted through this operation.
/// This must never be used to certify or adopt a resident allocation.
pub(super) trait RetainedFields {
    fn retained_fields<S: Admission, C: Progress>(
        &self,
        memory: &mut Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<usize, C::Error>;
}
fn add_layout<E>(a: usize, b: usize) -> PhysicalResult<usize, E> {
    a.checked_add(b)
        .ok_or(ControlledError::Native(StorageError::SizeOverflow))
}
impl RetainedFields for String {
    fn retained_fields<S: Admission, C: Progress>(
        &self,
        _: &mut Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<usize, C::Error> {
        progress.charge(1).map_err(ControlledError::Control)?;
        Ok(self.capacity())
    }
}
impl<T: ?Sized> RetainedFields for &T {
    fn retained_fields<S: Admission, C: Progress>(
        &self,
        _: &mut Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<usize, C::Error> {
        progress.charge(1).map_err(ControlledError::Control)?;
        Ok(0)
    }
}
impl<T: RetainedFields> RetainedFields for Option<T> {
    fn retained_fields<S: Admission, C: Progress>(
        &self,
        memory: &mut Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<usize, C::Error> {
        progress.charge(1).map_err(ControlledError::Control)?;
        self.as_ref()
            .map_or(Ok(0), |value| value.retained_fields(memory, progress))
    }
}
impl<T: RetainedFields> RetainedFields for Vec<T> {
    fn retained_fields<S: Admission, C: Progress>(
        &self,
        memory: &mut Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<usize, C::Error> {
        let mut bytes = core::alloc::Layout::array::<T>(self.capacity())
            .map_err(|_| ControlledError::Native(StorageError::SizeOverflow))?
            .size();
        for value in self {
            progress.charge(1).map_err(ControlledError::Control)?;
            bytes = add_layout(bytes, value.retained_fields(memory, progress)?)?;
        }
        Ok(bytes)
    }
}
impl RetainedFields for TermValue {
    fn retained_fields<S: Admission, C: Progress>(
        &self,
        memory: &mut Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<usize, C::Error> {
        let mut bytes = 0usize;
        let flow = self
            .visit_terms_with_memory(
                |node, _| {
                    if let Err(cause) = progress.charge(1) {
                        return Ok(core::ops::ControlFlow::Break(cause));
                    }
                    bytes = bytes
                        .checked_add(node.owned_heap_bytes().ok_or(StorageError::SizeOverflow)?)
                        .ok_or(StorageError::SizeOverflow)?;
                    Ok(core::ops::ControlFlow::Continue(()))
                },
                memory,
            )
            .map_err(ControlledError::Native)?;
        match flow {
            core::ops::ControlFlow::Continue(()) => Ok(bytes),
            core::ops::ControlFlow::Break(cause) => Err(ControlledError::Control(cause)),
        }
    }
}
impl<K: RetainedFields, V: RetainedFields> RetainedFields
    for purrdf_lex::allocation::OrderedTable<K, V>
{
    fn retained_fields<S: Admission, C: Progress>(
        &self,
        memory: &mut Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<usize, C::Error> {
        let mut bytes = self.storage_bytes().map_err(ControlledError::Native)?;
        for (key, value) in self.ordered() {
            progress.charge(1).map_err(ControlledError::Control)?;
            bytes = add_layout(bytes, key.retained_fields(memory, progress)?)?;
            bytes = add_layout(bytes, value.retained_fields(memory, progress)?)?;
        }
        Ok(bytes)
    }
}
impl RetainedFields for () {
    fn retained_fields<S: Admission, C: Progress>(
        &self,
        _: &mut Memory<'_, S>,
        _: &mut C,
    ) -> PhysicalResult<usize, C::Error> {
        Ok(0)
    }
}
impl RetainedFields for crate::RdfDiagnostic {
    fn retained_fields<S: Admission, C: Progress>(
        &self,
        _: &mut Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<usize, C::Error> {
        progress.charge(1).map_err(ControlledError::Control)?;
        self.retained_bytes()
            .ok_or(ControlledError::Native(StorageError::SizeOverflow))
    }
}
impl RetainedFields for FnoAdmissionReason {
    fn retained_fields<S: Admission, C: Progress>(
        &self,
        memory: &mut Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<usize, C::Error> {
        use FnoAdmissionReason as Reason;
        let sum = |a, b| add_layout(a, b);
        match self {
            Reason::Iri {
                subject,
                predicate,
                value,
                cause,
            } => {
                let a = subject.retained_fields(memory, progress)?;
                let b = predicate.retained_fields(memory, progress)?;
                let c = value.retained_fields(memory, progress)?;
                sum(sum(a, b)?, sum(c, cause.owned_text_bytes())?)
            }
            Reason::ExpectedLiteral {
                subject,
                predicate,
                value,
            } => {
                let a = subject.retained_fields(memory, progress)?;
                let b = predicate.retained_fields(memory, progress)?;
                sum(sum(a, b)?, value.retained_fields(memory, progress)?)
            }
            Reason::Literal {
                subject,
                predicate,
                cause,
            } => {
                let a = subject.retained_fields(memory, progress)?;
                let b = predicate.retained_fields(memory, progress)?;
                sum(sum(a, b)?, cause.retained_fields(memory, progress)?)
            }
            Reason::ConflictingNode { subject } => subject.retained_fields(memory, progress),
            Reason::Reference {
                subject,
                predicate,
                target,
            } => {
                let a = subject.retained_fields(memory, progress)?;
                let b = predicate.retained_fields(memory, progress)?;
                sum(sum(a, b)?, target.retained_fields(memory, progress)?)
            }
        }
    }
}

/// A source identifier is a native Copy atom; this wrapper keeps its local
/// comparison admission distinct from owned semantic term comparison.
#[derive(Clone, Copy)]
pub(super) struct OrdinalKey<I: crate::ViewTermId>(pub(super) I);
impl<I: crate::ViewTermId> CompareFields for OrdinalKey<I> {
    fn compare_fields<S: Admission, C: Progress>(
        &self,
        other: &Self,
        _: &mut Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<core::cmp::Ordering, C::Error> {
        let extent = size_of::<I>()
            .checked_mul(2)
            .and_then(|bytes| bytes.checked_add(1))
            .and_then(|bytes| u64::try_from(bytes).ok())
            .ok_or(ControlledError::Native(StorageError::SizeOverflow))?;
        progress.charge(extent).map_err(ControlledError::Control)?;
        Ok(self.0.cmp(&other.0))
    }
}
impl<I: crate::ViewTermId> RetainedFields for OrdinalKey<I> {
    fn retained_fields<S: Admission, C: Progress>(
        &self,
        _: &mut Memory<'_, S>,
        _: &mut C,
    ) -> PhysicalResult<usize, C::Error> {
        Ok(0)
    }
}
impl RetainedFields for usize {
    fn retained_fields<S: Admission, C: Progress>(
        &self,
        _: &mut Memory<'_, S>,
        _: &mut C,
    ) -> PhysicalResult<usize, C::Error> {
        Ok(0)
    }
}
impl RetainedFields for super::FnoRejectedCatalog {
    fn retained_fields<S: Admission, C: Progress>(
        &self,
        memory: &mut Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<usize, C::Error> {
        let input = self.input.retained_fields(memory, progress)?;
        add_layout(input, self.reason.retained_fields(memory, progress)?)
    }
}
impl RetainedFields for super::FnoReadError {
    fn retained_fields<S: Admission, C: Progress>(
        &self,
        memory: &mut Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<usize, C::Error> {
        use super::FnoReadError as Error;
        progress.charge(1).map_err(ControlledError::Control)?;
        match self {
            Error::Document {
                ontology_iri,
                candidates,
            } => {
                let selector = ontology_iri.retained_fields(memory, progress)?;
                add_layout(selector, candidates.retained_fields(memory, progress)?)
            }
            Error::Field {
                subject,
                predicate,
                objects,
                expected,
            } => {
                let subject = subject.retained_fields(memory, progress)?;
                let predicate = predicate.retained_fields(memory, progress)?;
                let objects = objects.retained_fields(memory, progress)?;
                let expected = match expected {
                    super::FnoFieldKind::Role { class } => {
                        class.retained_fields(memory, progress)?
                    }
                    _ => 0,
                };
                add_layout(
                    add_layout(subject, predicate)?,
                    add_layout(objects, expected)?,
                )
            }
            Error::List {
                subject,
                predicate,
                cause,
            } => {
                let subject = subject.retained_fields(memory, progress)?;
                let predicate = predicate.retained_fields(memory, progress)?;
                let node = cause.node.retained_fields(memory, progress)?;
                let members = cause.members.retained_fields(memory, progress)?;
                add_layout(add_layout(subject, predicate)?, add_layout(node, members)?)
            }
            Error::Returns {
                subject,
                predicate,
                head,
                members,
            } => {
                let subject = subject.retained_fields(memory, progress)?;
                let predicate = predicate.retained_fields(memory, progress)?;
                let head = head.retained_fields(memory, progress)?;
                let members = members.retained_fields(memory, progress)?;
                add_layout(add_layout(subject, predicate)?, add_layout(head, members)?)
            }
            Error::Statement { row, .. } => {
                let s = row.s.retained_fields(memory, progress)?;
                let p = row.p.retained_fields(memory, progress)?;
                let o = row.o.retained_fields(memory, progress)?;
                let g = row.g.retained_fields(memory, progress)?;
                add_layout(add_layout(s, p)?, add_layout(o, g)?)
            }
            Error::Graph { graph } => graph.retained_fields(memory, progress),
            Error::Admission(cause) => cause.retained_fields(memory, progress),
        }
    }
}
