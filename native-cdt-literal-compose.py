from pathlib import Path
import difflib
root=Path(__file__).resolve().parents[2];stage=Path(__file__).resolve().parent
images={}
def read(p):
    text=(root/p).read_text();images[p]=[text,text];return text
def save(p,text):images[p][1]=text
p='crates/cdt/src/memory.rs';s=read(p)
s=s.replace('        Ok(Box::new(value))','        purrdf_lex::allocation::try_boxed(value).map_err(|_| StorageError::AllocationFailed)')
s+='\n'+(stage/'native-cdt-memory-bridge.rs').read_text().split('\n',3)[3]
save(p,s)
p='crates/cdt/src/literal.rs';s=read(p)
a=s.index('pub fn parse_literal(lexical:');b=s.find('\n#[cfg(test)]',a)
if b<0:b=len(s)
newbody='''pub fn parse_literal(lexical: &str, datatype: &str) -> LiteralValue {
    let mut storage = crate::memory::Resident;
    let mut memory = crate::memory::Memory::new(&mut storage);
    let result = resolve_literal_with_memory(lexical, datatype, &mut memory)
        .expect("resident literal producer storage");
    match result {
        NativeLiteralValue::Xsd(value) => LiteralValue::Xsd(value),
        NativeLiteralValue::Cdt(value) => LiteralValue::Cdt(value),
        NativeLiteralValue::IllTyped => LiteralValue::IllTyped {
            datatype: memory.string(datatype).expect("resident datatype text"),
            lexical: memory.string(lexical).expect("resident lexical text"),
        },
        NativeLiteralValue::Opaque => LiteralValue::Opaque,
    }
}

/// Native literal denotation. Type-only refusals need no copied diagnostic pair;
/// the caller still borrows the literal that was refused.
pub(crate) enum NativeLiteralValue {
    Xsd(XsdValue),
    Cdt(CdtValue),
    IllTyped,
    Opaque,
}

/// The one composite/XSD dispatch law, with native producer ownership.
pub(crate) fn resolve_literal_with_memory(
    lexical: &str,
    datatype: &str,
    memory: &mut crate::memory::Memory<'_>,
) -> Result<NativeLiteralValue, crate::memory::StorageError> {
    if let Some(composite) = CdtDatatype::from_iri(datatype) {
        return Ok(match crate::memory::parse_cdt_in_memory(lexical, composite, memory)? {
            Some(value) => NativeLiteralValue::Cdt(value),
            None => NativeLiteralValue::IllTyped,
        });
    }
    let Some(datatype) = purrdf_xsd::XsdDatatype::from_iri(datatype) else { return Ok(NativeLiteralValue::Opaque); };
    Ok(match purrdf_xsd::value::try_parse_with_memory(lexical, datatype, false, memory)? {
        purrdf_xsd::value::ParsedValue::Value(value) => NativeLiteralValue::Xsd(value),
        purrdf_xsd::value::ParsedValue::Invalid(_) => NativeLiteralValue::IllTyped,
    })
}

/// Destroy a natively produced XSD payload before releasing its original layouts.
pub(crate) fn release_xsd_with_memory(value: XsdValue, memory: &mut crate::memory::Memory<'_>) -> Result<(), crate::memory::StorageError> {
    let bytes = value.owned_heap_bytes().ok_or(crate::memory::StorageError::SizeOverflow)?;
    drop(value);
    memory.release_bytes(bytes)
}

'''
s=s[:a]+newbody+s[b:]
# The original parser's import is now called through its native shared home.
s=s.replace('use crate::{CdtDatatype, CdtValue, parse_cdt};','use crate::{CdtDatatype, CdtValue};')
s=s.replace('use alloc::string::{String, ToString};','use alloc::string::String;').replace('use crate::parse::parse_cdt;\n','')
save(p,s)
patch=''.join(''.join(difflib.unified_diff(old.splitlines(True),new.splitlines(True),fromfile='a/'+path,tofile='b/'+path)) for path,(old,new) in images.items())
(stage/'native-cdt-literal-owner-draft.patch').write_text(patch)
for path,(_,new) in images.items(): (stage/(Path(path).stem+'-native-cdt-postimage.rs')).write_text(new)
