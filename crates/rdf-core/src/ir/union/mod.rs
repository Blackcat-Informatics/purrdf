// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Static, read-through unions of arbitrary backend views.

mod index;
mod storage;

use self::index::{Index, capture};
use self::storage::{Account, Grant, Reservation};
use crate::hash::{hash_of, reserve_table_with_memory, table::HashTable};
use crate::{
    BlankScope, DatasetView, GraphMatch, NoopReservation, QuadIds, RdfStoreCapabilities,
    ScopeBinding, TermGuard, TermRef, TermValue, ViewLimits, ViewStats, ViewTermId,
    WorkspaceReservation,
};
use purrdf_lex::allocation::{Admission, Memory, OrderedTable, StorageError};
use std::borrow::Cow;
use std::fmt;
use std::sync::{Mutex, atomic::AtomicUsize};

/// The original failing source, or a native union construction/read refusal.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum UnionError<L, R> {
    /// The left source's unchanged failure.
    Left(usize, L),
    /// The right source's unchanged failure.
    Right(usize, R),
    /// A stated view ceiling was exceeded before growth/publication.
    Limit {
        /// The retained dimension.
        dimension: &'static str,
        /// The required quantity.
        actual: usize,
        /// The caller's inclusive ceiling.
        limit: usize,
    },
    /// Insufficient native/shadow stack to enter another source read scope.
    Stack {
        /// Zero-based source position refused before entering its wrapper.
        source: usize,
    },
    /// Original checked layout/admission/allocation refusal.
    Storage(StorageError),
    /// A supplied source violated its term/reference contract.
    Invariant(&'static str),
}

impl<L: fmt::Display, R: fmt::Display> fmt::Display for UnionError<L, R> {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Left(source, error) => {
                write!(out, "union source {source} (left backend): {error}")
            }
            Self::Right(source, error) => {
                write!(out, "union source {source} (right backend): {error}")
            }
            Self::Limit {
                dimension,
                actual,
                limit,
            } => write!(out, "union requires {actual} {dimension}, limit is {limit}"),
            Self::Stack { source } => write!(
                out,
                "insufficient stack for union source {source} checked read"
            ),
            Self::Storage(error) => error.fmt(out),
            Self::Invariant(reason) => write!(out, "invalid union source: {reason}"),
        }
    }
}
impl<L: std::error::Error + 'static, R: std::error::Error + 'static> std::error::Error
    for UnionError<L, R>
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Left(_, error) => Some(error),
            Self::Right(_, error) => Some(error),
            Self::Storage(error) => Some(error),
            _ => None,
        }
    }
}
purrdf_lex::variant_from!(impl <L, R> UnionError<L, R> { Storage(StorageError) });

impl<L: purrdf_hash::error::StructuredError, R: purrdf_hash::error::StructuredError>
    purrdf_hash::error::StructuredError for UnionError<L, R>
{
    purrdf_hash::error_visit! {
        Left(source, cause) => "union-left-source" { "source" => Unsigned(*source as u128), "cause" => Cause(cause) },
        Right(source, cause) => "union-right-source" { "source" => Unsigned(*source as u128), "cause" => Cause(cause) },
        Limit { dimension, actual, limit } => "union-view-limit" {
            "dimension" => Code(*dimension), "actual" => Unsigned(*actual as u128), "limit" => Unsigned(*limit as u128),
        },
        Stack { source } => "union-read-stack" { "source" => Unsigned(*source as u128) },
        Storage(cause) => "union-storage" { "cause" => Cause(cause) },
        Invariant(reason) => "union-source-invariant" { "reason" => Code(*reason) },
    }
}

/// A canonical handle local to exactly one union. Source IDs are never cast.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnionTermId(u64);
impl ViewTermId for UnionTermId {
    type JoinKeyAtom = u128;
    fn encode(self) -> u128 {
        u128::from(self.0)
    }
    fn encode_computed(index: u32) -> u128 {
        (1_u128 << 127) | u128::from(index)
    }
}

/// A borrowed source in a runtime list, with an explicit blank-scope law.
///
/// The two variants preserve static dispatch and original backend error types.
/// A homogeneous list can use `UnionSource::<Host, Host>::Left(&host, binding)`.
/// More backend types can be represented by caller-owned typed sums or nested unions.
#[derive(Debug)]
pub enum UnionSource<'a, A: DatasetView, B: DatasetView> {
    /// A source using the first backend type.
    Left(&'a A, ScopeBinding),
    /// A source using the second backend type.
    Right(&'a B, ScopeBinding),
}
impl<A: DatasetView, B: DatasetView> Copy for UnionSource<'_, A, B> {}
impl<A: DatasetView, B: DatasetView> Clone for UnionSource<'_, A, B> {
    fn clone(&self) -> Self {
        *self
    }
}

#[derive(Clone, Copy, Debug)]
enum Local<L, R> {
    Left(L),
    Right(R),
}
#[derive(Clone, Copy, Debug)]
struct Origin<L, R> {
    source: usize,
    local: Local<L, R>,
}

enum Entry<'a, A: DatasetView, B: DatasetView> {
    Left(&'a A, ScopeBinding, Index<A::Id>),
    Right(&'a B, ScopeBinding, Index<B::Id>),
}
impl<A: DatasetView, B: DatasetView> Entry<'_, A, B> {
    fn scopes(&self) -> (&[(BlankScope, BlankScope)], ScopeBinding) {
        match self {
            Self::Left(_, b, i) => (&i.scopes, *b),
            Self::Right(_, b, i) => (&i.scopes, *b),
        }
    }
    fn scopes_mut(&mut self) -> (&mut [(BlankScope, BlankScope)], ScopeBinding) {
        match self {
            Self::Left(_, b, i) => (&mut i.scopes, *b),
            Self::Right(_, b, i) => (&mut i.scopes, *b),
        }
    }
    fn len(&self) -> usize {
        match self {
            Self::Left(_, _, i) => i.ids.len(),
            Self::Right(_, _, i) => i.ids.len(),
        }
    }
    fn state(&self, at: usize) -> u8 {
        match self {
            Self::Left(_, _, i) => i.states[at],
            Self::Right(_, _, i) => i.states[at],
        }
    }
    fn set_state(&mut self, at: usize, value: u8) {
        match self {
            Self::Left(_, _, i) => i.states[at] = value,
            Self::Right(_, _, i) => i.states[at] = value,
        }
    }
}

