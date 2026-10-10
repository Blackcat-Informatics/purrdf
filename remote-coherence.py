from pathlib import Path
stage=Path(__file__).parent;out=stage/'remote-native-owner-postimages'
def edit(path,body):
    p=out/path;s=p.read_text();p.write_text(body(s))
def remote(s):
    s=s.replace('purrdf_lex::walk::Dismantle<Continuation = purrdf_sparql_algebra::DropOwner>', 'purrdf_lex::walk::Dismantle')
    s=s.replace('arms: children.into()', 'arms: purrdf_sparql_algebra::Chain::try_from(children).expect("a UNION retains at least two arms")')
    s=s.replace('pending.try_push_admitted(child, |bytes| memory.add_bytes(bytes))', 'pending.try_push_admitted(child, memory)')
    s=s.replace('pending.release_admitted(|bytes| memory.release_bytes(bytes))', 'pending.release_admitted(memory)')
    s=s.replace('crate::aggregate::AggregateRegistry::new()', 'crate::agg_fn::AggregateRegistry::EMPTY')
    s=s.replace('crate::property_fn::PropertyFunctionRegistry::new()', 'crate::property_fn::PropertyFunctionRegistry::EMPTY')
    s=s.replace('owners.reserve(owners.len().checked_add(1).ok_or(EvalError::WorkspaceBoundOverflow)?)?;', 'owners.reserve_next()?;')
    s=s.replace('crate::RetainedDiagnostic::render(EvalError::SERVICE_UNCONFIGURED_CODE, format_args!', 'crate::RetainedDiagnostic::render(EvalError::SERVICE_UNCONFIGURED_CODE, &format_args!')
    s=s.replace('crate::RetainedDiagnostic::render("native-sparql-query-eval", format_args!', 'crate::RetainedDiagnostic::render("native-sparql-query-eval", &format_args!')
    s=s.replace('crate::RetainedDiagnostic::render(EvalError::UNSUPPORTED_CODE,format_args!', 'crate::RetainedDiagnostic::render(EvalError::UNSUPPORTED_CODE,&format_args!')
    s=s.replace('crate::property_fn_eval::pattern_reaches_property_function_admitted(inner, &ctx.growth)?', 'service_reaches(inner, false, &ctx.growth)?')
    s=s.replace('crate::property_fn_eval::pattern_reaches_custom_aggregate_admitted(inner, &ctx.growth)?', 'service_reaches(inner, true, &ctx.growth)?')
    i=s.index('fn service_unsupported(')
    s=s[:i]+'''fn service_reaches(pattern:&GraphPattern,aggregate:bool,workspace:&crate::WorkspaceCapability)->Result<bool,EvalError> {
        let mut frame=crate::workspace::LexicalFrame::new(workspace);
        let mut memory=purrdf_lex::allocation::Memory::new(&mut frame);
        if aggregate {crate::property_fn_eval::pattern_reaches_custom_aggregate_with_memory(pattern,&mut memory)} else {crate::property_fn_eval::pattern_reaches_property_function_with_memory(pattern,&mut memory)}
    }

'''+s[i:]
    # Inline(never) belongs to actual dispatcher, not the newly inserted helper.
    s=s.replace('#[inline(never)]\nfn service_reaches','fn service_reaches')
    s=s.replace('\npub(crate) fn eval_service<','\n#[inline(never)]\npub(crate) fn eval_service<',1)
    # No lexical/remote failure can override the ORIGINAL latched growth refusal.
    s=s.replace('    let post_return_trip = ctx.stop_check();','    let post_return_trip = ctx.stop_check();',1)
    # The raw resident reclassification door is never invoked by native paths.
    s=s.replace('        other => RemoteError::Decode(other.to_string()),','        EvalError::RetainedServiceFailure(error) => RemoteError::Decode(error.to_string()),\n        other => RemoteError::Decode(other.to_string()),',1)
    return s
edit('crates/sparql-eval/src/remote.rs',remote)
def http(s):
    i=s.index('    /// Construct transport buffers')
    s=s[:i]+'''    /// Move a resident body out while refusing to detach any bounded grant.
    /// # Errors
    /// Returns the intact original carrier when its producer is bounded.
    pub fn try_into_resident(self)->Result<Vec<u8>,Self> {
        let bounded=match &self._storage{HttpStorage::Native(frame)=>frame.workspace().is_bounded(),HttpStorage::Certified(grant)=>grant.is_bounded()};
        if bounded {Err(self)} else {Ok(self.body)}
    }
'''+s[i:]
    return s
edit('crates/sparql-eval/src/remote_http.rs',http)
def serializer(s):
    s=s.replace('        self.storage.run(|memory| work.try_push_admitted(value, memory));','        let _ = self.storage.run(|memory| work.try_push_admitted(value, memory));')
    s=s.replace('            s.storage.run(|memory| memory.push(', '            let _ = s.storage.run(|memory| memory.push(')
    s=s.replace('    for (variable, _) in &select_exprs { s.storage.run(', '    for (variable, _) in &select_exprs { let _ = s.storage.run(')
    return s
edit('crates/sparql-algebra/src/serialize.rs',serializer)
# This same existing immutable authored leaf producer is useful to admitted
# external response builders; no new allocation body or raw extraction is added.
root=stage.parent.parent
p=Path('crates/sparql-eval/src/workspace.rs');s=(root/p).read_text();s=s.replace('    pub(crate) fn authored_text(', '    pub fn authored_text(',1);(out/p).parent.mkdir(parents=True,exist_ok=True);(out/p).write_text(s)
