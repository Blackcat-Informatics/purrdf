from pathlib import Path
import runpy
stage=Path(__file__).parent;root=stage.parent.parent;out=stage/'remote-native-owner-postimages'
for name in ['remote-serializer-assemble.py','remote-sanitize-assemble.py','remote-core-assemble.py','remote-service-assemble.py']:
    runpy.run_path(str(stage/name))
def get(name):
    p=out/name;return (p if p.exists() else root/name).read_text()
def put(name,s):
    p=out/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(s)
name='crates/sparql-eval/src/remote.rs';s=get(name)
s=s.replace('use crate::eval::{EvalCtx, EvaluatedOutcome, Outcome, materialize_solutions};','use crate::eval::{EvalCtx, EvaluatedOutcome, Outcome};\n#[cfg(test)]\nuse crate::eval::materialize_solutions;')
protocol=(stage/'remote-native-protocol.rs').read_text()
protocol=protocol.replace('    storage: ServiceStorage,\n}\nimpl AdmittedResolvedBindings', '    term_owners: crate::AdmittedVec<crate::WorkspaceAllocation>,\n    storage: ServiceStorage,\n}\nimpl AdmittedResolvedBindings')
protocol=protocol.replace('Self { bindings, storage: ServiceStorage::Native(frame) }','Self { bindings, term_owners: crate::AdmittedVec::new(workspace), storage: ServiceStorage::Native(frame) }')
protocol=protocol.replace('if self.storage.is_bounded()', 'if self.storage.is_bounded() || self.term_owners.iter().any(crate::WorkspaceAllocation::is_bounded)')
protocol=protocol.replace('    fn native(error:', '    pub(crate) fn native(error:').replace('    fn into_resident(self)', '    pub(crate) fn into_resident(self)')
idx=protocol.index('    /// Only resident producers')
protocol=protocol[:idx]+'''    pub(crate) fn from_native(bindings: ResolvedBindings,frame:crate::workspace::LexicalFrame,workspace:&crate::WorkspaceCapability)->Self {
        Self{bindings,term_owners:crate::AdmittedVec::new(workspace),storage:ServiceStorage::Native(frame)}
    }
'''+protocol[idx:]
idx=protocol.index('    /// Borrow the original classified error')
protocol=protocol[:idx]+'''    pub(crate) fn certified(error:RemoteError,allocation:crate::WorkspaceAllocation,workspace:&crate::WorkspaceCapability)->Result<Self,EvalError> {
        Self::new(error,ServiceStorage::Certified(allocation),workspace)
    }
'''+protocol[idx:]
idx=protocol.index('    /// Borrow the original invocation error')
protocol=protocol[:idx]+'''    fn into_eval_error(self)->EvalError {
        if self.error.payload.storage.is_bounded() || self.error.payload._control.is_bounded() { return EvalError::RetainedServiceFailure(self); }
        let endpoint=self.endpoint.as_str();
        match self.error.error().clone() {
            RemoteError::Denied(denial)=>EvalError::ServiceDenied(denial),
            RemoteError::HostDenied{endpoint,message}=>EvalError::ServiceHostDenied{endpoint,message},
            RemoteError::HostFault{endpoint,message}=>EvalError::ServiceHostFault{endpoint,message},
            RemoteError::SourceRead(message)=>EvalError::SourceRead(message),
            RemoteError::Unconfigured(message)=>EvalError::ServiceUnconfigured(format!("SERVICE <{endpoint}>: {message}")),
            other=>EvalError::remote(format!("SERVICE <{endpoint}>: {other}")),
        }
    }
'''+protocol[idx:]
idx=s.index('/// A source that resolves a forwarded')
s=s[:idx]+protocol+'\n\n'+s[idx:]
idx=s.index('    fn resolve(&self, request: ServiceRequest');end=s.index('\n}',idx)
s=s[:end]+'''

    /// A concrete upper bound for all fresh live producer allocations, including
    /// returned bindings and dynamic errors, before opaque host dispatch. Native
    /// implementations override `resolve_admitted` and need no certificate.
    fn workspace_certificate(&self,_request:ServiceRequest<'_>)->Option<ServiceWorkspaceCertificate> {None}

    /// Resolve through an original native owner or an honest full peak grant.
    /// A missing opaque certificate fails BEFORE host dispatch in bounded mode.
    /// # Errors
    /// Preserves typed physical refusal and retained invocation failure.
    fn resolve_admitted(&self,request:ServiceRequest<'_>,workspace:crate::WorkspaceCapability)->Result<AdmittedResolvedBindings,ServiceResolutionError> {
        if let Some(trip)=request.stop_trip(){return Err(invocation_error(trip,&workspace)?);}
        if !workspace.is_bounded() {
            let result=self.resolve(request);
            return match result {
                Ok(bindings)=>Ok(AdmittedResolvedBindings::from_native(bindings,crate::workspace::LexicalFrame::new(&workspace),&workspace)),
                Err(error)=>Err(invocation_error(error,&workspace)?),
            };
        }
        let certificate=self.workspace_certificate(request).ok_or(EvalError::WorkspaceUnpriced("opaque SERVICE resolver"))?;
        let allocation=workspace.charge(certificate.peak_bytes)?;
        match self.resolve(request) {
            Ok(bindings)=>Ok(AdmittedResolvedBindings{bindings,term_owners:crate::AdmittedVec::new(&workspace),storage:ServiceStorage::Certified(allocation)}),
            Err(error)=>Err(ServiceResolutionError::Invocation(AdmittedRemoteError::certified(error,allocation,&workspace)?)),
        }
    }
'''+s[end:]
a=s.index('pub(crate) fn invoke_service<');b=s.index('/// The join identity:',a)
s=s[:a]+(stage/'remote-invoke.rs').read_text()+'\n'+s[b:]
a=s.index('fn ingest<D:');b=s.index('/// Reclassify an error',a)
s=s[:a]+(stage/'remote-remap.rs').read_text()+'\n\n'+s[b:]
a=s.index('pub(crate) fn evaluate_in_memory(');b=s.index('#[cfg(test)]\nmod tests',a)
production=(stage/'remote-production.rs').read_text()
production=production.replace('fn invocation_error(', 'pub(crate) fn invocation_error(')
production=production.replace('let (mut query, allocation, live) = parsed.into_parts();\n    let mut frame = LexicalFrame::from_allocation(workspace, allocation, live);', '''struct QueryOwner { query:purrdf_sparql_algebra::Query, frame:LexicalFrame }
    let (query,allocation,live)=parsed.into_parts();
    let mut owner=QueryOwner{query,frame:LexicalFrame::from_allocation(workspace,allocation,live)};''')
