from pathlib import Path

root=Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
stage=root/'.stage/sparql-eval-complete-bounded-workspace'
post=stage/'graph-native-postimages'
p='crates/rdf-core/src/ir/dataset.rs'
t=(root/p).read_text()
def replace_body(text,head,replacement):
    start=text.index(head);brace=text.index('{',start);depth=1;end=brace+1
    while depth:
        if text[end]=='{':depth+=1
        elif text[end]=='}':depth-=1
        end+=1
    return text[:start]+replacement+text[end:]
t=replace_body(t,'    fn try_statement_graph_rows(','''    fn try_statement_graph_rows(&self) -> Result<&HashTable<(TermId, usize)>, hashbrown::TryReserveError> {
        if let Some(rows) = self.statement_graph_rows.get() { return Ok(rows); }
        let mut rows = HashTable::new();
        rows.try_reserve(self.named_graphs.len(), |&(graph, _)| crate::hash::hash_of(&graph))?;
        self.fill_statement_graph_rows(&mut rows);
        let _ = self.statement_graph_rows.set(rows);
        Ok(self.statement_graph_rows.get().expect("the histogram was installed by this reader or a concurrent reader"))
    }

    fn fill_statement_graph_rows(&self, rows: &mut HashTable<(TermId, usize)>) {
        let graphs = self.reifiers.iter().map(|&(_, _, graph)| graph)
            .chain(self.annotations.iter().map(|&(_, _, _, graph)| graph));
        for graph in graphs.flatten() {
            let hash = crate::hash::hash_of(&graph);
            if let Some((_, count)) = rows.find_mut(hash, |&(key, _)| key == graph) { *count += 1; }
            else { rows.insert_unique(hash, (graph, 1), |&(key, _)| crate::hash::hash_of(&key)); }
        }
    }''')
t=replace_body(t,'    fn try_permutation(','''    fn try_permutation(&self, perm: QuadPermutation) -> Result<&[u32], std::collections::TryReserveError> {
        let cell = self.permutation_cell(perm);
        if cell.get().is_none() {
            let ordinals = self.build_permutation(perm, &mut (),
                |(), values, required| values.try_reserve_exact(required),
                |(), values| Ok(values.into_boxed_slice()))?;
            let _ = cell.set(ordinals);
        }
        Ok(cell.get().expect("this call or a concurrent reader installed the index"))
    }

    fn permutation_cell(&self, perm: QuadPermutation) -> &OnceLock<Box<[u32]>> {
        match perm {
            QuadPermutation::Spog => unreachable!("SPOG is the identity table, never materialized"),
            QuadPermutation::Pos => &self.indexes.pos,
            QuadPermutation::Osp => &self.indexes.osp,
            QuadPermutation::Gspo => &self.indexes.gspo,
            QuadPermutation::Gpos => &self.indexes.gpos,
            QuadPermutation::Gosp => &self.indexes.gosp,
        }
    }

    fn build_permutation<S, E>(
        &self, permutation: QuadPermutation, storage: &mut S,
        reserve: impl FnOnce(&mut S, &mut Vec<u32>, usize) -> Result<(), E>,
        freeze: impl FnOnce(&mut S, Vec<u32>) -> Result<Box<[u32]>, E>,
    ) -> Result<Box<[u32]>, E> {
        let axes = permutation.axes();
        let len = u32::try_from(self.quads.len()).expect("dataset quad count exceeds u32::MAX");
        let mut ordinals = Vec::new();
        reserve(storage, &mut ordinals, self.quads.len())?;
        ordinals.extend(0..len);
        ordinals.sort_unstable_by(|&a, &b| compare_quads(axes, &self.quads[a as usize], &self.quads[b as usize]));
        freeze(storage, ordinals)
    }''')
native='''
    /// Allocate query indexes before native graph publication, under the same
    /// original Memory as its frozen buffers. Exclusive access prevents another
    /// reader from installing an unadmitted index during construction.
    ///
    /// # Errors
    /// Returns physical layout, original admission or allocator refusal.
    pub fn warm_query_indexes_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
        &mut self, memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> Result<(), purrdf_lex::allocation::StorageError> {
        for permutation in [QuadPermutation::Pos, QuadPermutation::Osp, QuadPermutation::Gspo, QuadPermutation::Gpos, QuadPermutation::Gosp] {
            if self.permutation_cell(permutation).get().is_some() { continue; }
            let ordinals = self.build_permutation(permutation, memory,
                |memory, values, required| memory.reserve(values, required),
                |memory, values| memory.boxed_slice(values))?;
            self.permutation_cell(permutation).set(ordinals).expect("exclusive native construction owns the empty index cell");
        }
        if (!self.reifiers.is_empty() || !self.annotations.is_empty()) && self.statement_graph_rows.get().is_none() {
            let mut rows = HashTable::new();
            crate::hash::reserve_table_with_memory(&mut rows, self.named_graphs.len(), |&(graph, _)| crate::hash::hash_of(&graph), memory)?;
            self.fill_statement_graph_rows(&mut rows);
            self.statement_graph_rows.set(rows).expect("exclusive native construction owns the empty histogram cell");
        }
        Ok(())
    }
'''
t=t.replace('    /// A cheap, deterministic fingerprint',native+'\n    /// A cheap, deterministic fingerprint',1)
dest=post/p;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(t)
print('native index homes written')
