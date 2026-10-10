from pathlib import Path
import difflib
stage=Path('.stage/sparql-eval-complete-bounded-workspace'); changes={}
def load(path):
    s=Path(path).read_text(); changes[path]=[s,s]; return s
def save(path,s): changes[path][1]=s
path='crates/sparql-eval/src/contain.rs'; s=load(path); a=s.index('pub(crate) fn call_contained<T>('); b=s.index('\n#[cfg(test)]',a)
body='''pub(crate) fn call_contained<T>(
    kind: &str, iri: &str, what: &str, call: impl FnOnce() -> Result<T, EvalError>,
) -> Result<T, EvalError> {
    call_contained_admitted(kind,iri,what,call,&crate::WorkspaceCapability::resident())
}

/// The original host-call and typed-failure law with before-render admission.
pub(crate) fn call_contained_admitted<T>(
    kind: &str, iri: &str, what: &str, call: impl FnOnce() -> Result<T, EvalError>,
    workspace: &crate::WorkspaceCapability,
) -> Result<T, EvalError> {
    match catch_unwind(AssertUnwindSafe(call)) {
        Ok(result) => result.map_err(EvalError::preserve_function_failure),
        Err(_) => Err(crate::error::NativeDiagnostic::error(
            crate::error::NativeDiagnosticKind::FunctionOperational,
            format_args!("{kind} <{iri}> panicked while {what}"),workspace,
        )),
    }
}
'''
s=s[:a]+body+s[b:]; save(path,s)
path='crates/sparql-eval/src/property_fn.rs'; s=load(path)
a=s.index('pub(crate) fn open_with_workspace_contained('); b=s.index('\n/// Pull one row',a)
s=s[:a]+'''pub(crate) fn open_with_workspace_contained(
    relation: &dyn PropertyFunction, iri: &str, args: &PfArgs<'_>, ceiling: Option<u64>, workspace: &crate::WorkspaceCapability,
) -> Result<Box<dyn PfCursor>, EvalError> {
    let declared = declaration_contained_admitted(iri,"arity",||relation.arity(),workspace)?;
    let supplied = args.arity();
    if declared != supplied {
        return Err(crate::error::NativeDiagnostic::error(crate::error::NativeDiagnosticKind::Function,
            format_args!("property function <{iri}> expects {declared} argument(s), got {supplied}"), workspace));
    }
    crate::contain::call_contained_admitted("property function",iri,"opening an invocation",|| {
        if workspace.is_bounded() { relation.open_admitted(args,ceiling,workspace.clone()) }
        else { relation.open(args,ceiling) }
    },workspace)
}

pub(crate) fn next_with_workspace_contained(cursor: &mut dyn PfCursor, iri: &str,
    workspace: &crate::WorkspaceCapability) -> Result<Option<PfRowOwner>, EvalError> {
    if !workspace.is_bounded() { return next_contained(cursor,iri).map(|row|row.map(PfRowOwner::Resident)); }
    crate::contain::call_contained_admitted("property function",iri,"producing a row",
        || cursor.next_admitted(),workspace).map(|row|row.map(PfRowOwner::Admitted))
}
'''+s[b:]
a=s.index('pub fn next_contained('); b=s.index('\n/// Take `cursor`',a)
s=s[:a]+'''pub fn next_contained(cursor: &mut dyn PfCursor, iri: &str) -> Result<Option<PfRow>, EvalError> {
    crate::contain::call_contained("property function",iri,"producing a row",||cursor.next())
}
'''+s[b:]
a=s.index('pub fn take_work_contained('); b=s.index('\n/// Read `cursor`',a)
s=s[:a]+'''pub fn take_work_contained(cursor: &mut dyn PfCursor, iri: &str) -> Result<u64, EvalError> {
    take_work_contained_admitted(cursor,iri,&crate::WorkspaceCapability::resident())
}
pub(crate) fn take_work_contained_admitted(cursor: &mut dyn PfCursor, iri: &str,
    workspace: &crate::WorkspaceCapability) -> Result<u64, EvalError> {
    declaration_contained_admitted(iri,"work",||cursor.take_work(),workspace)
}
'''+s[b:]
at=s.index('\n/// Read `cursor`\'s [`PfCursor::service_level`]'); s=s[:at]+'''
pub(crate) fn generation_contained_admitted(cursor: &dyn PfCursor, iri: &str,
    workspace: &crate::WorkspaceCapability) -> Result<IndexGeneration,EvalError> {
    declaration_contained_admitted(iri,"index generation",||cursor.generation(),workspace)
}
'''+s[at:]
at=s.index('\n/// Read one of a relation\'s DECLARATIONS'); s=s[:at]+'''
pub(crate) fn service_level_contained_admitted(cursor: &dyn PfCursor, iri: &str,
    workspace: &crate::WorkspaceCapability) -> Result<ServiceLevel,EvalError> {
    declaration_contained_admitted(iri,"service level",||cursor.service_level(),workspace)
}
'''+s[at:]
a=s.index('pub fn declaration_contained<T>('); b=s.index('\n// ---------------------------------------------------------------------------',a)
s=s[:a]+'''pub fn declaration_contained<T>(iri: &str, what: &str, read: impl FnOnce() -> T) -> Result<T,EvalError> {
    declaration_contained_admitted(iri,what,read,&crate::WorkspaceCapability::resident())
}
pub(crate) fn declaration_contained_admitted<T>(iri: &str, what: &str, read: impl FnOnce() -> T,
    workspace: &crate::WorkspaceCapability) -> Result<T,EvalError> {
    crate::contain::declaration_contained_admitted("property function",iri,what,read,workspace)
}
'''+s[b:]
save(path,s)
path='crates/sparql-eval/src/property_fn_eval.rs'; s=load(path); a=s.index('fn eval_call_over<'); b=s.index('\n/// Resolve a call',a); part=s[a:b]
part=part.replace('crate::property_fn::declaration_contained(&call.iri, "arity", || relation.arity())?',
    'crate::property_fn::declaration_contained_admitted(&call.iri, "arity", || relation.arity(), &ctx.growth)?')
part=part.replace('crate::property_fn::declaration_contained(&call.iri, "declared modes", || {',
    'crate::property_fn::declaration_contained_admitted(&call.iri, "declared modes", || {')
part=part.replace('    })??;','    }, &ctx.growth)??;',1)
part=part.replace('generation_contained(&*cursor, &call.iri)?', 'crate::property_fn::generation_contained_admitted(&*cursor, &call.iri, &ctx.growth)?')
part=part.replace('next_with_workspace_contained(&mut *cursor, &call.iri, ctx.growth.is_bounded())?',
    'next_with_workspace_contained(&mut *cursor, &call.iri, &ctx.growth)?')
part=part.replace('take_work_contained(&mut *cursor, &call.iri)?','crate::property_fn::take_work_contained_admitted(&mut *cursor, &call.iri, &ctx.growth)?')
part=part.replace('service_level_contained(&*cursor, &call.iri)?','crate::property_fn::service_level_contained_admitted(&*cursor, &call.iri, &ctx.growth)?')
s=s[:a]+part+s[b:]; save(path,s)
out=stage/'native-property-containment-postimages'; out.mkdir(exist_ok=True); patch=''
for path,(old,new) in changes.items():
    target=out/path; target.parent.mkdir(parents=True,exist_ok=True); target.write_text(new)
    patch+=''.join(difflib.unified_diff(old.splitlines(True),new.splitlines(True),fromfile='a/'+path,tofile='b/'+path))
(stage/'native-property-containment-owner-draft.patch').write_text(patch)
