from pathlib import Path
stage=Path(__file__).parent; root=stage.parent.parent; out=stage/'remote-native-owner-postimages'
def put(path,text):
    dest=out/path;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(text)
path=Path('crates/rdf-core/src/ir/term_walk.rs');s=(root/path).read_text()
a=s.index('    pub fn try_fold_owned<T, E>(');b=s.index('\n    ///',a)
old=s[a:b]
header=old[:old.index('        enum Step')]
header=header.replace('        leaf:', '        mut leaf:').replace('        triple:', '        mut triple:')
# Preserve original signature and route its callbacks through the same native body.
new='''    pub fn try_fold_owned<T, E>(self, mut leaf: impl FnMut(Self) -> Result<T, E>, mut triple: impl FnMut(T,T,T) -> Result<T,E>) -> Result<T,E> {
        match self.try_fold_owned_with_memory(|value, _| leaf(value), |s,p,o,_| triple(s,p,o), &mut purrdf_lex::allocation::Memory::new(&mut purrdf_lex::allocation::Resident)) {
            Ok(value) => Ok(value),
            Err(OwnedTermFoldError::Visitor(error)) => Err(error),
            Err(OwnedTermFoldError::Storage(error)) => panic!("resident owned term fold allocation: {error}"),
        }
    }
'''+(stage/'remote-core-fold.rs').read_text()
s=s[:a]+new+s[b:]
idx=s.index('impl TermValue {')
s=s[:idx]+'''/// The original visitor failure or a before-allocation native working-storage refusal.
#[derive(Debug)]
pub enum OwnedTermFoldError<E> {
    /// First leaf or triple callback failure.
    Visitor(E),
    /// Original checked layout, admission or allocator refusal.
    Storage(purrdf_lex::allocation::StorageError),
}
impl<E> From<purrdf_lex::allocation::StorageError> for OwnedTermFoldError<E> {
    fn from(error: purrdf_lex::allocation::StorageError) -> Self { Self::Storage(error) }
}
impl<E: core::fmt::Display> core::fmt::Display for OwnedTermFoldError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { match self { Self::Visitor(error) => error.fmt(f), Self::Storage(error) => error.fmt(f) } }
}
impl<E: std::error::Error + 'static> std::error::Error for OwnedTermFoldError<E> {}

'''+s[idx:]
put(path,s)
for name in ['crates/rdf-core/src/ir/mod.rs','crates/rdf-core/src/lib.rs']:
    path=Path(name);s=(root/path).read_text()
    s=s.replace('TermVisit,','TermVisit, OwnedTermFoldError,',1)
    # ir/mod.rs exports the term_walk family with this marker too.
    if 'OwnedTermFoldError' not in s:
        s=s.replace('pub use term_walk::{','pub use term_walk::{OwnedTermFoldError,',1)
    put(path,s)
path=Path('crates/rdf-core/src/cdt_blank.rs');s=(root/path).read_text()
a=s.index('pub fn rewrite_cdt_terms<');b=s.index('/// Rewrite each `BLANK_NODE_LABEL`',a)
s=s[:a]+'''pub fn rewrite_cdt_terms<'a>(lexical:&'a str,datatype:&str,on_blank:&mut dyn FnMut(&str)->Option<String>,on_iri:&mut dyn FnMut(&str,bool)->Option<String>) -> Cow<'a,str> {
    rewrite_cdt_terms_with_memory(lexical,datatype,&mut |token,memory| { let value=on_blank(token); if let Some(value)=&value { memory.add_bytes(value.capacity())?; } Ok(value) },&mut |iri,only,memory| { let value=on_iri(iri,only); if let Some(value)=&value { memory.add_bytes(value.capacity())?; } Ok(value) }, &mut Memory::new(&mut Resident)).expect("resident composite term rewrite allocation failed")
}

/// Rewrite the same embedded blank and IRI token spans under their original
/// native storage owner. Callback output must be allocated through `memory`.
/// # Errors
/// Returns physical layout, admission or allocator refusal before growth.
pub fn rewrite_cdt_terms_with_memory<'a,S:Admission+?Sized>(
    lexical:&'a str,datatype:&str,
    on_blank:&mut impl FnMut(&str,&mut Memory<'_,S>)->Result<Option<String>,StorageError>,
    on_iri:&mut impl FnMut(&str,bool,&mut Memory<'_,S>)->Result<Option<String>,StorageError>,
    memory:&mut Memory<'_,S>,
) -> Result<Cow<'a,str>,StorageError> {
    if !is_cdt_datatype(datatype) { return Ok(Cow::Borrowed(lexical)); }
    let spans=scan_tokens_with_memory(lexical,memory)?;
    let result=splice_with_memory(lexical,&spans,&mut |token,memory| match token { TokenKind::Blank(label)=>on_blank(label,memory),TokenKind::Iri{iri,iri_only}=>on_iri(iri,*iri_only,memory) },memory)?;
    release_spans(spans,memory)?;
    Ok(result)
}

'''+s[b:]
a=s.index('pub fn rewrite_cdt_blank_terms<');b=s.index('/// Every `(label, scope)`',a)
s=s[:a]+'''pub fn rewrite_cdt_blank_terms<'a>(lexical:&'a str,datatype:&str,rewrite:&mut dyn FnMut(&str)->Option<String>)->Cow<'a,str> {
    rewrite_cdt_terms(lexical,datatype,rewrite,&mut |_,_|None)
}

/// Rename the same embedded composite blanks through the original native scanner.
/// # Errors
/// Returns physical refusal; every returned owned byte retains caller `memory`.
pub fn rewrite_cdt_blank_terms_with_memory<'a,S:Admission+?Sized>(lexical:&'a str,datatype:&str,rewrite:&mut impl FnMut(&str,&mut Memory<'_,S>)->Result<Option<String>,StorageError>,memory:&mut Memory<'_,S>)->Result<Cow<'a,str>,StorageError> {
    rewrite_cdt_terms_with_memory(lexical,datatype,rewrite,&mut |_,_,_|Ok(None),memory)
}

'''+s[b:]
put(path,s)
