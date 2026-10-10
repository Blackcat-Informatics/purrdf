from pathlib import Path
import difflib,re
root=Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
stage=root/'.stage/sparql-eval-complete-bounded-workspace'
post=stage/'native-witness-postimages';post.mkdir(exist_ok=True)
changes={}
def read(path):
    text=(root/path).read_text();changes[path]=[text,text];return text
def save(path,text):changes[path][1]=text
def replace_body(text,head,replacement):
    start=text.index(head);opening=text.index('{',start);depth=1;end=opening+1
    while depth:
        if text[end]=='{':depth+=1
        elif text[end]=='}':depth-=1
        end+=1
    return text[:start]+replacement+text[end:]

p='crates/sparql-eval/src/witness.rs';t=read(p)
t=t.replace('use std::collections::{BTreeMap, BTreeSet, btree_map};','#[cfg(test)]\nuse std::collections::BTreeSet;')
t=t.replace('pub generations: BTreeSet<IndexGeneration>','pub generations: WitnessSet<IndexGeneration>')
t=t.replace('pub incompleteness: BTreeSet<String>','pub incompleteness: WitnessSet<String>')
start=t.index('impl RelationAttestations {');end=t.index('\n/// What every relation',start)
t=t[:start]+t[end:]
t=t.replace('/// Per-relation ledgers, keyed by registered IRI. A [`BTreeMap`], so iteration is in\n    /// IRI order on every target and in every build — a hash map would make the order of\n    /// [`Self::iter`], and therefore of anything a caller derived from it, a function of\n    /// the hasher rather than of what was attested.\n    entries: BTreeMap<String, RelationAttestations>,','/// Per-relation ledgers in a sorted native array, sharing original buffer\n    /// ownership when a receipt is cloned.\n    entries: Option<crate::workspace::SharedWorkspace<WitnessEntries>>,')
start=t.index('pub struct RelationWitness');preceding=t.rfind('#[derive',0,start)
t=t[:preceding]+t[preceding:].replace('#[derive(Clone, Debug, Default, PartialEq, Eq)]','#[derive(Clone, Debug, Default)]',1)
t=t.replace('entries: BTreeMap::new()','entries: None')
t=replace_body(t,'    pub fn is_empty(','''    pub fn is_empty(&self) -> bool { self.len() == 0 }''')
t=replace_body(t,'    pub fn len(','''    pub fn len(&self) -> usize { self.entries.as_ref().map_or(0, |entries| entries.len()) }''')
t=replace_body(t,'    pub fn get(','''    pub fn get(&self, iri: &str) -> Option<&RelationAttestations> {
        let entries = self.entries.as_ref()?;
        entries.binary_search_by(|(name, _)| name.as_str().cmp(iri)).ok().map(|index| &entries[index].1)
    }''')
t=replace_body(t,'    pub fn iter(','''    pub fn iter(&self) -> impl Iterator<Item = (&str, &RelationAttestations)> {
        self.entries.as_deref().map_or(&[][..], |entries| &entries[..]).iter().map(|(iri, attestations)| (iri.as_str(), attestations))
    }''')
t=replace_body(t,'    pub fn record(','''    pub fn record(&mut self, iri: &str, generation: IndexGeneration, service: ServiceLevel) {
        self.try_record(iri, &generation, &service, &crate::workspace::WorkspaceCapability::resident())
            .expect("resident relation witness allocation failed");
    }''')
t=replace_body(t,'    pub fn merge(','''    pub fn merge(&mut self, other: Self) {
        self.try_merge(&other, &crate::workspace::WorkspaceCapability::resident())
            .expect("resident relation witness merge allocation failed");
    }''')
t=t.replace('#[cfg(test)]\nmod tests {',(stage/'native-witness-storage.rs').read_text()+'\n'+(stage/'native-witness-methods.rs').read_text()+'\n#[cfg(test)]\nmod tests {',1)
save(p,t)
p='crates/sparql-eval/src/eval.rs';t=read(p)
t=replace_body(t,'    pub(crate) fn absorb_worker_witnesses(','''    pub(crate) fn absorb_worker_witnesses(
        &mut self, witnesses: impl IntoIterator<Item = RelationWitness>,
    ) -> Result<(), EvalError> {
        for witness in witnesses { self.witness.try_merge(&witness, &self.growth)?; }
        Ok(())
    }''')
save(p,t)
for p in ['crates/sparql-eval/src/expr.rs','crates/sparql-eval/src/modifier.rs','crates/sparql-eval/src/binop.rs','crates/sparql-eval/src/user_fn.rs']:
    t=read(p)
    # Complete call expressions may contain nested arguments. Apply propagation
    # at their own closing delimiter, never inside a nested map/array expression.
    inserts=[]
    for match in re.finditer(r'ctx\.absorb_worker_witnesses\(',t):
        depth=1;end=match.end()
        while depth:
            if t[end]=='(':depth+=1
            elif t[end]==')':depth-=1
            end+=1
        inserts.append(end)
    for end in reversed(inserts):t=t[:end]+'?'+t[end:]
    if p.endswith('user_fn.rs'):
        old='''    if !witness.is_empty() {
        ctx.absorb_worker_witnesses([witness])?;
    }
    result'''
        new='''    let witness_result = if witness.is_empty() { Ok(()) }
        else { ctx.absorb_worker_witnesses([witness]) };
    // The original function failure wins over a derived receipt refusal.
    result.and_then(|value| witness_result.map(|()| value))'''
        assert old in t;t=t.replace(old,new,1)
    save(p,t)
p='crates/sparql-eval/src/property_fn_eval.rs';t=read(p)
t=t.replace('ctx.witness.record(&call.iri, generation, service);','ctx.witness.try_record(&call.iri, &generation, &service, &ctx.growth)?;')
t=replace_body(t,'    pub fn settle(','''    pub fn settle(&self) -> Result<RelationWitness, EvalError> {
        self.settle_admitted(&crate::workspace::WorkspaceCapability::resident())
    }

    /// Settle a bounded invocation under the original operation's admission.
    ///
    /// # Errors
    /// Returns original declaration failure or native receipt storage refusal.
    pub fn settle_admitted(&self, workspace: &crate::workspace::WorkspaceCapability) -> Result<RelationWitness, EvalError> {
        let generation = generation_contained(&*self.cursor, &self.iri)?;
        let service = service_level_contained(&*self.cursor, &self.iri)?;
        let mut witness = RelationWitness::default();
        witness.try_record(&self.iri, &self.opened.generation, &service, workspace)?;
        if generation != self.opened.generation {
            witness.try_record(&self.iri, &generation, &service, workspace)?;
        }
        Ok(witness)
    }''')
save(p,t)
p='crates/sparql-eval/src/lib.rs';t=read(p)
t=t.replace('pub use witness::{RelationAttestations, RelationWitness};','pub use witness::{RelationAttestations, RelationWitness, WitnessSet, WitnessSetIter};')
save(p,t)
for path,(original,updated) in changes.items():
    dest=post/path;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(updated)
patch=''.join(''.join(difflib.unified_diff(original.splitlines(True),updated.splitlines(True),fromfile='a/'+path,tofile='b/'+path)) for path,(original,updated) in changes.items())
(stage/'native-relation-witness-owner-draft.patch').write_text(patch)
print(len(patch.splitlines()),'patch lines')