// Only the continuation is dynamically dispatched; every backend, ID, pin and
// original error remains a concrete static type. This prevents recursive generic
// closure expansion while retaining every source's actual checked-read scope.
fn checked_sources<A: DatasetView, B: DatasetView>(
    sources: &[UnionSource<'_, A, B>],
    at: usize,
    read: &mut dyn FnMut() -> Result<(), UnionError<A::ReadError, B::ReadError>>,
) -> Result<(), UnionError<A::ReadError, B::ReadError>> {
    let Some(source) = sources.get(at) else {
        return read();
    };
    if purrdf_stack::is_low() {
        return Err(UnionError::Stack { source: at });
    }
    let mut next = || checked_sources(sources, at + 1, read);
    match source {
        UnionSource::Left(view, _) => view
            .checked_read(|_| next())
            .map_err(|e| UnionError::Left(at, e))?,
        UnionSource::Right(view, _) => view
            .checked_read(|_| next())
            .map_err(|e| UnionError::Right(at, e))?,
    }
}

enum Pin<'a, A: DatasetView + 'a, B: DatasetView + 'a> {
    Left(A::TermGuard<'a>, &'a Index<A::Id>, usize),
    Right(B::TermGuard<'a>, &'a Index<B::Id>, usize),
}

/// An original backend pin plus borrowed union identity/rebinding metadata.
pub struct UnionTermGuard<'a, A: DatasetView, B: DatasetView> {
    pin: Pin<'a, A, B>,
}

impl<A: DatasetView, B: DatasetView> fmt::Debug for UnionTermGuard<'_, A, B> {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.debug_struct("UnionTermGuard")
            .field("term", &self.term())
            .finish()
    }
}

fn mapped<'a, I: ViewTermId, L, R>(
    term: TermRef<'a, I>,
    index: &'a Index<I>,
    at: usize,
) -> Result<TermRef<'a, UnionTermId>, UnionError<L, R>> {
    let alias = |id| {
        index.alias(id).ok_or(UnionError::Invariant(
            "reference is outside the reached term registry",
        ))
    };
    Ok(match term {
        TermRef::Iri(iri) => TermRef::Iri(iri),
        TermRef::Blank { label, scope } => TermRef::Blank {
            label,
            scope: index.scope(scope).ok_or(UnionError::Invariant(
                "blank scope is outside the captured source",
            ))?,
        },
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => TermRef::Literal {
            lexical: index.rebound[at].as_deref().unwrap_or(lexical),
            datatype: alias(datatype)?,
            language,
            direction,
        },
        TermRef::Triple { s, p, o } => TermRef::Triple {
            s: alias(s)?,
            p: alias(p)?,
            o: alias(o)?,
        },
    })
}

impl<A: DatasetView, B: DatasetView> TermGuard<UnionTermId> for UnionTermGuard<'_, A, B> {
    fn term(&self) -> TermRef<'_, UnionTermId> {
        // resolve() checked these exact immutable references before publishing
        // the pin. No new read or allocation happens between it and this borrow.
        match &self.pin {
            Pin::Left(guard, index, at) => mapped::<_, (), ()>(guard.term(), index, *at),
            Pin::Right(guard, index, at) => mapped::<_, (), ()>(guard.term(), index, *at),
        }
        .expect("validated immutable union term pin")
    }
}

struct Parts<'a, A: DatasetView, B: DatasetView> {
    sources: Vec<UnionSource<'a, A, B>>,
    entries: Vec<Entry<'a, A, B>>,
    representatives: Vec<Origin<A::Id, B::Id>>,
    keys: HashTable<(u64, UnionTermId)>,
    graphs: Vec<UnionTermId>,
    stats: ViewStats,
}

/// A borrowed set union with statically dispatched, potentially different backends.
///
/// Only reached term handles, aliases, scope maps and rewritten composite-literal
/// text are retained. Ordinary, reifier and annotation rows stay in their sources.
/// Source scoped reservations outlive all metadata. The inferred reservation
/// parameters need not be named by callers. Keep sources in local bindings; use
/// `from_sources` for runtime lists and `union` for further typed composition.
pub struct UnionDatasetView<
    'a,
    A: DatasetView,
    B: DatasetView,
    LA: WorkspaceReservation<Error = A::ReadError> = NoopReservation<<A as DatasetView>::ReadError>,
    RB: WorkspaceReservation<Error = B::ReadError> = NoopReservation<<B as DatasetView>::ReadError>,
> {
    data: Parts<'a, A, B>,
    live: AtomicUsize,
    failure: Mutex<Option<UnionError<A::ReadError, B::ReadError>>>,
    // Payloads above die before their original source grants.
    account: Account<LA, RB>,
}

impl<
    A: DatasetView,
    B: DatasetView,
    LA: WorkspaceReservation<Error = A::ReadError>,
    RB: WorkspaceReservation<Error = B::ReadError>,
> fmt::Debug for UnionDatasetView<'_, A, B, LA, RB>
{
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.debug_struct("UnionDatasetView")
            .field("stats", &self.data.stats)
            .finish_non_exhaustive()
    }
}

