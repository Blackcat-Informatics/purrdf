from pathlib import Path
import re

root=Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
stage=root/'.stage/sparql-eval-complete-bounded-workspace'
post=stage/'graph-native-postimages'
path=post/'crates/rdf-core/src/ir/builder.rs'
t=path.read_text()
error='''
/// A native graph build refuses physical storage separately from RDF structure.
#[derive(Debug)]
pub enum NativeBuildError {
    /// Checked layout, original admission or physical allocator refusal.
    Storage(StorageError),
    /// The original RDF diagnostic; its producer's Memory retains its buffers.
    Diagnostic(RdfDiagnostic),
}
impl From<StorageError> for NativeBuildError {
    fn from(error: StorageError) -> Self { Self::Storage(error) }
}
impl core::fmt::Display for NativeBuildError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self { Self::Storage(error) => error.fmt(f), Self::Diagnostic(error) => error.fmt(f) }
    }
}
impl std::error::Error for NativeBuildError {}
'''
t=t.replace('impl RdfDatasetBuilder {',error+'\nimpl RdfDatasetBuilder {',1)
methods='''
    /// Validate and freeze originally admitted native graph buffers by value.
    /// Shared publication happens later at the caller's fallible shared home.
    ///
    /// # Errors
    /// Returns physical refusal or the same RDF diagnostic as resident freeze.
    pub fn freeze_with_memory<S: Admission + ?Sized>(self, memory: &mut Memory<'_, S>) -> Result<RdfDataset, NativeBuildError> {
        let buffers = self.native_buffer_bytes()?;
        if let Err(error) = super::validate::validate_with_memory(&self, memory) {
            drop(self);
            memory.release_bytes(buffers)?;
            return Err(error);
        }
        self.materialize_with_memory(memory).map_err(NativeBuildError::Storage)
    }

    fn native_buffer_bytes(&self) -> Result<usize, StorageError> {
        fn array<T>(values: &Vec<T>) -> Result<usize, StorageError> {
            std::alloc::Layout::array::<T>(values.capacity()).map(|layout| layout.size()).map_err(|_| StorageError::SizeOverflow)
        }
        let dropped_indexes = [self.quad_index.capacity(), self.reifier_index.capacity(), self.annotation_index.capacity()]
            .into_iter().try_fold(0usize, |total, capacity| total.checked_add(crate::hash::hash_table_allocation_bound::<u32>(capacity).ok_or(StorageError::SizeOverflow)?).ok_or(StorageError::SizeOverflow))?;
        [self.interner.buffer_bytes()?, array(&self.quads)?, array(&self.reifiers)?, array(&self.annotations)?, array(&self.locations)?, array(&self.declared_graphs)?, dropped_indexes,
         self.derivation_predicate.as_ref().map_or(0, String::capacity)]
            .into_iter().try_fold(0usize, |total, bytes| total.checked_add(bytes).ok_or(StorageError::SizeOverflow))
    }
'''
t=t.replace('    /// Validate structure (positional constraints',methods+'\n    /// Validate structure (positional constraints',1)
start=t.index('    fn materialize(self)');end=t.index('\n}\n\nuse crate::{RdfDiagnostic',start)
original=t[start:end]
native=original.replace('fn materialize(self) -> RdfDataset','fn materialize_with_memory<S: Admission + ?Sized>(self, memory: &mut Memory<\'_, S>) -> Result<RdfDataset, StorageError>')
native=native.replace('        let Self {','''        let discarded_indexes = [self.quad_index.capacity(), self.reifier_index.capacity(), self.annotation_index.capacity()]
            .into_iter().try_fold(0usize, |total, capacity| total.checked_add(crate::hash::hash_table_allocation_bound::<u32>(capacity).ok_or(StorageError::SizeOverflow)?).ok_or(StorageError::SizeOverflow))?;
        let Self {''',1)
native=native.replace('        } = self;','        } = self;\n        memory.release_bytes(discarded_indexes)?;',1)
native=native.replace('let derivation_predicate_iri = derivation_predicate.map(String::into_boxed_str);','''let derivation_predicate_iri = derivation_predicate.map(|text| -> Result<Box<str>, StorageError> {
            let exact = memory.string(&text)?;
            memory.release_string(text)?;
            Ok(exact.into_boxed_str())
        }).transpose()?;''')
old='''            let mut indexed: Vec<(QuadRow, u32)> = quads
                .into_iter()
                .enumerate()
                .map(|(push, row)| (row, push as u32))
                .collect();'''
