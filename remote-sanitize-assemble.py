from pathlib import Path
stage = Path(__file__).parent
root = stage.parent.parent
path = Path('crates/sparql-eval/src/remote.rs')
s = (root / path).read_text()
def swap(start, end, text):
    global s
    a = s.index(start); b = s.index(end, a)
    s = s[:a] + text + '\n\n' + s[b:]

swap('fn sanitize_forwarded_body(', '/// The `index`th child pattern', '''#[cfg(test)]
fn sanitize_forwarded_body(pattern: &GraphPattern) -> GraphPattern {
    let mut frame = crate::workspace::LexicalFrame::new(&crate::WorkspaceCapability::resident());
    sanitize_forwarded_body_with_memory(pattern, &mut purrdf_lex::allocation::Memory::new(&mut frame))
        .expect("resident SERVICE sanitization allocation failed")
}

fn service_storage(error: purrdf_lex::allocation::StorageError, memory: &mut crate::expr::CorrelatedMemory<'_>) -> EvalError {
    memory.admission_mut().storage_error(error, "SERVICE forwarded body")
}

fn service_child<T: purrdf_sparql_algebra::Subtree + purrdf_lex::walk::Dismantle<Continuation = purrdf_sparql_algebra::DropOwner>>(
    value: T, memory: &mut crate::expr::CorrelatedMemory<'_>,
) -> Result<Child<T>, EvalError> {
    let layout = core::alloc::Layout::new::<T>();
    memory.add_bytes(layout.size()).map_err(|e| service_storage(e, memory))?;
    purrdf_core::small::try_boxed(value).map(Child::from)
        .map_err(|_| EvalError::AllocationFailed { construct: "SERVICE child", requested_bytes: layout.size() as u64 })
}

fn clone_service_slice<T, U>(values: &[T], memory: &mut crate::expr::CorrelatedMemory<'_>,
    mut clone: impl FnMut(&T, &mut crate::expr::CorrelatedMemory<'_>) -> Result<U, purrdf_lex::allocation::StorageError>,
) -> Result<Vec<U>, EvalError> {
    let mut result = Vec::new();
    memory.reserve(&mut result, values.len()).map_err(|e| service_storage(e, memory))?;
    for value in values { result.push(clone(value, memory).map_err(|e| service_storage(e, memory))?); }
    Ok(result)
}

fn sanitize_forwarded_body_with_memory(pattern: &GraphPattern, memory: &mut crate::expr::CorrelatedMemory<'_>) -> Result<GraphPattern, EvalError> {
    struct Frame<'a> { node: &'a GraphPattern, children: Vec<GraphPattern> }
    let mut frames: Vec<Frame<'_>> = Vec::new();
    let mut entering = Some(pattern);
    loop {
        if let Some(node) = entering.take() {
            #[cfg(test)]
            purrdf_sparql_algebra::NodeRef::Pattern(node).for_each_child(|child| {
                if !matches!(child, purrdf_sparql_algebra::NodeRef::Pattern(_)) { crate::op_count::count_copied(child); }
            });
            memory.push(&mut frames, Frame { node, children: Vec::new() }).map_err(|e| service_storage(e, memory))?;
        }
        let frame = frames.last_mut().expect("a frame is open until the root is rebuilt");
        if let Some(child) = nth_child_pattern(frame.node, frame.children.len()) { entering = Some(child); continue; }
        let Frame { node, children } = frames.pop().expect("the frame is open");
        let rebuilt = rebuild_sanitized_with_memory(node, children, memory)?;
        match frames.last_mut() {
            None => { memory.release_vec(frames).map_err(|e| service_storage(e, memory))?; return Ok(rebuilt); }
            Some(parent) => memory.push(&mut parent.children, rebuilt).map_err(|e| service_storage(e, memory))?,
        }
    }
}''')

a = s.index('fn rebuild_sanitized('); b = s.index('/// Whether `term` is a blank node', a)
body = s[a:b]
body = body.replace('fn rebuild_sanitized(node: &GraphPattern, children: Vec<GraphPattern>) -> GraphPattern {', '''fn rebuild_sanitized_with_memory(node: &GraphPattern, children: Vec<GraphPattern>, memory: &mut crate::expr::CorrelatedMemory<'_>) -> Result<GraphPattern, EvalError> {
    let child_bytes = core::alloc::Layout::array::<GraphPattern>(children.capacity()).map_err(|_| EvalError::WorkspaceBoundOverflow)?.size();''')