fn check_stats<L, R>(limits: ViewLimits, stats: &ViewStats) -> Result<(), UnionError<L, R>> {
    for (dimension, actual, limit) in [
        ("sources", stats.retained_sources, limits.max_sources),
        ("terms", stats.retained_terms, limits.max_terms),
        ("rows", stats.retained_rows, limits.max_rows),
        (
            "payload bytes",
            stats.retained_payload_bytes,
            limits.max_payload_bytes,
        ),
    ] {
        if actual > limit {
            return Err(UnionError::Limit {
                dimension,
                actual,
                limit,
            });
        }
    }
    Ok(())
}

fn assign_scopes<A: DatasetView, B: DatasetView, S: Admission + ?Sized>(
    entries: &mut [Entry<'_, A, B>],
    memory: &mut Memory<'_, S>,
) -> Result<(), UnionError<A::ReadError, B::ReadError>> {
    let mut occupied = OrderedTable::default();
    for entry in &*entries {
        let (scopes, binding) = entry.scopes();
        if binding == ScopeBinding::Shared {
            for &(scope, _) in scopes {
                occupied.insert(scope, (), memory)?;
            }
        }
    }
    let mut next = 0_u32;
    for entry in entries {
        let (scopes, binding) = entry.scopes_mut();
        if binding == ScopeBinding::Independent {
            for (_, target) in scopes {
                while occupied.get(&BlankScope(next)).is_some() {
                    next = next
                        .checked_add(1)
                        .ok_or(UnionError::Invariant("blank scope space exhausted"))?;
                }
                *target = BlankScope(next);
                occupied.insert(*target, (), memory)?;
            }
        }
    }
    occupied.release(memory)?;
    Ok(())
}

fn rewrite<D: DatasetView, L, R, S: Admission + ?Sized>(
    view: &D,
    index: &mut Index<D::Id>,
    error: impl Fn(D::ReadError) -> UnionError<L, R> + Copy,
    memory: &mut Memory<'_, S>,
) -> Result<(), UnionError<L, R>> {
    use crate::blank_label::{
        LabelAlphabet, decode_blank_label_with_memory, encode_blank_label_with_memory,
    };
    if index.scopes.iter().all(|(a, b)| a == b) {
        return Ok(());
    }
    for at in 0..index.ids.len() {
        let pin = view.resolve(index.ids[at]).map_err(error)?;
        let TermRef::Literal {
            lexical, datatype, ..
        } = pin.term()
        else {
            continue;
        };
        let datatype_pin = view.resolve(datatype).map_err(error)?;
        let TermRef::Iri(datatype) = datatype_pin.term() else {
            return Err(UnionError::Invariant("literal datatype is not an IRI"));
        };
        let mut missing = false;
        let rewritten = crate::cdt_blank::rewrite_cdt_blank_terms_with_memory(
            lexical,
            datatype,
            &mut |token, memory| {
                let (label, original) =
                    decode_blank_label_with_memory(token, LabelAlphabet::BlankNodeLabel, memory)?;
                let Some(target) = index.scope(original) else {
                    missing = true;
                    return Err(StorageError::AdmissionFailed);
                };
                let result = if target == original {
                    None
                } else {
                    let encoded = encode_blank_label_with_memory(
                        &label,
                        target,
                        LabelAlphabet::BlankNodeLabel,
                        memory,
                    )?;
                    let result = memory.format(&format_args!("_:{encoded}"))?;
                    if let Cow::Owned(encoded) = encoded {
                        memory.release_string(encoded)?;
                    }
                    Some(result)
                };
                if let Cow::Owned(label) = label {
                    memory.release_string(label)?;
                }
                Ok(result)
            },
            memory,
        );
        if missing {
            return Err(UnionError::Invariant(
                "composite blank scope changed during construction",
            ));
        }
        if let Cow::Owned(text) = rewritten? {
            index.rebound[at] = Some(text);
        }
    }
    Ok(())
}

// Keep the opaque original guard types inferred without repeating the complete
// view/error relationship in each constructor's return signature.
type BuildResult<'a, A, B, LA, RB> = Result<
    UnionDatasetView<'a, A, B, LA, RB>,
    UnionError<<A as DatasetView>::ReadError, <B as DatasetView>::ReadError>,
>;
type GrantResult<L, R, LE, RE> = Result<Grant<L, R>, UnionError<LE, RE>>;

impl<'a, A: DatasetView, B: DatasetView> UnionDatasetView<'a, A, B> {
    /// Borrow two views through the same runtime-list engine as `from_sources`.
    /// # Errors
    /// Preserves original source failures, view ceilings and allocation refusals.
    pub fn new(
        left: &'a A,
        right: &'a B,
        bindings: (ScopeBinding, ScopeBinding),
        limits: ViewLimits,
    ) -> BuildResult<
        'a,
        A,
        B,
        impl WorkspaceReservation<Error = A::ReadError> + 'a + use<'a, A, B>,
        impl WorkspaceReservation<Error = B::ReadError> + 'a + use<'a, A, B>,
    > {
        Self::from_sources(
            &[
                UnionSource::Left(left, bindings.0),
                UnionSource::Right(right, bindings.1),
            ],
            limits,
        )
    }

    /// Borrow a runtime list of two or more sources without materializing rows.
    ///
    /// The list itself is copied into admitted storage; backend references remain
    /// borrowed. Shared blank scopes are reserved before assigning independent
    /// scopes. Reached vocabulary includes nested triples and composite blanks;
    /// unused dictionary entries are excluded. Retained limits include inherited
    /// source dimensions and metadata, all source handles, indexes and guards.
    ///
    /// For a homogeneous list, annotate it as `Vec<UnionSource<'_, Host, Host>>`.
    /// Keep hosts in local bindings until the union is dropped. Source errors carry
    /// their zero-based position and unchanged original cause.
    ///
    /// ```
    /// use purrdf_core::{DatasetView, ScopeBinding, UnionDatasetView, UnionSource, ViewLimits};
    /// fn read_together<A: DatasetView, B: DatasetView>(external: &A, resident: &B) {
    ///     let sources = vec![
    ///         UnionSource::Left(external, ScopeBinding::Independent),
    ///         UnionSource::Right(resident, ScopeBinding::Independent),
    ///     ];
    ///     let union = UnionDatasetView::from_sources(&sources, ViewLimits::default()).unwrap();
    ///     // Both source bindings outlive this read; their blank identities stay apart.
    ///     let _ = union.quads().count();
    /// }
    /// ```
    /// # Errors
    /// Refuses fewer than two sources, original checked-read/resource failures,
    /// incoherent references, view ceilings and native allocation failures.
    pub fn from_sources(
        sources: &[UnionSource<'a, A, B>],
        limits: ViewLimits,
    ) -> BuildResult<
        'a,
        A,
        B,
        impl WorkspaceReservation<Error = A::ReadError> + 'a + use<'a, A, B>,
        impl WorkspaceReservation<Error = B::ReadError> + 'a + use<'a, A, B>,
    > {
        if sources.len() < 2 {
            return Err(UnionError::Invariant(
                "a union requires at least two sources",
            ));
        }
        let mut stats = ViewStats::default();
        for source in sources {
            let retained = match source {
                UnionSource::Left(v, _) => v.retained_view_stats(),
                UnionSource::Right(v, _) => v.retained_view_stats(),
            };
            stats.retained_sources = stats
                .retained_sources
                .checked_add(retained.map_or(1, |s| s.retained_sources))
                .ok_or(UnionError::Storage(StorageError::SizeOverflow))?;
            stats.auxiliary_bytes = stats
                .auxiliary_bytes
                .checked_add(retained.map_or(0, |s| s.auxiliary_bytes))
                .ok_or(UnionError::Storage(StorageError::SizeOverflow))?;
        }
        check_stats(limits, &stats)?;
        if stats.auxiliary_bytes > limits.max_auxiliary_bytes {
            return Err(UnionError::Limit {
                dimension: "auxiliary bytes",
                actual: stats.auxiliary_bytes,
                limit: limits.max_auxiliary_bytes,
            });
        }
        let inherited = stats.auxiliary_bytes;
        let mut result = None;
        checked_sources(sources, 0, &mut || {
            let bootstrap = reserve_source(sources[0], 0)?;
            let mut account = Account {
                grants: Vec::new(),
                bootstrap,
                live: 0,
                limit: limits.max_auxiliary_bytes - inherited,
                failure: None,
            };
            let built = (|| {
                // The first original guard admits control storage before allocation
                // and is dropped after the vector and all metadata it admitted.
                let mut grants = Vec::new();
                Memory::new(&mut account).reserve(&mut grants, sources.len() - 1)?;
                for (at, source) in sources.iter().copied().enumerate().skip(1) {
                    let mut grant = reserve_source(source, at)?;
                    grant.resize(account.live as u64)?;
                    grants.push(grant);
                }
                account.grants = grants;
                let admitted = account.live;
                let mut memory = Memory::resume(&mut account, admitted);
                let mut handles = Vec::new();
                memory.reserve(&mut handles, sources.len())?;
                handles.extend_from_slice(sources);
                let mut entries = Vec::new();
                memory.reserve(&mut entries, sources.len())?;
                for (at, source) in sources.iter().copied().enumerate() {
                    let before = stats;
                    let (entry, retained) = match source {
                        UnionSource::Left(v, b) => (
                            Entry::Left(
                                v,
                                b,
                                capture(
                                    v,
                                    |e| UnionError::Left(at, e),
                                    &mut stats,
                                    limits,
                                    &mut memory,
                                )?,
                            ),
                            v.retained_view_stats(),
                        ),
                        UnionSource::Right(v, b) => (
                            Entry::Right(
                                v,
                                b,
                                capture(
                                    v,
                                    |e| UnionError::Right(at, e),
                                    &mut stats,
                                    limits,
                                    &mut memory,
                                )?,
                            ),
                            v.retained_view_stats(),
                        ),
                    };
                    if let Some(retained) = retained {
                        stats.retained_terms = before
                            .retained_terms
                            .checked_add(retained.retained_terms)
                            .ok_or(UnionError::Storage(StorageError::SizeOverflow))?;
                        stats.retained_rows = before
                            .retained_rows
                            .checked_add(retained.retained_rows)
                            .ok_or(UnionError::Storage(StorageError::SizeOverflow))?;
                        stats.retained_payload_bytes = before
                            .retained_payload_bytes
                            .checked_add(retained.retained_payload_bytes)
                            .ok_or(UnionError::Storage(StorageError::SizeOverflow))?;
                        check_stats(limits, &stats)?;
                    }
                    entries.push(entry);
                }
                assign_scopes(&mut entries, &mut memory)?;
                for (at, entry) in entries.iter_mut().enumerate() {
                    match entry {
                        Entry::Left(v, _, i) => {
                            rewrite(*v, i, |e| UnionError::Left(at, e), &mut memory)?;
                        }
                        Entry::Right(v, _, i) => {
                            rewrite(*v, i, |e| UnionError::Right(at, e), &mut memory)?;
                        }
                    }
                }
                let mut parts = Parts {
                    sources: handles,
                    entries,
                    representatives: Vec::new(),
                    keys: HashTable::new(),
                    graphs: Vec::new(),
                    stats,
                };
                parts.intern_all(&mut memory)?;
                for entry in &parts.entries {
                    match entry {
                        Entry::Left(v, _, i) => {
                            for id in v.named_graphs() {
                                memory.push(
                                    &mut parts.graphs,
                                    i.alias(id)
                                        .ok_or(UnionError::Invariant("uncaptured source graph"))?,
                                )?;
                            }
                        }
                        Entry::Right(v, _, i) => {
                            for id in v.named_graphs() {
                                memory.push(
                                    &mut parts.graphs,
                                    i.alias(id)
                                        .ok_or(UnionError::Invariant("uncaptured source graph"))?,
                                )?;
                            }
                        }
                    }
                }
                parts.graphs.sort_unstable();
                parts.graphs.dedup();
                Ok(parts)
            })();
            let mut data = built.map_err(|e| match e {
                UnionError::Storage(e) => account.restore(e),
                e => e,
            })?;
            let live = inherited
                .checked_add(account.live)
                .ok_or(UnionError::Storage(StorageError::SizeOverflow))?;
            data.stats.auxiliary_bytes = live;
            result = Some(UnionDatasetView {
                data,
                live: AtomicUsize::new(live),
                failure: Mutex::new(None),
                account,
            });
            Ok(())
        })?;
        result.ok_or(UnionError::Invariant(
            "checked source did not invoke its read callback",
        ))
    }
}

fn reserve_source<A: DatasetView, B: DatasetView>(
    source: UnionSource<'_, A, B>,
    at: usize,
) -> GrantResult<
    impl WorkspaceReservation<Error = A::ReadError> + '_,
    impl WorkspaceReservation<Error = B::ReadError> + '_,
    A::ReadError,
    B::ReadError,
> {
    match source {
        UnionSource::Left(v, _) => v
            .reserve_workspace(0)
            .map(|g| Grant::Left(at, g))
            .map_err(|e| UnionError::Left(at, e)),
        UnionSource::Right(v, _) => v
            .reserve_workspace(0)
            .map(|g| Grant::Right(at, g))
            .map_err(|e| UnionError::Right(at, e)),
    }
}

#[derive(Clone, Copy)]
enum Table {
    Ordinary,
    Reifier,
    Annotation,
}

enum Rows<A, B, C> {
    Ordinary(A),
    Reifier(B),
    Annotation(C),
}
impl<T, A: Iterator<Item = T>, B: Iterator<Item = T>, C: Iterator<Item = T>> Iterator
    for Rows<A, B, C>
{
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match self {
            Self::Ordinary(rows) => rows.next(),
            Self::Reifier(rows) => rows.next(),
            Self::Annotation(rows) => rows.next(),
        }
    }
}
fn source_rows<D: DatasetView>(
    source: &D,
    table: Table,
) -> impl Iterator<Item = QuadIds<D::Id>> + '_ {
    match table {
        Table::Ordinary => Rows::Ordinary(source.quads()),
        Table::Reifier => Rows::Reifier(source.reifier_quads()),
        Table::Annotation => Rows::Annotation(source.annotation_quads()),
    }
}

