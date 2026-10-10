from pathlib import Path

def rewrite_ingresses(s):
    def edit(name, transform):
        nonlocal s
        a = s.index('    '+name)
        b = s.index('\n    ///', a)
        s = s[:a] + transform(s[a:b]) + s[b:]
    reporting = '''            let reporting = bounded_workspace::reserve_reporting(dataset)
                .map_err(bounded_workspace::AdmissionError::into_diagnostic)?;
            let workspace = reporting.execution();
'''
    def direct(part, governed=False, operation=False):
        needle = '            let options = publication.options(dataset, options)?;\n'
        part = part.replace(needle, needle+reporting, 1)
        part = part.replace('AdmittedSubstitutions::requested(request.substitutions, options.declared_prebound);',
            'AdmittedSubstitutions::requested_admitted(request.substitutions, options.declared_prebound, &workspace.capability())?;')
        part = part.replace('self.prepare_request(', 'self.prepare_request_admitted(')
        part = part.replace('                &admitted.parameters,\n', '                &admitted.parameters,\n                &workspace.capability(),\n')
        if governed:
            part = part.replace('''            let _reporting =
                bounded_workspace::reserve_reporting(dataset).map_err(bounded_workspace::AdmissionError::into_diagnostic)?;
            let state = Arc::new(GovernorState::new(governors));''', '''            let state = crate::workspace::SharedWorkspace::new_admitted(
                GovernorState::with_workspace(governors, workspace.capability()), &workspace.capability(),
            )?;''')
            part = part.replace('self.query_governed_prepared_in_state(', 'self.query_governed_prepared_admitted(')
            if operation:
                part = part.replace('                state,\n', '                &crate::workspace::SharedWorkspace::from(state.clone()),\n')
            part = part.replace('                Sequencing::Free,\n', '                Sequencing::Free,\n                &workspace,\n')
        else:
            a=part.index('            let workspace = bounded_workspace::reserve(')
            b=part.index('            self.query_prepared_admitted(', a)
            part=part[:a]+part[b:]
        return part
    edit("fn query_governed_view", lambda p:direct(p, True))
    edit("pub fn query_governed_in_operation", lambda p:direct(p, True, True))
    edit("pub fn query_with_options_view", direct)
    def prepared_governed(part):
        part=part.replace('''            let _reporting =
                bounded_workspace::reserve_reporting(dataset).map_err(bounded_workspace::AdmissionError::into_diagnostic)?;
            let state = Arc::new(GovernorState::new(governors));''', reporting+'''            let state = crate::workspace::SharedWorkspace::new_admitted(
                GovernorState::with_workspace(governors, workspace.capability()), &workspace.capability(),
            )?;''')
        part=part.replace('self.query_governed_prepared_in_state(', 'self.query_governed_prepared_admitted(')
        return part.replace('                Sequencing::Free,\n','                Sequencing::Free,\n                &workspace,\n')
    edit("pub fn query_prepared_governed_view", prepared_governed)
    def explain(part):
        needle='            let options = publication.options(dataset, options)?;\n'
        part=part.replace(needle,needle+reporting)
        a=part.index('            let prepared = self.prepare_for(')
        b=part.index('            self.explain_prepared(',a)
        return part[:a]+'''            let prepared = self.prepare_for_admitted(query_text, base_iri, options.env, &workspace.capability())?;
'''+part[b:]
    edit("fn explain_for", explain)
    def interned(part):
        needle='            let options = publication.options(dataset, options)?;\n'
        part=part.replace(needle,needle+reporting)
        part=part.replace('RequestParameters::of(', 'RequestParameters::of_admitted(')
        part=part.replace('                options.declared_prebound,\n            );', '                options.declared_prebound,\n                &workspace.capability(),\n            )?;')
        part=part.replace('self.prepare_request(request.query, request.base_iri, options.env, &admitted)?;', 'self.prepare_request_admitted(request.query, request.base_iri, options.env, &admitted, &workspace.capability())?;')
        part=part.replace('            admitted.check(&prepared, options)?;\n','            admitted.check_admitted(&prepared, options, &workspace.capability())?;\n')
        a=part.index('            let workspace = bounded_workspace::reserve(')
        b=part.index('            let ', a+len('            let workspace ='))
        part=part[:a]+part[b:]
        return part
    edit("pub fn query_interned_view",interned)
    edit("pub fn query_governed_interned_in_operation",interned)
    def prepared(part):
        return part.replace('check_plan_matches_relations(prepared, options, &crate::DetHashSet::default())?;',
            'RequestParameters::none().check_admitted(prepared, options, &workspace.capability())?;')
    edit("pub fn query_prepared_view",prepared)
    # Request metadata owns its precise spilled SmallVec arrays, independently
    # of the broader execution account; drop arrays before these leases.
    s=s.replace("    exempt: purrdf_core::SmallVec<[&'a str; 8]>,\n}", "    exempt: purrdf_core::SmallVec<[&'a str; 8]>,\n    _allocations: [Option<crate::WorkspaceAllocation>; 2],\n}", 1)
    s=s.replace('''        for count in [substitutions.len(), declared.len()] {''','''        let mut allocations = [None, None];
        for (index, count) in [substitutions.len(), declared.len()].into_iter().enumerate() {''',1)
    s=s.replace('''                workspace.retain(u64::try_from(bytes).map_err(|_| crate::EvalError::WorkspaceBoundOverflow)?)?;''','''                allocations[index] = Some(workspace.charge(u64::try_from(bytes).map_err(|_| crate::EvalError::WorkspaceBoundOverflow)?)?);''',1)
    s=s.replace('        Ok(Self { names, exempt })','        Ok(Self { names, exempt, _allocations: allocations })',1)
    s=s.replace('''            exempt: purrdf_core::SmallVec::new(),
        }''','''            exempt: purrdf_core::SmallVec::new(),
            _allocations: [None, None],
        }''',1)
    marker='''    /// Re-check `prepared` against `options` under these parameters'''
    s=s.replace(marker,'''    fn check_admitted(&self, prepared: &PreparedQuery, options: QueryOptions<'_>, workspace: &crate::WorkspaceCapability) -> Result<(), EvaluationFailure> {
        let mut frame = crate::workspace::LexicalFrame::new(workspace);
        let mut memory = purrdf_lex::allocation::Memory::new(&mut frame);
        admit_structure_with_memory::<true>(&prepared.query, &mut memory)?;
        check_plan_matches_registries_with_memory(prepared, options, &self.names, &mut memory)
    }

'''+marker,1)
    s=s.replace('substitutions.parameters.check(prepared, options)?;', 'substitutions.parameters.check_admitted(prepared, options, &workspace.capability())?;')
    return s