body = body.replace('    match node {', '    let rebuilt = match node {', 1)
body = body.replace('patterns: patterns.clone()', 'patterns: clone_service_slice(patterns, memory, |v, m| v.clone_with_memory(m))?')
body = body.replace('subject: subject.clone()', 'subject: subject.clone_with_memory(memory).map_err(|e| service_storage(e, memory))?')
body = body.replace('path: path.clone()', 'path: path.clone_with_memory(memory).map_err(|e| service_storage(e, memory))?')
body = body.replace('object: object.clone()', 'object: object.clone_with_memory(memory).map_err(|e| service_storage(e, memory))?')
body = body.replace('GraphPattern::PropertyFunction(call.clone())', 'GraphPattern::PropertyFunction(call.clone_with_memory(memory).map_err(|e| service_storage(e, memory))?)')
body = body.replace('strip_blank_columns(variables, bindings)', 'strip_blank_columns_with_memory(variables, bindings, memory)?')
body = body.replace('join_dropping_empty_values(left, right)', 'join_dropping_empty_values_with_memory(left, right, memory)?')
body = body.replace('Child::new(next())', 'service_child(next(), memory)?')
body = body.replace('expression: expression.clone(),', 'expression: expression.as_ref().map(|e| e.clone_with_memory(memory)).transpose().map_err(|e| service_storage(e, memory))?,', 1)
body = body.replace('policy: policy.clone()', 'policy: policy.clone_box_with_memory(memory).map_err(|e| service_storage(e, memory))?')
body = body.replace('expr: expr.clone()', 'expr: expr.clone_with_memory(memory).map_err(|e| service_storage(e, memory))?')
body = body.replace('GraphPattern::Union { arms } => GraphPattern::Union {\n            arms: arms.map_ref(|_| next()),\n        },', 'GraphPattern::Union { .. } => GraphPattern::Union { arms: children.collect::<Vec<_>>().into() },')
# Union already owns the admitted child vector; move it intact instead of allocating a collection.
body = body.replace('    let mut children = children.into_iter();', '''    if matches!(node, GraphPattern::Union { .. }) { return Ok(GraphPattern::Union { arms: children.into() }); }
    let mut children = children.into_iter();''')
body = body.replace('GraphPattern::Union { .. } => GraphPattern::Union { arms: children.collect::<Vec<_>>().into() },', 'GraphPattern::Union { .. } => unreachable!("union moves its admitted arms before child iteration"),')
body = body.replace('expression: expression.clone()', 'expression: expression.clone_with_memory(memory).map_err(|e| service_storage(e, memory))?')
body = body.replace('GraphPattern::OrderBy { expression, .. } => GraphPattern::OrderBy {\n            inner: service_child(next(), memory)?,\n            expression: expression.clone_with_memory(memory).map_err(|e| service_storage(e, memory))?,', 'GraphPattern::OrderBy { expression, .. } => GraphPattern::OrderBy {\n            inner: service_child(next(), memory)?,\n            expression: clone_service_slice(expression, memory, |v, m| v.clone_with_memory(m))?,')
body = body.replace('variables: variables.clone()', 'variables: clone_service_slice(variables, memory, |v, _| Ok(v.clone()))?')
body = body.replace('aggregates: aggregates.clone()', 'aggregates: clone_service_slice(aggregates, memory, |(v, a), m| Ok((v.clone(), a.clone_with_memory(m)?)))?')
tail = body.rfind('    }\n}')
body = body[:tail] + '''    };
    drop(next);
    drop(children);
    memory.release_bytes(child_bytes).map_err(|e| service_storage(e, memory))?;
    Ok(rebuilt)
}''' + body[tail + len('    }\n}'):]
s = s[:a] + body + s[b:]

swap('fn ground_term_has_blank_node(', '/// Remove every column', '''fn ground_term_has_blank_node_with_memory(term: &GroundTerm, memory: &mut crate::expr::CorrelatedMemory<'_>) -> Result<bool, EvalError> {
    let mut pending = purrdf_lex::walk::WorkList::<_, 8>::with(term);
    let mut found = false;
    while let Some(term) = pending.pop() {
        match term {
            GroundTerm::NamedNode(_) | GroundTerm::Literal(_) => {},
            GroundTerm::BlankNode(_) => { found = true; break; },
            GroundTerm::Triple(t) => { for child in [&t.object, &t.subject] { pending.try_push_admitted(child, |bytes| memory.add_bytes(bytes)).map_err(|e| service_storage(e, memory))?; } },
        }
    }
    pending.release_admitted(|bytes| memory.release_bytes(bytes)).map_err(|e| service_storage(e, memory))?;
    Ok(found)
}''')
swap('fn strip_blank_columns(', '/// `Join(x, Values', '''fn strip_blank_columns_with_memory(variables: &[Variable], bindings: &[Vec<Option<GroundTerm>>], memory: &mut crate::expr::CorrelatedMemory<'_>) -> Result<(Vec<Variable>, Vec<Vec<Option<GroundTerm>>>), EvalError> {
    let mut keep = Vec::new();
    for column in 0..variables.len() {
        let mut keep_column = true;
        for row in bindings {
            if let Some(Some(term)) = row.get(column) { if ground_term_has_blank_node_with_memory(term, memory)? { keep_column = false; break; } }
        }
        memory.push(&mut keep, keep_column).map_err(|e| service_storage(e, memory))?;
    }
    let mut new_variables = Vec::new();
    for (v, k) in variables.iter().zip(&keep) { if *k { memory.push(&mut new_variables, v.clone()).map_err(|e| service_storage(e, memory))?; } }
    let mut new_bindings = Vec::new();
    for row in bindings {
        let mut cells = Vec::new();
        for (cell, k) in row.iter().zip(&keep) {
            if *k { let value = cell.as_ref().map(|v| v.clone_with_memory(memory)).transpose().map_err(|e| service_storage(e, memory))?; memory.push(&mut cells, value).map_err(|e| service_storage(e, memory))?; }
        }
        memory.push(&mut new_bindings, cells).map_err(|e| service_storage(e, memory))?;
    }
    memory.release_vec(keep).map_err(|e| service_storage(e, memory))?;
    Ok((new_variables, new_bindings))
}''')