fn map_row<I: ViewTermId, L, R>(
    index: &Index<I>,
    row: QuadIds<I>,
) -> Result<QuadIds<UnionTermId>, UnionError<L, R>> {
    let get = |id| {
        index
            .alias(id)
            .ok_or(UnionError::Invariant("row references an uncaptured term"))
    };
    Ok(QuadIds {
        s: get(row.s)?,
        p: get(row.p)?,
        o: get(row.o)?,
        g: row.g.map(get).transpose()?,
    })
}
fn local_row<I: ViewTermId>(index: &Index<I>, row: QuadIds<UnionTermId>) -> Option<QuadIds<I>> {
    Some(QuadIds {
        s: index.local(row.s)?,
        p: index.local(row.p)?,
        o: index.local(row.o)?,
        g: match row.g {
            Some(id) => Some(index.local(id)?),
            None => None,
        },
    })
}

fn contains<D: DatasetView>(
    source: &D,
    row: QuadIds<D::Id>,
    table: Table,
) -> Result<bool, D::ReadError> {
    let graph = row.g.map_or(GraphMatch::Default, GraphMatch::Named);
    source.checked_read(|source| match table {
        Table::Ordinary => source
            .quads_for_pattern(Some(row.s), Some(row.p), Some(row.o), graph)
            .next()
            .is_some(),
        Table::Reifier => source.reifier_quads_of(row.s).any(|q| q == row),
        Table::Annotation => source
            .annotation_quads_for_pattern(Some(row.p), Some(row.o), graph)
            .any(|q| q == row),
    })
}