new='''            let old_quad_bytes = std::alloc::Layout::array::<QuadRow>(quads.capacity()).map_err(|_| StorageError::SizeOverflow)?.size();
            let mut indexed = memory.collect(quads.into_iter().enumerate().map(|(push, row)| (row, push as u32)))?;
            memory.release_bytes(old_quad_bytes)?;'''
assert old in native;native=native.replace(old,new)
native=native.replace('let mut push_to_frozen = vec![u32::MAX; indexed.len()];','let mut push_to_frozen = memory.collect(core::iter::repeat_n(u32::MAX, indexed.len()))?;\n            let old_indexed_bytes = std::alloc::Layout::array::<(QuadRow, u32)>(indexed.capacity()).map_err(|_| StorageError::SizeOverflow)?.size();')
native=native.replace('let quads: Vec<QuadRow> = indexed','let quads = memory.collect(indexed')
native=native.replace('''                    row
                })
                .collect();''','''                    row
                }))?;
            memory.release_bytes(old_indexed_bytes)?;
            let old_location_bytes = std::alloc::Layout::array::<(QuadHandle, RdfLocation)>(locations.capacity()).map_err(|_| StorageError::SizeOverflow)?.size();''')
native=native.replace('let locations = locations','let locations = memory.collect(locations')
native=native.replace('''                })
                .collect();
            (quads, locations)''','''                }))?;
            memory.release_bytes(old_location_bytes)?;
            memory.release_vec(push_to_frozen)?;
            (quads, locations)''')
native=native.replace('named_graphs.reserve(quads.len() + reifiers.len() + annotations.len());','''let additional = quads.len().checked_add(reifiers.len()).and_then(|count| count.checked_add(annotations.len())).ok_or(StorageError::SizeOverflow)?;
            let required = named_graphs.len().checked_add(additional).ok_or(StorageError::SizeOverflow)?;
            memory.reserve(&mut named_graphs, required)?;''')
native=native.replace('named_graphs.extend(quads.iter().filter_map(|q| q.g));','memory.extend(&mut named_graphs, quads.iter().filter_map(|q| q.g))?;')
native=native.replace('named_graphs.extend(reifiers.iter().filter_map(|(_, _, g)| *g));','memory.extend(&mut named_graphs, reifiers.iter().filter_map(|(_, _, g)| *g))?;')
native=native.replace('named_graphs.extend(annotations.iter().filter_map(|(_, _, _, g)| *g));','memory.extend(&mut named_graphs, annotations.iter().filter_map(|(_, _, _, g)| *g))?;')
for expr in ['interner.arena','interner.terms','quads','reifiers','annotations','locations','named_graphs']:
    native=native.replace(expr+'.into_boxed_slice()','memory.boxed_slice('+expr+')?')
native=native.replace('        RdfDataset::from_parts(', '        Ok(RdfDataset::from_parts(')
native=native[:-len(')\n    }')]+'))\n    }' if native.endswith(')\n    }') else native
assert native.endswith('))\n    }')
wrapper='''    fn materialize(self) -> RdfDataset {
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, self.native_buffer_bytes().expect("resident graph buffer layout"));
        self.materialize_with_memory(&mut memory).expect("resident graph materialization allocation failed")
    }
'''
t=t[:start]+wrapper+native+t[end:]
# Avoid a simultaneous immutable/mutable borrow of content_ids at its reserve.
t=t.replace('crate::hash::reserve_map_with_memory(&mut self.content_ids, self.content_ids.len().checked_add(1).ok_or(StorageError::SizeOverflow)?, memory)?;', 'let required = self.content_ids.len().checked_add(1).ok_or(StorageError::SizeOverflow)?;\n            crate::hash::reserve_map_with_memory(&mut self.content_ids, required, memory)?;')
path.write_text(t)
for p in ['crates/rdf-core/src/ir/mod.rs','crates/rdf-core/src/lib.rs']:
    text=(root/p).read_text()
    if p.endswith('mod.rs'): text=text.replace('pub use builder::{','pub use builder::{NativeBuildError, ')
    else:text=text.replace('RESERVED_NAMESPACE, RdfDataset, RdfDatasetBuilder,','RESERVED_NAMESPACE, NativeBuildError, RdfDataset, RdfDatasetBuilder,',1)
    dest=post/p;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(text)
dest=post/'crates/rdf-core/src/ir/validate.rs'
dest.write_text((stage/'graph-native-validation.rs').read_text())
print('native freeze body written')
