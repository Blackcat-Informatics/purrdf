from pathlib import Path
import difflib

root=Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
stage=root/'.stage/sparql-eval-complete-bounded-workspace'
post=stage/'graph-native-postimages'
post.mkdir(exist_ok=True)
changes={}
def read(path):
    original=(root/path).read_text()
    changes[path]=[original,original]
    return original
def save(path,text): changes[path][1]=text
def snip(name): return (stage/name).read_text()
def replace_body(text,head,replacement):
    start=text.index(head)
    brace=text.index('{',start)
    depth=1; end=brace+1
    while depth:
        if text[end]=='{': depth+=1
        elif text[end]=='}': depth-=1
        end+=1
    return text[:start]+replacement+text[end:]

p='crates/lex/src/allocation.rs'; t=read(p)
t=t.replace('    /// Append one value after admitting',snip('graph-native-box.rs')+'\n    /// Append one value after admitting',1)
save(p,t)
p='crates/rdf-core/src/hash.rs'; t=read(p)
t+='\n'+snip('graph-native-hash.rs')+'\n'+snip('graph-native-map.rs')
save(p,t)
p='crates/rdf-core/src/ir/term.rs'; t=read(p)
t=replace_body(t,'pub(crate) fn push_arena_str(','''pub(crate) fn push_arena_str(arena: &mut Vec<u8>, s: &str) -> StrRange {
    use purrdf_lex::allocation::{Memory, Resident};
    let mut resident = Resident;
    let mut memory = Memory::resume(&mut resident, arena.capacity());
    push_arena_str_with_memory(arena, s, &mut memory).expect("resident term arena allocation failed")
}

/// The original arena producer, admitting growth before any allocation.
pub(crate) fn push_arena_str_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
    arena: &mut Vec<u8>, s: &str, memory: &mut purrdf_lex::allocation::Memory<'_, S>,
) -> Result<StrRange, purrdf_lex::allocation::StorageError> {
    use purrdf_lex::allocation::StorageError;
    let offset = u32::try_from(arena.len()).map_err(|_| StorageError::SizeOverflow)?;
    let len = u32::try_from(s.len()).map_err(|_| StorageError::SizeOverflow)?;
    offset.checked_add(len).ok_or(StorageError::SizeOverflow)?;
    memory.extend(arena, s.bytes())?;
    Ok(StrRange { offset, len })
}''')
t=replace_body(t,'pub(crate) fn interned_language(','''pub(crate) fn interned_language(tag: &str) -> std::borrow::Cow<'_, str> {
    interned_language_with_memory(tag, &mut purrdf_lex::allocation::Memory::new(&mut purrdf_lex::allocation::Resident))
        .expect("resident language fold allocation failed")
}

/// The shared identity fold, with its actual destination admitted at birth.
pub(crate) fn interned_language_with_memory<'a, S: purrdf_lex::allocation::Admission + ?Sized>(
    tag: &'a str, memory: &mut purrdf_lex::allocation::Memory<'_, S>,
) -> Result<std::borrow::Cow<'a, str>, purrdf_lex::allocation::StorageError> {
    if purrdf_iri::langtag::is_identity_folded(tag) { Ok(std::borrow::Cow::Borrowed(tag)) }
    else { Ok(std::borrow::Cow::Owned(purrdf_iri::langtag::identity_fold_with_memory(tag, memory)?)) }
}''')
save(p,t)
p='crates/iri/src/langtag.rs';t=read(p)
t=replace_body(t,'pub fn identity_fold(','''pub fn identity_fold(tag: &str) -> String {
    identity_fold_with_memory(tag, &mut purrdf_lex::allocation::Memory::new(&mut purrdf_lex::allocation::Resident))
        .expect("resident language identity fold allocation failed")
}

/// Fold a language identity into an originally admitted destination.
///
/// # Errors
/// Returns native layout, admission or allocator refusal.
pub fn identity_fold_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
    tag: &str, memory: &mut purrdf_lex::allocation::Memory<'_, S>,
) -> Result<String, purrdf_lex::allocation::StorageError> {
    let mut text = memory.string(tag)?;
    text.make_ascii_lowercase();
    Ok(text)
}''')
save(p,t)
p='crates/rdf-core/src/cdt_blank.rs';t=read(p)
t=replace_body(t,'pub fn cdt_embedded_blanks(','''pub fn cdt_embedded_blanks(lexical: &str, datatype: &str) -> Vec<(String, BlankScope)> {
    cdt_embedded_blanks_with_memory(lexical, datatype, &mut Memory::new(&mut Resident))
        .expect("resident composite blank discovery allocation failed")
}

/// Discover the same embedded blank identities through the original scanner,
/// preserving their original buffer grant until the returned labels die.
///
/// # Errors
/// Returns physical layout, admission or allocator refusal.
pub fn cdt_embedded_blanks_with_memory<S: Admission + ?Sized>(
    lexical: &str, datatype: &str, memory: &mut Memory<'_, S>,
) -> Result<Vec<(String, BlankScope)>, StorageError> {
    if !is_cdt_datatype(datatype) { return Ok(Vec::new()); }
    let spans = scan_tokens_with_memory(lexical, memory)?;
    let mut blanks = Vec::new();
    for span in &spans {
        if let TokenKind::Blank(token) = &span.kind {
            let (label, scope) = decode_blank_label_with_memory(token, LabelAlphabet::BlankNodeLabel, memory)?;
            let label = match label { Cow::Owned(label) => label, Cow::Borrowed(label) => memory.string(label)? };
            memory.push(&mut blanks, (label, scope))?;
        }
    }
    release_spans(spans, memory)?;
    Ok(blanks)
}''')
save(p,t)
p='crates/rdf-core/src/term_fixture.rs';t=read(p)
t=replace_body(t,'pub fn intern_value(','''pub fn intern_value(builder: &mut RdfDatasetBuilder, value: &TermValue) -> TermId {
    use purrdf_lex::allocation::{Memory, Resident};
    let mut resident = Resident;
    let mut memory = Memory::resume(&mut resident, builder.interner_buffer_bytes().expect("resident interner layout"));
    intern_value_with_memory(builder, value, &mut memory).expect("resident term interning allocation failed")
}''')
t+='\n'+snip('graph-native-term-fixture.rs')
save(p,t)
p='crates/rdf-core/src/ir/builder.rs';t=read(p)
t=t.replace('use hashbrown::HashTable;','use hashbrown::HashTable;\nuse purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};')
t=t.replace('arena_str, push_arena_str,','arena_str, push_arena_str_with_memory,')
# The resident wrappers and bounded callers share the same original insertion body.
start=t.index('fn store_once<T:');end=t.index('\n/// Reserve room',start)
old=t[start:end]
native=old.replace('fn store_once<T: Hash + Eq>','fn store_once_with_memory<T: Hash + Eq, S: Admission + ?Sized>')
native=native.replace('value: T) -> u32','value: T, memory: &mut Memory<\'_, S>) -> Result<u32, StorageError>')
native=native.replace('return i;','return Ok(i);')
native=native.replace('u32::try_from(vec.len()).expect("interner table exceeds u32::MAX entries")','u32::try_from(vec.len()).map_err(|_| StorageError::SizeOverflow)?')
native=native.replace('    vec.push(value);','    memory.push(vec, value)?;\n    crate::hash::reserve_table_with_memory(table, vec.len(), |&i| hash_of(&vec[i as usize]), memory)?;')
native=native.replace('\n    i\n','\n    Ok(i)\n')
wrapper='''fn store_once<T: Hash + Eq>(vec: &mut Vec<T>, table: &mut HashTable<u32>, value: T) -> u32 {
    let mut resident = Resident;
    let bytes = std::alloc::Layout::array::<T>(vec.capacity()).expect("resident array layout").size()
        .checked_add(crate::hash::hash_table_allocation_bound::<u32>(table.capacity()).expect("resident table layout"))
        .expect("resident store-once layout");
    let mut memory = Memory::resume(&mut resident, bytes);
    store_once_with_memory(vec, table, value, &mut memory).expect("resident store-once allocation failed")
}
'''
t=t[:start]+wrapper+native+t[end:]
t=replace_body(t,'    fn push_str(','''    fn push_str_with_memory<S: Admission + ?Sized>(&mut self, s: &str, memory: &mut Memory<'_, S>) -> Result<StrRange, StorageError> {
        push_arena_str_with_memory(&mut self.arena, s, memory)
    }

    fn buffer_bytes(&self) -> Result<usize, StorageError> {
        let terms = std::alloc::Layout::array::<InternedTerm>(self.terms.capacity()).map_err(|_| StorageError::SizeOverflow)?.size();
        let index = crate::hash::hash_table_allocation_bound::<u32>(self.index.capacity()).ok_or(StorageError::SizeOverflow)?;
        let content = crate::hash::hash_table_allocation_bound::<(TermId, Blake3ContentId)>(self.content_ids.capacity()).ok_or(StorageError::SizeOverflow)?;
        [self.arena.capacity(), terms, index, content, self.relative_iri.as_ref().map_or(0, |(iri, _)| iri.capacity())]
            .into_iter().try_fold(0usize, |total, bytes| total.checked_add(bytes).ok_or(StorageError::SizeOverflow))
    }''')