struct Scratch<R: WorkspaceReservation> {
    reservation: R,
    failure: Option<R::Error>,
}
impl<R: WorkspaceReservation> Admission for Scratch<R> {
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        if self.failure.is_some() {
            return Err(StorageError::AdmissionFailed);
        }
        self.reservation
            .resize(u64::try_from(bytes).map_err(|_| StorageError::SizeOverflow)?)
            .map_err(|error| {
                self.failure = Some(error);
                StorageError::AdmissionFailed
            })
    }
}

impl<
    A: DatasetView,
    B: DatasetView,
    LA: WorkspaceReservation<Error = A::ReadError>,
    RB: WorkspaceReservation<Error = B::ReadError>,
> UnionDatasetView<'_, A, B, LA, RB>
{
    fn latch(
        &self,
        error: UnionError<A::ReadError, B::ReadError>,
    ) -> UnionError<A::ReadError, B::ReadError> {
        let mut failure = self
            .failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        failure.get_or_insert(error).clone()
    }

    fn record<T>(
        &self,
        result: Result<T, UnionError<A::ReadError, B::ReadError>>,
    ) -> Result<T, UnionError<A::ReadError, B::ReadError>> {
        result.map_err(|error| self.latch(error))
    }

    /// Retained source dimensions and exact admitted union metadata capacity.
    #[must_use]
    pub fn stats(&self) -> ViewStats {
        self.data.stats
    }

    /// Add another arbitrary source while retaining this union's established
    /// identity space. The new source explicitly shares it or is standardized apart.
    /// Keep this union in a local binding until the returned view is dropped.
    /// # Errors
    /// Returns the same original-source, scope, limit and physical refusals as `new`.
    pub fn union<'b, C: DatasetView>(
        &'b self,
        source: &'b C,
        binding: ScopeBinding,
        limits: ViewLimits,
    ) -> BuildResult<
        'b,
        Self,
        C,
        impl WorkspaceReservation<Error = <Self as DatasetView>::ReadError> + 'b,
        impl WorkspaceReservation<Error = C::ReadError> + 'b,
    > {
        UnionDatasetView::new(self, source, (ScopeBinding::Shared, binding), limits)
    }

    fn has(
        &self,
        row: QuadIds<UnionTermId>,
        table: Table,
        source: usize,
    ) -> Result<bool, UnionError<A::ReadError, B::ReadError>> {
        match &self.data.entries[source] {
            Entry::Left(v, _, i) => local_row(i, row).map_or(Ok(false), |row| {
                contains(*v, row, table).map_err(|e| UnionError::Left(source, e))
            }),
            Entry::Right(v, _, i) => local_row(i, row).map_or(Ok(false), |row| {
                contains(*v, row, table).map_err(|e| UnionError::Right(source, e))
            }),
        }
    }

    fn rows(&self, table: Table) -> impl Iterator<Item = QuadIds<UnionTermId>> + '_ {
        let mut source = 0;
        let mut current = None;
        let mut complete = false;
        std::iter::from_fn(move || {
            while !complete {
                if let Some(error) = self.read_error() {
                    self.latch(error);
                    complete = true;
                    return None;
                }
                if current.is_none() {
                    let Some(entry) = self.data.entries.get(source) else {
                        complete = true;
                        return None;
                    };
                    current = Some(match entry {
                        Entry::Left(v, _, i) => {
                            Either::Left(source_rows(*v, table).map(move |row| map_row(i, row)))
                        }
                        Entry::Right(v, _, i) => {
                            Either::Right(source_rows(*v, table).map(move |row| map_row(i, row)))
                        }
                    });
                }
                let row = current.as_mut().expect("active source iterator").next();
                if let Some(error) = self.read_error() {
                    self.latch(error);
                    complete = true;
                    return None;
                }
                let Some(row) = row else {
                    current = None;
                    source += 1;
                    continue;
                };
                let result = (|| {
                    if let Some(error) = self.read_error() {
                        return Err(error);
                    }
                    let row = row?;
                    for prior in 0..source {
                        if self.has(row, table, prior)? {
                            return Ok(None);
                        }
                    }
                    if matches!(table, Table::Ordinary) {
                        for virtual_table in [Table::Reifier, Table::Annotation] {
                            for at in 0..self.data.entries.len() {
                                if self.has(row, virtual_table, at)? {
                                    return Ok(None);
                                }
                            }
                        }
                    }
                    Ok(Some(row))
                })();
                match self.record(result) {
                    Ok(Some(row)) => return Some(row),
                    Ok(None) => {}
                    Err(_) => {
                        complete = true;
                        return None;
                    }
                }
            }
            None
        })
    }
}