a=s.index('fn join_dropping_empty_values('); b=s.index('/// See [`join_dropping_empty_values`]',a)
body=s[a:b]
body=body.replace('fn join_dropping_empty_values(left: GraphPattern, right: GraphPattern) -> GraphPattern {', 'fn join_dropping_empty_values_with_memory(left: GraphPattern, right: GraphPattern, memory: &mut crate::expr::CorrelatedMemory<\'_>) -> Result<GraphPattern, EvalError> {\n    Ok(')
body=body.replace('sink_values_under_filters(left, right)', 'sink_values_under_filters_with_memory(left, right, memory)?')
pos=body.index('\n}\n\n/// `Join(Filter')
body=body[:pos]+ '\n    )'+body[pos:]
body=body.replace('fn sink_values_under_filters(filtered: GraphPattern, values: GraphPattern) -> GraphPattern {', 'fn sink_values_under_filters_with_memory(filtered: GraphPattern, values: GraphPattern, memory: &mut crate::expr::CorrelatedMemory<\'_>) -> Result<GraphPattern, EvalError> {')
body=body.replace('return GraphPattern::Join {', 'return Ok(GraphPattern::Join {',1).replace('right: Child::new(values),\n        };', 'right: Child::new(values),\n        });',1)
body=body.replace('if !mentions_any(&expr, block_vars)', 'if !mentions_any_admitted(&expr, block_vars, memory)?')
body=body.replace('conditions.push(expr);', 'memory.push(&mut conditions, expr).map_err(|e| service_storage(e, memory))?;')
body=body.replace('if joins_identical_block(&rest, &values)', 'if joins_identical_block_with_memory(&rest, &values, memory)?')
body=body.replace('    for expr in conditions.into_iter().rev() {', '    while let Some(expr) = conditions.pop() {')
body=body.replace('    pattern\n}', '    memory.release_vec(conditions).map_err(|e| service_storage(e, memory))?;\n    Ok(pattern)\n}',1)
for term in ['left','right','filtered','values','rest','pattern']:
    body=body.replace('Child::new('+term+')', 'service_child('+term+', memory)?')
ma=body.index('fn mentions_any('); mb=body.index('/// Whether `pattern`,',ma)
body=body[:ma]+'''fn mentions_any_admitted(expr: &purrdf_sparql_algebra::Expression, vars: &[Variable], memory: &mut crate::expr::CorrelatedMemory<'_>) -> Result<bool, EvalError> {
    let mentioned = crate::expr::expr_vars_admitted(expr, memory.admission_mut().workspace())?;
    Ok(vars.iter().any(|v| mentioned.contains(v)))
}

'''+body[mb:]
body=body.replace('fn joins_identical_block(pattern: &GraphPattern, values: &GraphPattern) -> bool {', 'fn joins_identical_block_with_memory(pattern: &GraphPattern, values: &GraphPattern, memory: &mut crate::expr::CorrelatedMemory<\'_>) -> Result<bool, EvalError> {')
body=body.replace('return false;', 'return Ok(false);')
body=body.replace('    matches!(cur, GraphPattern::Join { right, .. } if **right == *values)', '    match cur { GraphPattern::Join { right, .. } => right.eq_with_memory(values, memory).map_err(|e| service_storage(e, memory)), _ => Ok(false) }')
s=s[:a]+body+s[b:]
for old,new in [('sanitize_forwarded_body','sanitize_forwarded_body_with_memory'),('strip_blank_columns','strip_blank_columns_with_memory'),('sink_values_under_filters','sink_values_under_filters_with_memory'),('join_dropping_empty_values','join_dropping_empty_values_with_memory')]:
    s=s.replace('[`'+old+'`]', '[`'+new+'`]')
out=stage/'remote-native-owner-postimages'/path
out.parent.mkdir(parents=True,exist_ok=True);out.write_text(s)
