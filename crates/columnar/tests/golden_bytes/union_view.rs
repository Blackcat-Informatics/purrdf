// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The backend-owned plus resident archive path, including live source failures.

use purrdf_columnar::{ColumnarSinkError, Table, WriteOptions, write_to};
use purrdf_core::{
    BlankScope, ContentStore, DatasetView, GraphMatch, QuadIds, RdfDataset, RdfDatasetBuilder,
    RdfLiteral, RdfStoreCapabilities, ScopeBinding, TermId, TermRef, TermValue, UnionDatasetView,
    UnionError, UnionSource, ViewLimits, ViewTermId, WorkspaceReservation,
};
use std::convert::Infallible;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Key(u64);
impl ViewTermId for Key {
    type JoinKeyAtom = u128;
    fn encode(self) -> u128 {
        u128::from(self.0)
    }
    fn encode_computed(index: u32) -> u128 {
        (1_u128 << 127) | u128::from(index)
    }
}
fn key(id: TermId) -> Key {
    Key(id.index() as u64 * 17 + 101)
}
fn local(id: Key) -> TermId {
    assert_eq!((id.0 - 101) % 17, 0);
    TermId::from_index(u32::try_from((id.0 - 101) / 17).unwrap()).unwrap()
}
fn row(q: QuadIds) -> QuadIds<Key> {
    QuadIds {
        s: key(q.s),
        p: key(q.p),
        o: key(q.o),
        g: q.g.map(key),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum HostError {
    Iterator,
    Resolve,
    CompletedRead,
    Reservation,
}
impl std::fmt::Display for HostError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "host {self:?}")
    }
}
impl std::error::Error for HostError {}

struct Host {
    data: Arc<RdfDataset>,
    mode: AtomicUsize,
    failure: Mutex<Option<HostError>>,
    checked: AtomicUsize,
    live: AtomicUsize,
    ceiling: AtomicUsize,
    pulled: AtomicUsize,
}
impl Host {
    fn new(data: Arc<RdfDataset>) -> Self {
        Self {
            data,
            mode: AtomicUsize::new(0),
            failure: Mutex::new(None),
            checked: AtomicUsize::new(0),
            live: AtomicUsize::new(0),
            ceiling: AtomicUsize::new(usize::MAX),
            pulled: AtomicUsize::new(0),
        }
    }
    fn fail(&self, error: HostError) -> HostError {
        self.failure.lock().unwrap().get_or_insert(error).clone()
    }
}
struct HostReservation<'a> {
    host: &'a Host,
    bytes: usize,
}
impl WorkspaceReservation for HostReservation<'_> {
    type Error = HostError;
    fn resize(&mut self, bytes: u64) -> Result<(), HostError> {
        let bytes = usize::try_from(bytes).unwrap();
        let live = self.host.live.load(Ordering::Relaxed) - self.bytes + bytes;
        if live > self.host.ceiling.load(Ordering::Relaxed) {
            return Err(self.host.fail(HostError::Reservation));
        }
        self.host.live.store(live, Ordering::Relaxed);
        self.bytes = bytes;
        Ok(())
    }
}
impl Drop for HostReservation<'_> {
    fn drop(&mut self) {
        self.host.live.fetch_sub(self.bytes, Ordering::Relaxed);
    }
}