production=production.replace('Memory::resume(&mut frame, live);\n        let prepared', 'Memory::resume(&mut owner.frame, live);\n        let prepared')
production=production.replace('plan_query_with_memory(&query,','plan_query_with_memory(&owner.query,')
production=production.replace('if let Some(prepared) = prepared { query = prepared;', 'if let Some(prepared) = prepared { owner.query = prepared;')
production=production.replace('evaluate_query_evaluated(&query,', 'evaluate_query_evaluated(&owner.query,')
production=production.replace('    drop(query);\n    drop(frame);','    // Keep the query carrier alive through context caches and response publication.')
production=production.replace('EvaluationFailure::Evaluation(EvalError::RetainedServiceFailure(error)) =>', 'EvaluationFailure::Evaluation(EvalError::RetainedServiceFailure(error)) if matches!(error.error.error(), RemoteError::Denied(_) | RemoteError::HostDenied { .. } | RemoteError::HostFault { .. }) =>')
helpers='''pub(crate) fn build_error(error:ServiceBuildError,mut frame:crate::workspace::LexicalFrame)->ServiceResolutionError {
    if let Some(error)=frame.take_failure(){return ServiceResolutionError::Operational(error);}
    match error {
        ServiceBuildError::Operational(error)=>ServiceResolutionError::Operational(error),
        ServiceBuildError::Storage(error)=>ServiceResolutionError::Operational(frame.storage_error(error,"SERVICE producer")),
        ServiceBuildError::Remote(error)=>match AdmittedRemoteError::native(error,frame){Ok(error)=>ServiceResolutionError::Invocation(error),Err(error)=>ServiceResolutionError::Operational(error)},
    }
}

pub(crate) fn resident_resolution(result:Result<AdmittedResolvedBindings,ServiceResolutionError>)->Result<ResolvedBindings,RemoteError> {
    match result {
        Ok(response)=>Ok(response.try_into_resident().expect("resident response must retain only resident grants")),
        Err(ServiceResolutionError::Invocation(error))=>Err(error.into_resident().expect("resident invocation error")),
        Err(ServiceResolutionError::Operational(error))=>Err(remote_error_for(error)),
    }
}

'''
s=s[:a]+helpers+production+'\n'+s[b:]
s=s.replace('requested_bytes: layout.size() as u64 ', '').replace(', requested_bytes: layout.size() as u64', '')
s=s.replace('    drop(next);\n','')
# The original static semantic refusals remain, but their owned text is admitted.
s=s.replace('return Err(EvalError::unsupported(\n            ', 'return Err(service_unsupported(\n            ')
s=s.replace('registry would never resolve the call and the answer would be silently wrong",\n        ));','registry would never resolve the call and the answer would be silently wrong",\n            &ctx.growth,\n        ));')
s=s.replace('relation would never be invoked and the answer would be silently wrong",\n        ));','relation would never be invoked and the answer would be silently wrong",\n            &ctx.growth,\n        ));')
idx=s.index('pub(crate) fn eval_service<')
s=s[:idx]+'''fn service_unsupported(message:&'static str,workspace:&crate::WorkspaceCapability)->EvalError {
    if workspace.is_bounded() {
        match crate::RetainedDiagnostic::render(EvalError::UNSUPPORTED_CODE,format_args!("unsupported: {message}"),workspace) {Ok(error)=>EvalError::RetainedDiagnostic(error),Err(error)=>error}
    } else {EvalError::unsupported(message)}
}

'''+s[idx:]
# Property/aggregate reachability already has admitted producer doors.
s=s.replace('crate::property_fn_eval::pattern_reaches_property_function(inner)', 'crate::property_fn_eval::pattern_reaches_property_function_admitted(inner, &ctx.growth)?')
s=s.replace('crate::property_fn_eval::pattern_reaches_custom_aggregate(inner)', 'crate::property_fn_eval::pattern_reaches_custom_aggregate_admitted(inner, &ctx.growth)?')
put(name,s)
name='crates/sparql-eval/src/row_ingest.rs';s=get(name)
s=s.replace('use purrdf_core::{DatasetView, TermValue, TrippedGovernor};','use purrdf_core::{DatasetView, TrippedGovernor};')
s=s.replace('Option<TermValue>', 'Option<crate::WorkspaceTerm>')
a=s.index('                    row[i] =');b=s.index('\n                }',a)
s=s[:a]+'                    row[i] = ctx.intern_workspace_term(value)?;'+s[b:]
put(name,s)
name='crates/sparql-eval/src/remote_http.rs';s=get(name)
a=s.index('    fn resolve(&self, request: ServiceRequest');b=s.index('\n}\n\n#[cfg(test)]',a)
s=s[:a]+'''    fn resolve(&self,request:ServiceRequest<'_>)->Result<ResolvedBindings,RemoteError> {
        crate::remote::resident_resolution(self.resolve_admitted(request,crate::WorkspaceCapability::resident()))
    }
    fn resolve_admitted(&self,request:ServiceRequest<'_>,workspace:crate::WorkspaceCapability)->Result<crate::remote::AdmittedResolvedBindings,crate::remote::ServiceResolutionError> {
        self.resolve_native(request,workspace)
    }
'''+s[b:]
a=s.index('    fn post(&self, request: HttpRequest');b=s.index('\n}',a)
s=s[:b]+'''
    /// Certified peak live bytes for an opaque host exchange, including body
    /// and dynamic error storage. Native `post_admitted` overrides need no bound.
    fn workspace_certificate(&self,_request:HttpRequest<'_>)->Option<crate::remote::ServiceWorkspaceCertificate> {None}
    /// The native/certified body door used by bounded SERVICE production.
    /// # Errors
    /// Refuses an unpriced opaque exchange before calling it.
    fn post_admitted(&self,request:HttpRequest<'_>,workspace:crate::WorkspaceCapability)->Result<AdmittedHttpBody,crate::remote::ServiceResolutionError> {
        use crate::remote::{AdmittedRemoteError,ServiceResolutionError};
        let grant=if workspace.is_bounded(){
            let certificate=self.workspace_certificate(request).ok_or(crate::EvalError::WorkspaceUnpriced("opaque HTTP transport"))?;
            Some(workspace.charge(certificate.peak_bytes)?)
        }else{None};
        match self.post(request){
            Ok(body)=>Ok(AdmittedHttpBody{body,_storage:match grant{Some(grant)=>HttpStorage::Certified(grant),None=>HttpStorage::Native(crate::workspace::LexicalFrame::new(&workspace))}}),
            Err(error)=>Err(ServiceResolutionError::Invocation(match grant{Some(grant)=>AdmittedRemoteError::certified(error,grant,&workspace)?,None=>AdmittedRemoteError::native(error,crate::workspace::LexicalFrame::new(&workspace))?})),
        }
    }
'''+s[b:]
idx=s.index('impl<F> HttpTransport')
s=s[:idx]+(stage/'remote-http-native.rs').read_text()+'\n\n'+s[idx:]
s=s.replace('AdmittedRemoteError,ServiceBuildError,ServiceResolutionError','AdmittedRemoteError,ServiceResolutionError')
put(name,s)
name='crates/sparql-eval/src/engine.rs';s=get(name);s=s.replace('\nfn parse_admitted_query(','\npub(crate) fn parse_admitted_query(',1);put(name,s)
name='crates/sparql-eval/src/error.rs';s=get(name)
s=s.replace('    RetainedDiagnostic(crate::RetainedDiagnostic),','    RetainedDiagnostic(crate::RetainedDiagnostic),\n    /// Federation failure retaining its original producer payload and physical grant.\n    RetainedServiceFailure(crate::remote::RetainedServiceFailure),',1)
s=s.replace('Self::RetainedDiagnostic(_) => None,','Self::RetainedDiagnostic(_) => None,\n            Self::RetainedServiceFailure(error) => if matches!(error.remote_error().error(), crate::remote::RemoteError::SourceRead(_)) {Some("native-sparql-source-read")} else {None},',1)
s=s.replace('Self::RetainedDiagnostic(message) => Some(&message.diagnostic().code),','Self::RetainedDiagnostic(message) => Some(&message.diagnostic().code),\n            Self::RetainedServiceFailure(error) => Some(error.code()),',1)
s=s.replace('Self::RetainedDiagnostic(message) => core::fmt::Display::fmt(message.diagnostic(), f),','Self::RetainedDiagnostic(message) => core::fmt::Display::fmt(message.diagnostic(), f),\n            Self::RetainedServiceFailure(error) => core::fmt::Display::fmt(error, f),',1)
put(name,s)
name='crates/sparql-eval/src/protocol.rs';s=get(name)
idx=s.index('            EvalError::RetainedDiagnostic(diagnostic) =>')
s=s[:idx]+'            EvalError::RetainedServiceFailure(error) => error.failure_code(),\n'+s[idx:]
put(name,s)
name='crates/sparql-eval/src/lib.rs';s=get(name)
s=s.replace('pub use remote::{','pub use remote::{AdmittedResolvedBindings, AdmittedRemoteError, RetainedServiceFailure, ServiceBuildError, ServiceResolutionError, ServiceWorkspaceCertificate, ',1)
s=s.replace('pub use remote_http::{','pub use remote_http::{AdmittedHttpBody, ',1)
put(name,s)