impl<
    A: DatasetView,
    B: DatasetView,
    LA: WorkspaceReservation<Error = A::ReadError>,
    RB: WorkspaceReservation<Error = B::ReadError>,
> DatasetView for UnionDatasetView<'_, A, B, LA, RB>
{
    type Id = UnionTermId;
    type ReadError = UnionError<A::ReadError, B::ReadError>;
    type TermGuard<'a>
        = UnionTermGuard<'a, A, B>
    where
        Self: 'a;
    type ProbePlan = ();

    fn retained_view_stats(&self) -> Option<ViewStats> {
        Some(self.stats())
    }

    fn read_error(&self) -> Option<Self::ReadError> {
        let stored = self
            .failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        stored.or_else(|| {
            self.data
                .sources
                .iter()
                .enumerate()
                .find_map(|(at, source)| match source {
                    UnionSource::Left(v, _) => v.read_error().map(|e| UnionError::Left(at, e)),
                    UnionSource::Right(v, _) => v.read_error().map(|e| UnionError::Right(at, e)),
                })
        })
    }

    fn checked_read<T>(&self, read: impl FnOnce(&Self) -> T) -> Result<T, Self::ReadError> {
        if let Some(error) = self.read_error() {
            return Err(self.latch(error));
        }
        let mut read = Some(read);
        let mut value = None;
        let result = checked_sources(&self.data.sources, 0, &mut || {
            value = Some(read.take().ok_or(UnionError::Invariant(
                "source repeated checked-read callback",
            ))?(self));
            Ok(())
        });
        match result {
            Err(error) => Err(self.latch(error)),
            Ok(()) => match self.read_error() {
                Some(error) => Err(self.latch(error)),
                None => value.ok_or_else(|| {
                    self.latch(UnionError::Invariant(
                        "source omitted checked-read callback",
                    ))
                }),
            },
        }
    }

    fn resolve(&self, id: Self::Id) -> Result<Self::TermGuard<'_>, Self::ReadError> {
        self.record(self.checked_read(|_| self.data.resolve(id))?)
    }

    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<Self::Id>, Self::ReadError> {
        self.checked_read(|_| {
            let reservation = self.reserve_workspace(0)?;
            let mut scratch = Scratch {
                reservation,
                failure: None,
            };
            let result = {
                let mut memory = Memory::new(&mut scratch);
                self.data.lookup(value, &mut memory)
            };
            let result = match scratch.failure.take() {
                Some(error) => Err(error),
                None => result,
            };
            self.record(result)
        })?
    }

    fn reserve_workspace(
        &self,
        bytes: u64,
    ) -> Result<impl WorkspaceReservation<Error = Self::ReadError> + '_, Self::ReadError> {
        if let Some(error) = self.read_error() {
            return Err(self.latch(error));
        }
        let result = (|| {
            let bootstrap = reserve_source(self.data.sources[0], 0)?;
            let limit = self.account.limit + (self.data.stats.auxiliary_bytes - self.account.live);
            let mut result = Reservation {
                account: Account {
                    grants: Vec::new(),
                    bootstrap,
                    live: 0,
                    limit,
                    failure: None,
                },
                live: &self.live,
                limit,
                bytes: 0,
                control: 0,
            };
            let mut grants = Vec::new();
            let allocated = {
                let mut scratch = Scratch {
                    reservation: BorrowReservation(&mut result),
                    failure: None,
                };
                let allocated =
                    Memory::new(&mut scratch).reserve(&mut grants, self.data.sources.len() - 1);
                match scratch.failure.take() {
                    Some(e) => Err(e),
                    None => allocated.map_err(UnionError::Storage),
                }
            };
            allocated?;
            result.control = result.bytes;
            for (at, source) in self.data.sources.iter().copied().enumerate().skip(1) {
                let mut grant = reserve_source(source, at)?;
                grant.resize(result.bytes as u64)?;
                grants.push(grant);
            }
            result.account.grants = grants;
            result.resize(bytes)?;
            Ok(result)
        })();
        self.record(result)
    }

    fn storage_live_budget(&self) -> Option<u64> {
        u64::try_from(self.account.limit + (self.data.stats.auxiliary_bytes - self.account.live))
            .ok()
    }
    fn quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.rows(Table::Ordinary)
    }
    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.rows(Table::Reifier)
    }
    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.rows(Table::Annotation)
    }
    fn named_graphs(&self) -> impl Iterator<Item = Self::Id> + '_ {
        self.data.graphs.iter().copied()
    }
    fn capabilities(&self) -> RdfStoreCapabilities {
        self.data
            .sources
            .iter()
            .fold(RdfStoreCapabilities::plain_rdf(), |caps, source| {
                caps.union(match source {
                    UnionSource::Left(v, _) => v.capabilities(),
                    UnionSource::Right(v, _) => v.capabilities(),
                })
            })
    }
    fn term_count(&self) -> u64 {
        self.data.representatives.len() as u64
    }
    fn triple_term_nesting_bound(&self) -> Option<usize> {
        crate::dataset_view::widest_nesting_bound(self.data.sources.iter().map(
            |source| match source {
                UnionSource::Left(v, _) => v.triple_term_nesting_bound(),
                UnionSource::Right(v, _) => v.triple_term_nesting_bound(),
            },
        ))
    }
    fn supports_named_graphs(&self) -> bool {
        self.data.sources.iter().any(|source| match source {
            UnionSource::Left(v, _) => v.supports_named_graphs(),
            UnionSource::Right(v, _) => v.supports_named_graphs(),
        })
    }
    fn probe_plan(&self, _: bool, _: bool, _: bool, _: GraphMatch<Self::Id>) -> Self::ProbePlan {}
    fn quads_for_pattern_with_plan(
        &self,
        (): &Self::ProbePlan,
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        g: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.quads_for_pattern(s, p, o, g)
    }
}