impl DatasetView for Host {
    type Id = Key;
    type ReadError = HostError;
    type TermGuard<'a> = TermRef<'a, Key>;
    type ProbePlan = ();
    fn read_error(&self) -> Option<HostError> {
        self.failure.lock().unwrap().clone()
    }
    fn checked_read<T>(&self, read: impl FnOnce(&Self) -> T) -> Result<T, HostError> {
        self.checked.fetch_add(1, Ordering::Relaxed);
        if let Some(error) = self.read_error() {
            return Err(error);
        }
        let result = read(self);
        if self.mode.load(Ordering::Relaxed) == 3 {
            return Err(self.fail(HostError::CompletedRead));
        }
        self.read_error().map_or(Ok(result), Err)
    }
    fn reserve_workspace(
        &self,
        bytes: u64,
    ) -> Result<impl WorkspaceReservation<Error = HostError> + '_, HostError> {
        let mut reservation = HostReservation {
            host: self,
            bytes: 0,
        };
        reservation.resize(bytes)?;
        Ok(reservation)
    }
    fn quads(&self) -> impl Iterator<Item = QuadIds<Key>> + '_ {
        let mut rows = self.data.quads();
        let mut at = 0;
        std::iter::from_fn(move || {
            self.pulled.fetch_add(1, Ordering::Relaxed);
            let next = rows.next();
            let fail = self.mode.load(Ordering::Relaxed) == 1 && at == 1;
            at += 1;
            if fail {
                self.fail(HostError::Iterator);
                None
            } else {
                next.map(row)
            }
        })
    }
    fn resolve(&self, id: Key) -> Result<TermRef<'_, Key>, HostError> {
        if self.mode.load(Ordering::Relaxed) == 2 {
            return Err(self.fail(HostError::Resolve));
        }
        Ok(self.data.as_ref().resolve(local(id)).map_ids(key))
    }
    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<Key>, HostError> {
        Ok(self.data.as_ref().term_id_by_value(value).map(key))
    }
    fn capabilities(&self) -> RdfStoreCapabilities {
        self.data.capabilities()
    }
    fn term_count(&self) -> u64 {
        self.data.term_count()
    }
    fn probe_plan(&self, _: bool, _: bool, _: bool, _: GraphMatch<Key>) -> Self::ProbePlan {}
    fn quads_for_pattern_with_plan(
        &self,
        (): &(),
        s: Option<Key>,
        p: Option<Key>,
        o: Option<Key>,
        g: GraphMatch<Key>,
    ) -> impl Iterator<Item = QuadIds<Key>> + '_ {
        self.quads_for_pattern(s, p, o, g)
    }
    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds<Key>> + '_ {
        self.data.reifier_quads().map(row)
    }
    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds<Key>> + '_ {
        self.data.annotation_quads().map(row)
    }
    fn named_graphs(&self) -> impl Iterator<Item = Key> + '_ {
        self.data.named_graphs().map(key)
    }
}

fn fixture(extra: bool) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    if extra {
        b.intern_iri("urn:unused").unwrap();
    }
    let s = b.intern_iri("urn:s").unwrap();
    let p = b.intern_iri("urn:p").unwrap();
    let blank = b.intern_blank("node", BlankScope(4)).unwrap();
    let list = b
        .intern_literal(RdfLiteral::typed(
            "[_:embedded]",
            "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List",
        ))
        .unwrap();
    let triple = b.intern_triple(s, p, blank).unwrap();
    b.push_quad(s, p, triple, None).unwrap();
    b.push_quad(blank, p, list, None).unwrap();
    let r = b.intern_iri("urn:r").unwrap();
    b.push_reifier_in_graph(r, triple, None).unwrap();
    b.push_annotation(r, p, list).unwrap();
    let graph = b.intern_iri("urn:empty").unwrap();
    b.declare_named_graph(graph).unwrap();
    if extra {
        let additional = b.intern_literal(RdfLiteral::simple("extra")).unwrap();
        b.push_quad(s, p, additional, None).unwrap();
    }
    b.freeze().unwrap()
}

fn files<D: DatasetView>(view: &D) -> [Vec<u8>; 5] {
    let mut files: [Vec<u8>; 5] = std::array::from_fn(|_| Vec::new());
    write_to(
        view,
        &ContentStore::new(),
        &WriteOptions::default(),
        |table, bytes| {
            files[Table::ALL
                .iter()
                .position(|candidate| *candidate == table)
                .unwrap()]
            .extend_from_slice(bytes);
            Ok::<_, Infallible>(())
        },
    )
    .unwrap();
    files
}

