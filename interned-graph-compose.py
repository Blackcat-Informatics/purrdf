from pathlib import Path
import difflib
stage=Path(__file__).parent
root=stage.parents[1]
changed={}
def edit(relative,fn):
    relative=Path(relative)
    before=(root/relative).read_text()
    changed[relative]=(before,fn(before))
def interned(s):
    marker='/// A borrowed result of any query form'
    s=s.replace(marker,(stage/'interned-graph-owner.rs').read_text()+'\n'+marker,1)
    s=s.replace("    Graph(&'a Arc<RdfDataset>),","    Graph(InternedGraph<'a>),",1)
    s=s.replace('''    /// by `Arc`; a visitor that keeps it clones the handle, not the data.''','''    /// by its original owner; a visitor retaining it keeps the admission too.''')
    s=s.replace('pub fn try_constructed_dataset(&self) -> Result<Arc<RdfDataset>, crate::EvalError> {\n        self.ctx.constructed_dataset_of(self.seq)\n    }', '''pub fn try_constructed_dataset(&self) -> Result<crate::RetainedGraph, crate::EvalError> {
        let graph = self.ctx.constructed_dataset_of(self.seq)?;
        InternedGraph::new(&graph, &self.ctx.growth).retain()
    }''')
    s=s.replace('pub fn constructed_dataset(&self) -> Arc<RdfDataset>','pub fn constructed_dataset(&self) -> crate::RetainedGraph')
    return s
edit('crates/sparql-eval/src/interned.rs',interned)
edit('crates/sparql-eval/src/retained.rs',lambda s:s.replace('/// Ownership-carrying extraction of a SELECT output.',(stage/'retained-graph-owner.rs').read_text()+'\n/// Ownership-carrying extraction of a SELECT output.',1))
edit('crates/sparql-eval/src/engine.rs',lambda s:s.replace('Outcome::Graph(graph) => InternedOutcome::Graph(graph),', 'Outcome::Graph(graph) => InternedOutcome::Graph(crate::interned::InternedGraph::new(graph, &ctx.growth)),',1))
edit('crates/sparql-eval/src/lib.rs',lambda s:s.replace('    InternedGoverned, InternedOutcome,', '    InternedGraph, InternedGoverned, InternedOutcome,',1).replace('pub use retained::{', 'pub use retained::{RetainedGraph, ',1))
def shapes(s):
    a=s.index('fn project_graph<')
    b=s.index('\n/// Run a SELECT',a)
    p=s[a:b].replace('Result<Arc<RdfDataset>, String>', 'Result<purrdf_sparql_eval::RetainedGraph, String>')
    p=p.replace('Ok(Arc::clone(graph))','graph.retain().map_err(|error| error.to_string())')
    p=p.replace('''        // The graph is already frozen and shared by `Arc`; taking it out of the
        // evaluation is a handle clone, not a copy of the derived triples.''', '''        // Keep the original account with the shared frozen graph through rule
        // consumption; retaining copies no derived triples.''')
    s=s[:a]+p+s[b:]
    a=s.index('pub(crate) fn run_bound_construct_with_shacl_prebinding_view<')
    b=s.index('\n/// An RAII',a)
    p=s[a:b].replace('Result<Arc<RdfDataset>, String>', 'Result<purrdf_sparql_eval::RetainedGraph, String>')
    return s[:a]+p+s[b:]
edit('crates/shapes/src/sparql.rs',shapes)
edit('crates/sparql-eval/tests/segmented_query.rs',lambda s:s+'\n'+(stage/'interned-graph-retention-test.rs').read_text())
patch=[]
for relative,(before,after) in changed.items():
    post=stage/'interned-graph-postimages'/relative
    post.parent.mkdir(parents=True,exist_ok=True)
    post.write_text(after)
    patch.extend(difflib.unified_diff(before.splitlines(keepends=True),after.splitlines(keepends=True),fromfile='a/'+str(relative),tofile='b/'+str(relative)))
(stage/'interned-graph-owner-draft.patch').write_text(''.join(patch))