impl<A: DatasetView, B: DatasetView> Parts<'_, A, B> {
    fn lookup<S: Admission + ?Sized>(
        &self,
        value: &TermValue,
        memory: &mut Memory<'_, S>,
    ) -> Result<Option<UnionTermId>, UnionError<A::ReadError, B::ReadError>> {
        crate::ir::try_fold_nested_with_memory(
            value,
            &mut (),
            |(), value, _| {
                use crate::ir::Nested;
                let term = match value {
                    TermValue::Triple { s, p, o } => return Ok(Nested::Triple(&**s, &**p, &**o)),
                    TermValue::Iri(iri) => TermRef::Iri(iri),
                    TermValue::Blank { label, scope } => TermRef::Blank {
                        label,
                        scope: *scope,
                    },
                    TermValue::Literal {
                        lexical_form,
                        datatype,
                        language,
                        direction,
                    } => {
                        let Some(datatype) = self.find(TermRef::Iri(datatype))? else {
                            return Ok(Nested::Leaf(None));
                        };
                        TermRef::Literal {
                            lexical: lexical_form,
                            datatype,
                            language: language.as_deref(),
                            direction: *direction,
                        }
                    }
                };
                self.find(term).map(Nested::Leaf)
            },
            |(), _, s, p, o, _| match (s, p, o) {
                (Some(s), Some(p), Some(o)) => self.find(TermRef::Triple { s, p, o }),
                _ => Ok(None),
            },
            |(), _, _| {},
            |error, _| UnionError::Storage(error),
            memory,
        )
    }
}