pub(super) fn arbitrary_backend_union_writes_the_same_archive_and_scoped_rdf() {
    let original = fixture(false);
    let extra = fixture(true);
    let host = Host::new(original);
    let shared = (ScopeBinding::Shared, ScopeBinding::Shared);
    {
        let union = UnionDatasetView::new(&host, &*extra, shared, ViewLimits::default()).unwrap();
        assert_eq!(files(&union), files(&*extra));
        assert_eq!(union.quads().count(), extra.quad_count());
        assert_eq!(union.reifier_quads().count(), extra.reifiers().count());
        assert_eq!(
            union.annotation_quads().count(),
            extra.annotations().count()
        );
        assert!(
            union
                .term_id_by_value(&TermValue::iri("urn:unused"))
                .unwrap()
                .is_none()
        );
        let s = union
            .term_id_by_value(&TermValue::iri("urn:s"))
            .unwrap()
            .unwrap();
        assert_eq!(
            union
                .quads_for_pattern(Some(s), None, None, GraphMatch::Any)
                .count(),
            2
        );
        let third = fixture(false);
        let nested = union
            .union(&*third, ScopeBinding::Shared, ViewLimits::default())
            .unwrap();
        assert_eq!(nested.stats().retained_sources, 3);
        assert_eq!(files(&nested), files(&*extra));
        let limits = ViewLimits {
            max_sources: 2,
            ..ViewLimits::default()
        };
        assert!(matches!(
            union.union(&*third, ScopeBinding::Shared, limits),
            Err(UnionError::Limit {
                dimension: "sources",
                ..
            })
        ));
        assert!(host.live.load(Ordering::Relaxed) > 0);
    }
    assert_eq!(host.live.load(Ordering::Relaxed), 0);

    // Runtime lists share the pair engine, preserve ordering-independent set
    // identity and admit every original guard, including repeated backend types.
    let third = Host::new(fixture(false));
    let sources = vec![
        UnionSource::Left(&host, ScopeBinding::Shared),
        UnionSource::Right(&*extra, ScopeBinding::Shared),
        UnionSource::Left(&third, ScopeBinding::Shared),
    ];
    let list = UnionDatasetView::from_sources(&sources, ViewLimits::default()).unwrap();
    assert_eq!(list.stats().retained_sources, 3);
    assert_eq!(files(&list), files(&*extra));
    assert_eq!(
        list.reifier_quads().count(),
        extra.reifiers().count(),
        "reifier probe uses the reifier identity"
    );
    drop(list);
    let homogeneous: Vec<UnionSource<'_, Host, Host>> = vec![
        UnionSource::Left(&host, ScopeBinding::Shared),
        UnionSource::Left(&third, ScopeBinding::Shared),
    ];
    assert_eq!(
        files(&UnionDatasetView::from_sources(&homogeneous, ViewLimits::default()).unwrap()),
        files(&*host.data)
    );
    assert!(matches!(
        UnionDatasetView::from_sources(&sources[..1], ViewLimits::default()),
        Err(UnionError::Invariant(_))
    ));
    let limits = ViewLimits {
        max_sources: 2,
        ..ViewLimits::default()
    };
    assert!(matches!(
        UnionDatasetView::from_sources(&sources, limits),
        Err(UnionError::Limit {
            dimension: "sources",
            ..
        })
    ));

    let independent = UnionDatasetView::new(
        &host,
        &*extra,
        (ScopeBinding::Independent, ScopeBinding::Independent),
        ViewLimits::default(),
    )
    .unwrap();
    let mut nodes = Vec::new();
    for q in independent.quads() {
        if let TermValue::Blank { label, scope } = independent.term_value(q.s).unwrap()
            && label == "node"
        {
            nodes.push(scope);
        }
    }
    nodes.sort_unstable();
    nodes.dedup();
    assert_eq!(
        nodes.len(),
        2,
        "independent source blanks retain separate identities"
    );
    let mut embedded = Vec::new();
    for q in independent.quads() {
        let value = independent.term_value(q.o).unwrap();
        if let TermValue::Literal {
            lexical_form,
            datatype,
            ..
        } = value
            && datatype == "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List"
        {
            embedded.push(lexical_form);
        }
    }
    embedded.sort();
    embedded.dedup();
    assert_eq!(
        embedded.len(),
        2,
        "each source's CDT blank identity stays distinct"
    );
    assert_eq!(independent.named_graphs().count(), 1);
}

