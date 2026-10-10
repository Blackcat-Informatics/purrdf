from pathlib import Path
import difflib
import hashlib
import json

root = Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
stage = root / '.stage/sparql-eval-complete-bounded-workspace'
out = stage / 'wasm-dataset-handle-postimages'
paths = [
    'crates/rdf-wasm/src/dataset.rs',
    'crates/rdf-wasm/src/query.rs',
    'crates/rdf-wasm/src/stream.rs',
    'crates/rdf-wasm/src/projection.rs',
]
originals = {}
identities = {}

def replace(text, before, after, count=1):
    actual = text.count(before)
    assert actual == count, (before, actual, count)
    return text.replace(before, after)

for path in paths:
    before = (root / path).read_text()
    originals[path] = before
    base = stage / 'wasm-dataset-handle-preimages' / path
    base.parent.mkdir(parents=True, exist_ok=True)
    base.write_text(before)
    text = before
    if path.endswith('/dataset.rs'):
        text = replace(text, 'use purrdf_lex::json::{self, Value};',
                       'use purrdf_core::DatasetHandle;\nuse purrdf_lex::json::{self, Value};')
        text = replace(text,
            '    /// A new dataset over a frozen base, with a fresh identity at generation zero.\n'
            '    pub(crate) fn from_frozen(frozen: Arc<RdfDataset>) -> Result<Self, JsError> {',
            '    /// A new dataset over a frozen base, with a fresh identity at generation zero.\n'
            '    /// The mutable base retains the original native admission through this host\n'
            '    /// object; resident parser and sink owners enter through `Arc::into`.\n'
            '    pub(crate) fn from_frozen(frozen: DatasetHandle) -> Result<Self, JsError> {')
        text = replace(text, 'Self::from_frozen(dataset)', 'Self::from_frozen(dataset.into())')
        text = replace(text, 'Self::from_frozen(frozen)', 'Self::from_frozen(frozen.into())')
        text = replace(text, '    frozen: &Arc<RdfDataset>,\n', '    frozen: &RdfDataset,\n')
    elif path.endswith('/query.rs'):
        text = replace(text, 'fn default_graph_format(graph: &Arc<purrdf::RdfDataset>)',
                       'fn default_graph_format(graph: &purrdf::RdfDataset)')
        text = replace(text, '    graph: &Arc<purrdf::RdfDataset>,\n',
                       '    graph: &purrdf::RdfDataset,\n', 2)
        text = replace(text, 'fn serialize_graph_result(graph: &Arc<purrdf::RdfDataset>,',
                       'fn serialize_graph_result(graph: &purrdf::RdfDataset,')
        text = replace(text, 'if distinct_graph_names(&**graph).is_empty()',
                       'if distinct_graph_names(graph).is_empty()')
        text = replace(text, 'let names = distinct_graph_names(&**graph);',
                       'let names = distinct_graph_names(graph);')
        anchor = '    /// Two blank nodes that share the label `b` in different scopes: the engine\'s\n'
        fixture = '''    /// The native graph's original storage survives serialization, a wrong-kind
    /// take, and transfer out of the result wrapper; only its last host owner releases it.
    #[test]
    fn native_graph_storage_follows_wasm_result_and_dataset_lifetimes() {
        use purrdf_core::DatasetHandle;
        use purrdf_lex::allocation::{Admission, Memory, StorageError};
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct GraphStorage {
            live: Arc<AtomicUsize>,
            dropped: Arc<AtomicUsize>,
        }

        impl Admission for GraphStorage {
            fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
                self.live.store(bytes, Ordering::SeqCst);
                Ok(())
            }
        }

        impl Drop for GraphStorage {
            fn drop(&mut self) {
                self.live.store(0, Ordering::SeqCst);
                self.dropped.fetch_add(1, Ordering::SeqCst);
            }
        }

        for typed_wrapper in [false, true] {
            let live = Arc::new(AtomicUsize::new(0));
            let dropped = Arc::new(AtomicUsize::new(0));
            let mut storage = GraphStorage {
                live: Arc::clone(&live),
                dropped: Arc::clone(&dropped),
            };
            let mut memory = Memory::new(&mut storage);
            let mut builder = purrdf::RdfDatasetBuilder::new();
            let subject = builder
                .intern_iri_with_memory("https://example.org/s", &mut memory)
                .expect("native subject");
            let predicate = builder
                .intern_iri_with_memory("https://example.org/p", &mut memory)
                .expect("native predicate");
            let object = builder
                .intern_iri_with_memory("https://example.org/o", &mut memory)
                .expect("native object");
            let graph_name = builder
                .intern_iri_with_memory("https://example.org/g", &mut memory)
                .expect("native graph name");
            builder
                .push_quad_with_memory(subject, predicate, object, Some(graph_name), &mut memory)
                .expect("native quad");
            let value = builder
                .freeze_with_memory(&mut memory)
                .expect("native freeze");
            memory
                .add_bytes(DatasetHandle::allocation_layout::<GraphStorage>().size())
                .expect("original shared control admission");
            let admitted = memory.live_bytes();
            assert!(admitted > 0);
            let graph = DatasetHandle::try_from_admitted(value, storage)
                .expect("original storage publication");
            assert!(graph.retains_admission());
            let original = std::ptr::from_ref(graph.as_ref());
            let retained = graph.clone();
            let result = SparqlResult::Graph(graph);
            let bytes = serialize_query_result(&result, Some("nquads"), None, "")
                .expect("borrowed native graph serialization");
            assert!(bytes.contains("<https://example.org/g>"));
            assert_eq!(live.load(Ordering::SeqCst), admitted);
            let dataset = if typed_wrapper {
                let mut result = query_result_from_sparql(result, BlankScopeMode::Keep)
                    .expect("native graph wrapper");
                assert!(result.take_select().is_none());
                assert_eq!(live.load(Ordering::SeqCst), admitted);
                let dataset = result.take_dataset().expect("move host dataset");
                assert!(result.take_dataset().is_none());
                drop(result);
                dataset
            } else {
                graph_result_from_sparql(result).expect("direct host dataset")
            };
            assert!(std::ptr::eq(dataset.view().base().as_ref(), original));
            drop(retained);
            assert_eq!(live.load(Ordering::SeqCst), admitted);
            assert_eq!(dropped.load(Ordering::SeqCst), 0);
            let bytes = dataset.serialize("nquads", None).expect("host graph serialization");
            assert!(bytes.contains("<https://example.org/g>"));
            assert_eq!(live.load(Ordering::SeqCst), admitted);
            drop(dataset);
            assert_eq!(live.load(Ordering::SeqCst), 0);
            assert_eq!(dropped.load(Ordering::SeqCst), 1);
        }
    }

'''
        text = replace(text, anchor, fixture + anchor)
    elif path.endswith('/stream.rs'):
        text = replace(text, 'Dataset::from_frozen(dataset)', 'Dataset::from_frozen(dataset.into())')
    elif path.endswith('/projection.rs'):
        text = replace(text, 'Dataset::from_frozen(outcome.dataset)',
                       'Dataset::from_frozen(outcome.dataset.into())')
    dest = out / path
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(text)
    identities[path] = {'source_sha256': hashlib.sha256(before.encode()).hexdigest()}

(stage / 'wasm-dataset-handle-source-identities.json').write_text(json.dumps(identities, indent=2) + '\n')

def write_diff():
    parts = []
    for path in paths:
        after = (out / path).read_text()
        identities[path]['postimage_sha256'] = hashlib.sha256(after.encode()).hexdigest()
        parts.extend(difflib.unified_diff(originals[path].splitlines(True), after.splitlines(True),
                     fromfile='a/' + path, tofile='b/' + path))
    (stage / 'wasm-dataset-handle-owner-draft.patch').write_text(''.join(parts))
    (stage / 'wasm-dataset-handle-source-identities.json').write_text(json.dumps(identities, indent=2) + '\n')

if __name__ == '__main__':
    write_diff()