impl<A: DatasetView, B: DatasetView> Parts<'_, A, B> {
    fn resolve(
        &self,
        id: UnionTermId,
    ) -> Result<UnionTermGuard<'_, A, B>, UnionError<A::ReadError, B::ReadError>> {
        let origin = self
            .representatives
            .get(
                usize::try_from(id.0)
                    .map_err(|_| UnionError::Invariant("foreign union term ID"))?,
            )
            .ok_or(UnionError::Invariant("foreign union term ID"))?;
        let pin = match (&self.entries[origin.source], origin.local) {
            (Entry::Left(view, _, index), Local::Left(id)) => {
                let at = index
                    .position(id)
                    .ok_or(UnionError::Invariant("missing source term"))?;
                let guard = view
                    .resolve(id)
                    .map_err(|e| UnionError::Left(origin.source, e))?;
                mapped::<_, A::ReadError, B::ReadError>(guard.term(), index, at)?;
                Pin::Left(guard, index, at)
            }
            (Entry::Right(view, _, index), Local::Right(id)) => {
                let at = index
                    .position(id)
                    .ok_or(UnionError::Invariant("missing source term"))?;
                let guard = view
                    .resolve(id)
                    .map_err(|e| UnionError::Right(origin.source, e))?;
                mapped::<_, A::ReadError, B::ReadError>(guard.term(), index, at)?;
                Pin::Right(guard, index, at)
            }
            _ => return Err(UnionError::Invariant("source identity type changed")),
        };
        Ok(UnionTermGuard { pin })
    }

    fn find(
        &self,
        term: TermRef<'_, UnionTermId>,
    ) -> Result<Option<UnionTermId>, UnionError<A::ReadError, B::ReadError>> {
        let mut failure = None;
        let hash = hash_of(&term);
        let found = self
            .keys
            .find(hash, |(_, id)| match self.resolve(*id) {
                Ok(pin) => pin.term() == term,
                Err(error) => {
                    if failure.is_none() {
                        failure = Some(error);
                    }
                    false
                }
            })
            .map(|(_, id)| *id);
        match failure {
            Some(error) => Err(error),
            None => Ok(found),
        }
    }

    fn intern_all<S: Admission + ?Sized>(
        &mut self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), UnionError<A::ReadError, B::ReadError>> {
        let mut pending = Vec::new();
        for source in 0..self.entries.len() {
            for root in 0..self.entries[source].len() {
                memory.push(&mut pending, (root, false))?;
                while let Some((at, finish)) = pending.pop() {
                    let state = self.entries[source].state(at);
                    if state == 2 {
                        continue;
                    }
                    if !finish {
                        if state == 1 {
                            return Err(UnionError::Invariant("cyclic source term references"));
                        }
                        self.entries[source].set_state(at, 1);
                        memory.push(&mut pending, (at, true))?;
                        let children = match &self.entries[source] {
                            Entry::Left(v, _, i) => {
                                let pin = v
                                    .resolve(i.ids[at])
                                    .map_err(|e| UnionError::Left(source, e))?;
                                children(pin.term()).map(|id| id.and_then(|id| i.position(id)))
                            }
                            Entry::Right(v, _, i) => {
                                let pin = v
                                    .resolve(i.ids[at])
                                    .map_err(|e| UnionError::Right(source, e))?;
                                children(pin.term()).map(|id| id.and_then(|id| i.position(id)))
                            }
                        };
                        for child in children.into_iter().flatten().rev() {
                            memory.push(&mut pending, (child, false))?;
                        }
                        continue;
                    }
                    let (hash, found) = match &self.entries[source] {
                        Entry::Left(v, _, i) => {
                            let pin = v
                                .resolve(i.ids[at])
                                .map_err(|e| UnionError::Left(source, e))?;
                            let term = mapped::<_, A::ReadError, B::ReadError>(pin.term(), i, at)?;
                            (hash_of(&term), self.find(term)?)
                        }
                        Entry::Right(v, _, i) => {
                            let pin = v
                                .resolve(i.ids[at])
                                .map_err(|e| UnionError::Right(source, e))?;
                            let term = mapped::<_, A::ReadError, B::ReadError>(pin.term(), i, at)?;
                            (hash_of(&term), self.find(term)?)
                        }
                    };
                    let canonical = if let Some(id) = found {
                        id
                    } else {
                        let id = UnionTermId(
                            u64::try_from(self.representatives.len())
                                .map_err(|_| UnionError::Storage(StorageError::SizeOverflow))?,
                        );
                        reserve_table_with_memory(
                            &mut self.keys,
                            self.representatives.len() + 1,
                            |(hash, _)| *hash,
                            memory,
                        )?;
                        let local = match &self.entries[source] {
                            Entry::Left(_, _, i) => Local::Left(i.ids[at]),
                            Entry::Right(_, _, i) => Local::Right(i.ids[at]),
                        };
                        memory.push(&mut self.representatives, Origin { source, local })?;
                        self.keys.insert_unique(hash, (hash, id), |(hash, _)| *hash);
                        id
                    };
                    match &mut self.entries[source] {
                        Entry::Left(_, _, i) => {
                            i.aliases[at] = Some(canonical);
                            i.states[at] = 2;
                        }
                        Entry::Right(_, _, i) => {
                            i.aliases[at] = Some(canonical);
                            i.states[at] = 2;
                        }
                    }
                }
            }
        }
        memory.release_vec(pending)?;
        for entry in &mut self.entries {
            match entry {
                Entry::Left(_, _, i) => finish_index(i, memory)?,
                Entry::Right(_, _, i) => finish_index(i, memory)?,
            }
        }
        Ok(())
    }
}

fn children<I: Copy>(term: TermRef<'_, I>) -> [Option<I>; 3] {
    match term {
        TermRef::Literal { datatype, .. } => [Some(datatype), None, None],
        TermRef::Triple { s, p, o } => [Some(s), Some(p), Some(o)],
        _ => [None; 3],
    }
}

fn finish_index<I: ViewTermId, S: Admission + ?Sized>(
    index: &mut Index<I>,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    for (at, id) in index.ids.iter().enumerate() {
        index.reverse.insert(
            index.aliases[at].expect("all captured terms interned"),
            *id,
            memory,
        )?;
    }
    memory.release_vec(std::mem::take(&mut index.states))
}

enum Either<A, B> {
    Left(A),
    Right(B),
}
impl<T, A: Iterator<Item = T>, B: Iterator<Item = T>> Iterator for Either<A, B> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match self {
            Self::Left(v) => v.next(),
            Self::Right(v) => v.next(),
        }
    }
}

struct BorrowReservation<'a, R>(&'a mut R);
impl<R: WorkspaceReservation> WorkspaceReservation for BorrowReservation<'_, R> {
    type Error = R::Error;
    fn resize(&mut self, bytes: u64) -> Result<(), Self::Error> {
        self.0.resize(bytes)
    }
}