pub(super) fn arbitrary_backend_union_keeps_live_failures_and_exact_view_limits() {
    let extra = fixture(true);
    for (mode, expected) in [
        (1, HostError::Iterator),
        (2, HostError::Resolve),
        (3, HostError::CompletedRead),
    ] {
        let host = Host::new(fixture(false));
        let union = UnionDatasetView::new(
            &host,
            &*extra,
            (ScopeBinding::Shared, ScopeBinding::Shared),
            ViewLimits::default(),
        )
        .unwrap();
        let checked = host.checked.load(Ordering::Relaxed);
        host.mode.store(mode, Ordering::Relaxed);
        let mut callbacks = 0;
        let error = write_to(
            &union,
            &ContentStore::new(),
            &WriteOptions::default(),
            |_, _| {
                callbacks += 1;
                Ok::<_, Infallible>(())
            },
        )
        .unwrap_err();
        assert!(
            matches!(error, ColumnarSinkError::SourceRead(UnionError::Left(0, ref cause)) if *cause == expected)
        );
        assert_eq!(callbacks, 0);
        assert!(host.checked.load(Ordering::Relaxed) > checked);
    }
    // Construction also executes the actual source wrapper and preserves its
    // after-read failure rather than accepting a partially captured registry.
    let failed = Host::new(fixture(false));
    failed.mode.store(3, Ordering::Relaxed);
    assert!(matches!(
        UnionDatasetView::new(
            &failed,
            &*extra,
            (ScopeBinding::Shared, ScopeBinding::Shared),
            ViewLimits::default()
        ),
        Err(UnionError::Left(0, HostError::CompletedRead))
    ));
    assert_eq!(failed.live.load(Ordering::Relaxed), 0);

    // A later source keeps its ordinal and original reservation cause, including
    // the admitted runtime guard-array allocation before the requested scratch.
    let first_resource = Host::new(fixture(false));
    let last_resource = Host::new(fixture(false));
    let sources = [
        UnionSource::Left(&first_resource, ScopeBinding::Shared),
        UnionSource::Right(&*extra, ScopeBinding::Shared),
        UnionSource::Left(&last_resource, ScopeBinding::Shared),
    ];
    let union = UnionDatasetView::from_sources(&sources, ViewLimits::default()).unwrap();
    last_resource.ceiling.store(
        last_resource.live.load(Ordering::Relaxed) + 1,
        Ordering::Relaxed,
    );
    assert!(matches!(
        union.reserve_workspace(0),
        Err(UnionError::Left(2, HostError::Reservation))
    ));
    drop(union);
    assert_eq!(first_resource.live.load(Ordering::Relaxed), 0);
    assert_eq!(last_resource.live.load(Ordering::Relaxed), 0);

    // A failed source iterator terminates before any later source is pulled.
    let first = Host::new(fixture(false));
    let second = Host::new(fixture(false));
    let sources: Vec<UnionSource<'_, Host, Host>> = vec![
        UnionSource::Left(&first, ScopeBinding::Shared),
        UnionSource::Left(&second, ScopeBinding::Shared),
    ];
    let union = UnionDatasetView::from_sources(&sources, ViewLimits::default()).unwrap();
    let later = second.pulled.load(Ordering::Relaxed);
    first.mode.store(1, Ordering::Relaxed);
    let mut rows = union.quads();
    let _ = rows.next();
    let _ = rows.next();
    let _ = rows.next();
    assert_eq!(second.pulled.load(Ordering::Relaxed), later);
    assert!(matches!(
        union.read_error(),
        Some(UnionError::Left(0, HostError::Iterator))
    ));
    drop(rows);
    drop(union);

    // Source count cannot bypass the native/shadow-stack read-scope floor.
    let host = Host::new(fixture(false));
    let list: [UnionSource<'_, Host, Host>; 2] =
        [UnionSource::Left(&host, ScopeBinding::Shared); 2];
    let previous = purrdf_stack::replace_floor(
        purrdf_stack::stack_pointer().saturating_sub(purrdf_stack::MARGIN_BYTES / 2),
    );
    let refused = UnionDatasetView::from_sources(&list, ViewLimits::default());
    purrdf_stack::replace_floor(previous);
    assert!(matches!(refused, Err(UnionError::Stack { source: 0 })));

    let host = Host::new(fixture(false));
    let union = UnionDatasetView::new(
        &host,
        &*extra,
        (ScopeBinding::Shared, ScopeBinding::Shared),
        ViewLimits::default(),
    )
    .unwrap();
    let count = union.stats().retained_terms;
    drop(union);
    let mut limits = ViewLimits {
        max_terms: count,
        ..ViewLimits::default()
    };
    let valid = UnionDatasetView::new(
        &host,
        &*extra,
        (ScopeBinding::Shared, ScopeBinding::Shared),
        limits,
    )
    .unwrap();
    assert_eq!(files(&valid), files(&*extra));
    drop(valid);
    limits.max_terms -= 1;
    assert!(matches!(
        UnionDatasetView::new(
            &host,
            &*extra,
            (ScopeBinding::Shared, ScopeBinding::Shared),
            limits
        ),
        Err(UnionError::Limit {
            dimension: "terms",
            ..
        })
    ));
    assert_eq!(host.live.load(Ordering::Relaxed), 0);
    host.ceiling.store(1, Ordering::Relaxed);
    assert!(matches!(
        UnionDatasetView::new(
            &host,
            &*extra,
            (ScopeBinding::Shared, ScopeBinding::Shared),
            ViewLimits::default()
        ),
        Err(UnionError::Left(0, HostError::Reservation))
    ));
    assert_eq!(host.live.load(Ordering::Relaxed), 0);
}
