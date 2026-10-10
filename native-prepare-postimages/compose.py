from pathlib import Path
from ingress import rewrite_ingresses
stage=Path(__file__).parent
root=stage.parents[2]
p=stage/'crates/sparql-eval/src/engine.rs'
s=(root/'crates/sparql-eval/src/engine.rs').read_text()
marker='/// A parsed, ready-to-evaluate query'
s=s.replace(marker,(stage/'key_owner.rs').read_text()+'\n'+marker,1)
s=s.replace('entries: BoundedCache<Arc<[u8]>, Arc<PreparedQuery>>,','entries: BoundedCache<PreparedCacheKey, PreparedHandle>,')
s=s.replace('key_scratch: Vec<u8>,','key_scratch: PlanKeyScratch,').replace('key_scratch: Vec::new(),','key_scratch: PlanKeyScratch::default(),')
s=s.replace('enum EvaluationFailure {','pub(crate) enum EvaluationFailure {').replace('    fn native_diagnostic(','    pub(crate) fn native_diagnostic(')
s=s.replace('    pub(crate) plan: crate::plan::PlanCache,','''    pub(crate) plan: crate::plan::PlanCache,
    certified_bytes: Option<usize>,
    // Parsed/rewrite containers die before their original allocation grant.
    _allocation: Option<crate::WorkspaceAllocation>,''',1)
s=s.replace('            plan: crate::plan::PlanCache::default(),','''            plan: crate::plan::PlanCache::default(),
            certified_bytes: None,
            _allocation: None,''',1)
needle='''    pub fn retained_size_bytes(&self) -> usize {
        plan_payload_bytes('''
s=s.replace(needle,'''    pub fn retained_size_bytes(&self) -> usize {
        if let Some(bytes) = self.certified_bytes { return bytes; }
        plan_payload_bytes(''',1)
