from pathlib import Path
import re

root=Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
stage=root/'.stage/sparql-eval-complete-bounded-workspace'
original=(root/'crates/rdf-core/src/ir/validate.rs').read_text()
names=['validate','require_absolute_iris','validate_triple_terms','nesting_limit','check_acyclic','check_id_in_range','require_subject','require_subject_kind','require_iri_predicate','require_predicate_kind','require_graph_name','require_graph_kind','diag']
def function(name):
    start=re.search(r'(?:pub\(crate\) )?fn '+name+r'\(',original).start()
    opening=original.index('{',start)
    depth=1;end=opening+1
    while depth:
        if original[end]=='{':depth+=1
        elif original[end]=='}':depth-=1
        end+=1
    return original[start:end],start,end
def closing(code,opening):
    depth=1;end=opening+1
    while depth:
        if code[end]=='(':depth+=1
        elif code[end]==')':depth-=1
        end+=1
    return end-1
def calls(code):
    edits=[]
    for match in re.finditer(r'\b('+'|'.join(names)+r')\(',code):
        opening=match.end()-1;end=closing(code,opening)
        before=code[opening+1:end].rstrip()
        edits.append((end,end,(' ' if before.endswith(',') else ', ' if before else '')+ 'memory'))
        edits.append((match.start(),opening,match.group(1)+'_with_memory'))
    for start,end,value in sorted(edits,reverse=True):code=code[:start]+value+code[end:]
    return code
native=[]
for name in names:
    code,_,_=function(name)
    if name=='diag':
        native.append('''fn diag_with_memory<S: Admission + ?Sized>(code: &str, message: impl core::fmt::Display, memory: &mut Memory<'_, S>) -> NativeBuildError {
    match RdfDiagnostic::try_error_with_memory(code, &message, memory) {
        Ok(diagnostic) => NativeBuildError::Diagnostic(diagnostic),
        Err(error) => NativeBuildError::Storage(error),
    }
}''')
        continue
    opening=code.index('{')
    signature=code[:opening]
    body=code[opening:]
    signature=signature.replace('fn '+name+'(', 'fn '+name+'_with_memory<S: Admission + ?Sized>(')
    signature=signature.replace('ctx: impl FnOnce() -> String','ctx: &(impl core::fmt::Display + ?Sized)')
    opening=signature.index('>(')+1
    last=closing(signature,opening)
    before=signature[:last].rstrip()
    empty=signature[opening+1:last].strip()==''
    signature=before+(' ' if before.endswith(',') else '' if empty else ', ')+"memory: &mut Memory<'_, S>"+signature[last:]
    signature=signature.replace('RdfDiagnostic','NativeBuildError')
    body=body.replace('ctx()', 'ctx')
    body=body.replace('message.to_owned()', 'message')
    body=re.sub(r'\|\|\s*\{\s*format!\("named graph declaration #\{i\}"\)\s*\}', '&format_args!("named graph declaration #{i}")',body)
    body=re.sub(r'\|\|\s*format!', '&format_args!',body)
    body=re.sub(r'\|\|\s*quad_ref_ctx\((.*?)\)',r'&QuadContext(\1)',body)
    body=body.replace('format!(', 'format_args!(')
    body=calls(body)
    if name=='validate_triple_terms':
        body=body.replace('let mut state = vec![VisitState::Unvisited; term_count];','let mut state = memory.collect(core::iter::repeat_n(VisitState::Unvisited, term_count))?;\n    let result = (|| {')
        body=body.replace('    Ok(())\n}', '    Ok(())\n    })();\n    memory.release_vec(state)?;\n    result\n}')
    native.append(signature+body)

wrappers={
    'validate':'''pub(crate) fn validate(builder: &RdfDatasetBuilder) -> Result<(), RdfDiagnostic> {
    resident_validation(validate_with_memory(builder, &mut Memory::new(&mut Resident)))
}''',
    'require_subject_kind':'''fn require_subject_kind(kind: Kind, ctx: impl FnOnce() -> String, triple_reason: &str) -> Result<(), RdfDiagnostic> {
    resident_validation(require_subject_kind_with_memory(kind, &LazyContext(core::cell::RefCell::new(Some(ctx))), triple_reason, &mut Memory::new(&mut Resident)))
}''',
    'require_predicate_kind':'''fn require_predicate_kind(kind: Kind, ctx: impl FnOnce() -> String) -> Result<(), RdfDiagnostic> {
    resident_validation(require_predicate_kind_with_memory(kind, &LazyContext(core::cell::RefCell::new(Some(ctx))), &mut Memory::new(&mut Resident)))
}''',
    'require_graph_kind':'''fn require_graph_kind(kind: Kind, ctx: impl FnOnce() -> String) -> Result<(), RdfDiagnostic> {
    resident_validation(require_graph_kind_with_memory(kind, &LazyContext(core::cell::RefCell::new(Some(ctx))), &mut Memory::new(&mut Resident)))
}''',
    'nesting_limit':'''fn nesting_limit() -> RdfDiagnostic {
    match nesting_limit_with_memory(&mut Memory::new(&mut Resident)) {
        NativeBuildError::Diagnostic(diagnostic) => diagnostic,
        NativeBuildError::Storage(error) => panic!("resident RDF validation allocation failed: {error}"),
    }
}''',
    'diag':'''fn diag(code: &str, message: impl Into<String>) -> RdfDiagnostic { RdfDiagnostic::error(code, message) }'''
}
# Helpers called only from the builder validation are replaced by native bodies,
# rather than leaving an obsolete alternative. The record boundary keeps its
# existing resident wrappers over the same native positional rule bodies.
edits=[]
for name in names:
    _,start,end=function(name)
    edits.append((start,end,wrappers.get(name,'')))
text=original
for start,end,value in sorted(edits,reverse=True):text=text[:start]+value+text[end:]
text=text.replace('use crate::RdfDiagnostic;','use crate::RdfDiagnostic;\nuse purrdf_lex::allocation::{Admission, Memory, Resident};\nuse super::builder::NativeBuildError;')
text=re.sub(r'fn quad_ref_ctx\(index: usize, position: &str\) -> String \{\s*format!\("quad #\{index\} \{position\}"\)\s*\}', '', text)
support='''
fn resident_validation<T>(result: Result<T, NativeBuildError>) -> Result<T, RdfDiagnostic> {
    match result {
        Ok(value) => Ok(value),
        Err(NativeBuildError::Diagnostic(diagnostic)) => Err(diagnostic),
        Err(NativeBuildError::Storage(error)) => panic!("resident RDF validation allocation failed: {error}"),
    }
}

struct LazyContext<F>(core::cell::RefCell<Option<F>>);
impl<F: FnOnce() -> String> core::fmt::Display for LazyContext<F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let context = self.0.borrow_mut().take().expect("a positional context is rendered once");
        f.write_str(&context())
    }
}
struct QuadContext<'a>(usize, &'a str);
impl core::fmt::Display for QuadContext<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "quad #{} {}", self.0, self.1)
    }
}
'''
text=text.replace('#[cfg(test)]',support+'\n'+'\n\n'.join(native)+'\n\n#[cfg(test)]',1)
(stage/'graph-native-validation.rs').write_text(text)
print('native validation body written')