start=t.index('    fn intern(&mut');end=t.index('\n    /// The decoded content id',start)
original=t[start:end]
native=original.replace('fn intern(&mut self, lookup: TermLookup<\'_>) -> TermId','fn intern_with_memory<S: Admission + ?Sized>(&mut self, lookup: TermLookup<\'_>, memory: &mut Memory<\'_, S>) -> Result<TermId, StorageError>')
native=native.replace('return TermId::from_index(i);','return Ok(TermId::from_index(i));')
native=native.replace('u32::try_from(self.terms.len()).expect("term table exceeds u32::MAX entries")','u32::try_from(self.terms.len()).map_err(|_| StorageError::SizeOverflow)?')
native=native.replace('Some((iri.to_owned(), err))','Some((memory.string(iri)?, err))')
for expr in ['iri','label','lexical']:
    native=native.replace('self.push_str('+expr+')','self.push_str_with_memory('+expr+', memory)?')
native=native.replace('let language = language.map(|l| self.push_str(l));','let language = language.map(|l| self.push_str_with_memory(l, memory)).transpose()?;')
native=native.replace('        self.terms.push(term);','        memory.push(&mut self.terms, term)?;')
native=native.replace('            self.content_ids.insert','            crate::hash::reserve_map_with_memory(&mut self.content_ids, self.content_ids.len().checked_add(1).ok_or(StorageError::SizeOverflow)?, memory)?;\n            self.content_ids.insert')
native=native.replace('        self.index\n            .insert_unique','        crate::hash::reserve_table_with_memory(&mut self.index, terms.len(), |&i| hash_stored_value(arena, &terms[i as usize]), memory)?;\n        self.index\n            .insert_unique')
native=native.replace('\n        TermId::from_index(i)\n','\n        Ok(TermId::from_index(i))\n')
wrapper='''    fn intern(&mut self, lookup: TermLookup<'_>) -> TermId {
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, self.buffer_bytes().expect("resident interner layout"));
        self.intern_with_memory(lookup, &mut memory).expect("resident term interning allocation failed")
    }
'''
t=t[:start]+wrapper+native+t[end:]
t=t.replace('impl RdfDatasetBuilder {','impl RdfDatasetBuilder {\n'+snip('graph-native-builder-methods.rs'),1)
# Replace the resident literal body with the same native identity-policy producer.
head='    pub(crate) fn intern_literal_parts('
start=t.index(head);brace=t.index(' {',start); depth=1;end=brace+2
while depth:
    if t[end]=='{': depth+=1
    elif t[end]=='}': depth-=1
    end+=1
signature=t[start:brace]
t=t[:start]+signature+''' {
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, self.interner.buffer_bytes().expect("resident interner layout"));
        self.intern_literal_parts_with_memory(lexical, datatype, language, direction, &mut memory)
            .expect("resident literal interning allocation failed")
    }'''+t[end:]
save(p,t)
for path,(original,updated) in changes.items():
    destination=post/path
    destination.parent.mkdir(parents=True,exist_ok=True)
    destination.write_text(updated)
patch=''.join(''.join(difflib.unified_diff(original.splitlines(True),updated.splitlines(True),fromfile='a/'+path,tofile='b/'+path)) for path,(original,updated) in changes.items())
(stage/'graph-native-buffers-owner-draft.patch').write_text(patch)
print(len(patch.splitlines()),'patch lines')