a=s.index('    fn prepare_keyed(')
b=s.index('\n}\n\n/// Every component',a)
signature=s[a:s.index('        // The key is built',a)]
wrapper=signature+'''        let workspace = crate::WorkspaceCapability::resident();
        let prepared = self.prepare_keyed_admitted(query, base_iri, options,
            relations, aggregates, fingerprint, agg_fingerprint, parameters, exempt, &workspace)
            .map_err(EvaluationFailure::into_diagnostic)?;
        prepared.caller_arc().ok_or_else(|| RdfDiagnostic::error(
            "native-sparql-algebra", "a resident cache key returned a native allocation owner",
        ))
    }

    pub(crate) fn prepare_execution_plan_admitted(
        &mut self, query: &str, base_iri: Option<&str>,
        env: &crate::extension_env::ExtensionEnv, parameters: &[&str], exempt: &[&str],
        workspace: &crate::WorkspaceCapability,
    ) -> Result<PreparedHandle, EvaluationFailure> {
        self.prepare_keyed_admitted(query, base_iri, env.parser_options(), env.relations(),
            env.aggregates(), env.relations_fingerprint(), env.aggregates_fingerprint(),
            parameters, exempt, workspace)
    }

'''+(stage/'prepare_body.rs').read_text()
s=s[:a]+wrapper+s[b:]
a=s.index('    /// Append this key\'s bytes')
b=s.index('\n}\n\n/// The native, RDF-1.2-first',a)
s=s[:a]+(stage/'key_write.rs').read_text()+s[b:]
marker='fn plan_payload_bytes('
s=s.replace(marker,(stage/'parse_owner.rs').read_text()+'\n'+marker,1)
a=s.index('fn changed_source_schema(')
b=s.index('/// Admit `query`:',a)
s=s[:a]+(stage/'schema_owner.rs').read_text()+'\n'+s[b:]
marker='    /// Bind every SPARQL-bodied function in `functions`'
s=s.replace(marker,(stage/'request_owner.rs').read_text()+'\n'+marker,1)
# The direct operational doors already have the reporting account at ingress.
start=s.index('    pub fn query_fallible_view')
end=s.index('    /// Parse and execute one request under caller-supplied execution governors.',start)
part=s[start:end].replace('self.prepare_request(', 'self.prepare_request_admitted(')
part=part.replace('            &substitutions.parameters,\n', '            &substitutions.parameters,\n            &reporting.capability(),\n')
part=part.replace('Err(diagnostic) => return finish_fallible_query(dataset, Err(diagnostic.into()), None),','Err(error) => return finish_fallible_query(dataset, Err(error), Some(&reporting.execution())),')
s=s[:start]+part+s[end:]
start=s.index('    pub fn query_governed_fallible_view')
end=s.index('    /// [`Self::query_governed_fallible_view`] with a federation',start)
part=s[start:end].replace('self.prepare_request(', 'self.prepare_request_admitted(')
part=part.replace('            &admitted.parameters,\n', '            &admitted.parameters,\n            &reporting.capability(),\n')
part=part.replace('Err(diagnostic) => {\n                return finish_governed_fallible_query(dataset, &state, Err(diagnostic.into()), None);','Err(error) => {\n                return finish_governed_fallible_query(dataset, &state, Err(error), Some(&reporting.execution()));')
s=s[:start]+part+s[end:]
start=s.index('    fn explain_fallible_for')
end=s.index('    /// The resident diagnostic projection',start)
part=s[start:end].replace('self.prepare_for(query_text, base_iri, options.env)', 'self.prepare_for_admitted(query_text, base_iri, options.env, &_reporting.capability())')
part=part.replace('Err(diagnostic) => return finish_fallible_read(dataset, Err(diagnostic.into()), Some(&_reporting.execution())),','Err(error) => return finish_fallible_read(dataset, Err(error), Some(&_reporting.execution())),')
s=s[:start]+part+s[end:]
marker='impl EvaluationFailure {'
s=s.replace(marker,'''impl From<crate::substitute::GroundFailure> for EvaluationFailure {
    fn from(error: crate::substitute::GroundFailure) -> Self {
        match error {
            crate::substitute::GroundFailure::Diagnostic(error) => Self::RetainedDiagnostic(error),
            crate::substitute::GroundFailure::Operational(error) => Self::Evaluation(error),
        }
    }
}

'''+marker,1)
s=rewrite_ingresses(s)
s=s.replace('fn admit_structure(query:', (stage/'check_owner.rs').read_text()+'\nfn admit_structure(query:',1)
# Existing resident callers project the same admitted verification body.
a=s.index('fn check_prepared_registries_unchanged(')
b=s.index('\n/// The **plan-only**',a)
s=s[:a]+'''fn check_prepared_registries_unchanged(
    prepared: &PreparedQuery, options: QueryOptions<'_>, workspace: &crate::WorkspaceCapability,
) -> Result<(), EvaluationFailure> {
    let mut frame = crate::workspace::LexicalFrame::new(workspace);
    let mut memory = purrdf_lex::allocation::Memory::new(&mut frame);
    check_prepared_registries_with_memory(prepared, options, &mut memory)
}
'''+s[b:]
a=s.index('fn check_plan_matches_registries(')
b=s.index('\n/// The [`RelationIdentity`]',a)
s=s[:a]+'''fn check_plan_matches_registries(
    prepared: &PreparedQuery, options: QueryOptions<'_>, parameters: &crate::DetHashSet<Variable>,
) -> Result<(), RdfDiagnostic> {
    let names: Vec<&str> = parameters.iter().map(Variable::as_str).collect();
    let workspace = crate::WorkspaceCapability::resident();
    let mut frame = crate::workspace::LexicalFrame::new(&workspace);
    let mut memory = purrdf_lex::allocation::Memory::new(&mut frame);
    check_plan_matches_registries_with_memory(prepared, options, &names, &mut memory)
        .map_err(EvaluationFailure::into_diagnostic)
}
'''+s[b:]
p.write_text(s)
p=stage/'crates/sparql-eval/src/workspace.rs'
s=(root/'crates/sparql-eval/src/workspace.rs').read_text()
needle='''    pub(crate) fn new_admitted(
        value: T,'''
addition='''    /// Borrow the resident public handle without copying its payload. Native
    /// controls deliberately expose no Arc conversion or unadmitted copy.
    pub(crate) fn caller_arc(&self) -> Option<std::sync::Arc<T>> {
        match &self.0 {
            SharedWorkspaceStorage::Caller(value) => Some(value.clone()),
            SharedWorkspaceStorage::Native(_) => None,
        }
    }

    pub(crate) fn control_bytes() -> usize {
        Shared::<SharedWorkspacePayload<T>>::allocation_layout().size()
    }

'''
s=s.replace(needle,addition+needle,1)
marker='trait GrowthAdmission:'
addition='''impl purrdf_sparql_algebra::parser::ParserAdmission for LexicalFrame {
    fn text(&mut self, text: &dyn core::fmt::Display) -> Result<purrdf_lex::allocation::SharedText, purrdf_lex::allocation::StorageError> {
        if self.failure.is_some() { return Err(purrdf_lex::allocation::StorageError::AdmissionFailed); }
        self.workspace.authored_text(text).map_err(|error| {
            self.failure = Some(error);
            purrdf_lex::allocation::StorageError::AdmissionFailed
        })
    }
}

'''
s=s.replace(marker,addition+marker,1)
p.write_text(s)
p=stage/'crates/sparql-eval/src/plan_cache.rs'
s=(root/'crates/sparql-eval/src/plan_cache.rs').read_text()
s=s.replace('    fn insert_with_eviction_admitted(', '    pub(crate) fn insert_with_eviction_admitted(')
p.write_text(s)
