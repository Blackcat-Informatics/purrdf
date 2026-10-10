# Why not Rust: Ignored Stage-only patch assembly preserves shipping source untouched.
from pathlib import Path
import re
import difflib

WT = Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
STAGE = WT / '.stage/sparql-eval-complete-bounded-workspace'
OUT = STAGE / 'prebinding-rewrite-owner-postimages'
OUT.mkdir(exist_ok=True)
base = {}
post = {}

def read(path):
    if path not in post:
        base[path] = (WT/path).read_text()
        post[path] = base[path]
    return post[path]

def balanced(s, at, left='{', right='}'):
    depth = 0
    i = at
    while i < len(s):
        if s.startswith('//', i):
            j = s.find('\n', i)
            i = len(s) if j < 0 else j
            continue
        if s.startswith('/*', i):
            j = s.find('*/', i+2)
            if j < 0: raise ValueError('comment')
            i = j+2
            continue
        if s[i] == '"':
            i += 1
            while i < len(s):
                if s[i] == '\\': i += 2
                elif s[i] == '"': i += 1; break
                else: i += 1
            continue
        if s[i] == "'" and re.match(r"'(?:\\.|[^'\\])'", s[i:]):
            i += len(re.match(r"'(?:\\.|[^'\\])'", s[i:])[0]); continue
        if s[i] == left: depth += 1
        if s[i] == right:
            depth -= 1
            if depth == 0: return i+1
        i += 1
    raise ValueError(('unbalanced', at, s[at:at+80]))

def span(s, name):
    m = re.search(r'(?m)^[ \t]*(?:(?:pub(?:\([^\n]*?\))?|const) )*fn '+re.escape(name)+r'\b', s)
    if not m: raise ValueError(('missing fn', name))
    a = s.find('{', m.end())
    return m.start(), a, balanced(s, a)

def function(s, name):
    a,b,c=span(s,name)
    return s[a:c]

def replace(path,name,new):
    s=read(path); a,b,c=span(s,name)
    post[path]=s[:a]+new+s[c:]

def body_of(s,name):
    a,b,c=span(s,name); return s[b+1:c-1]

def calls(body, names, memory='memory', fallible=True):
    # Calls are transformed from right to left; balanced arguments retain actual
    # closures and nested expression structure. Definitions live outside this body.
    for old,new in sorted(names.items(), key=lambda p:-len(p[0])):
        matches=list(re.finditer(r'\b'+re.escape(old)+r'\s*\(',body))
        for m in reversed(matches):
            line=body[body.rfind('\n',0,m.start())+1:m.start()]
            if line.lstrip().startswith('//'): continue
            p=body.find('(',m.start(),m.end()); end=balanced(body,p,'(',')')
            args=body[p+1:end-1].rstrip()
            if args.endswith(','): args=args[:-1].rstrip()
            add=(', ' if args else '')+memory
            q='?' if fallible else ''
            body=body[:m.start()]+new+'('+args+add+')'+q+body[end:]
    return body

def fallible_body(body):
    body=re.sub(r'\breturn\s*;', 'return Ok(());', body)
    # Return expressions may terminate a match arm with a comma rather than a
    # statement with a semicolon. Respect nested expression delimiters.
    for m in reversed(list(re.finditer(r'\breturn\s+(?!Ok\()',body))):
        start=m.end(); i=start; depth=0
        while i<len(body):
            ch=body[i]
            if ch in '([{': depth+=1
            elif ch in ')]}':
                if depth==0: break
                depth-=1
            if depth==0 and ch in ',;': break
            i+=1
        body=body[:start]+'Ok('+body[start:i].rstrip()+')'+body[i:]
    return body

P='crates/sparql-eval/src/substitute.rs'
source=read(P)
support=(STAGE/'prebinding-rewrite-support.rs').read_text()
leaves=(STAGE/'prebinding-rewrite-leaves.rs').read_text()
support=support[support.index('type RewriteMemory'):].replace("RewriteMemory<'a, 's>","RewriteMemory<'a>").replace("RewriteMemory<'_, '_>","RewriteMemory<'_>")
support=support.replace('fn copy_entries(\n        entries: impl Iterator<Item = &\'_ (Variable, GroundTerm)> + Clone,','fn copy_entries<\'a>(\n        entries: impl Iterator<Item = &\'a (Variable, GroundTerm)> + Clone,')
leaves=leaves[leaves.index('fn apply_probes_with_memory'):].replace("RewriteMemory<'_, '_>","RewriteMemory<'_>")

# One original frame resumes already-admitted caller payload; no second admission
# or silent reset of the parser's existing requested layout total.
L='crates/lex/src/allocation.rs'
s=read(L)
needle='    /// The currently admitted requested layouts of payload and working buffers.'
s=s.replace(needle,'''    /// Resume a native computation under the same original admission grant.
    /// `live_bytes` must be the requested layout total already owned by `storage`;
    /// the caller keeps those payloads alive until this memory adopts or releases
    /// them. This constructor allocates nothing and does not resize the grant.
    #[must_use]
    pub fn resume(storage: &'a mut S, live_bytes: usize) -> Self {
        Self { storage, live: live_bytes }
    }

'''+needle,1)
post[L]=s

# Reuse the native vector replacement home for aggregate scratch / work stacks.
W='crates/sparql-eval/src/workspace.rs'
s=read(W)
at=s.index('    pub(crate) const fn live_bytes(&self) -> usize {')
s=s[:at]+'''    /// Adopt the original private query grant without allocating or resizing it.
    /// `allocation` must belong to `workspace` and cover the exact requested
    /// layout total `live_bytes` of the still-live query payload. Native callers
    /// resume Memory with that same total; they never reset a live AST to zero.
    pub(crate) fn from_allocation(
        workspace: &WorkspaceCapability,
        allocation: WorkspaceAllocation,
        live_bytes: usize,
    ) -> Self {
        Self { workspace: workspace.clone(), allocation: Some(allocation),
            live: live_bytes, failure: None }
    }

'''+s[at:]
at=s.index('    pub(crate) fn reserve_next(&mut self)')
s=s[:at]+'''    pub(crate) fn reserve_additional(&mut self, additional: usize) -> Result<(), EvalError> {
        let required = self.values.len().checked_add(additional)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        if required > self.values.capacity() {
            let capacity = self.values.capacity().checked_mul(2)
                .ok_or(EvalError::WorkspaceBoundOverflow)?.max(required);
            self.workspace.reserve_vec(&mut self.values, &mut self.allocation, capacity)?;
        }
        Ok(())
    }

'''+s[at:]
post[W]=s

# Fixed-length aggregate argument mutation preserves every constructor invariant.
A='crates/sparql-algebra/src/algebra.rs'
s=read(A)
at=s.index('    /// Decompose into `(function, args, scalarvals, order_by, distinct)`,')
s=s[:at]+'''    /// Borrow the existing argument slots, then the existing FOLD sort-key slots,
    /// in source order. Replacing a slot changes neither arity nor which function,
    /// scalar arguments or sort-key count this checked aggregate contains.
    pub fn expressions_mut(&mut self) -> impl Iterator<Item = &mut Expression> {
        self.args.iter_mut().chain(self.order_by.iter_mut().map(OrderExpression::expression_mut))
    }

'''+s[at:]
post[A]=s

# Native shared mutable walk; its resident API delegates the exact same body.
B='crates/sparql-algebra/src/walk.rs'
s=read(B)
old=function(s,'for_each_expression_mut')
new='''pub fn for_each_expression_mut<'e>(
    roots: impl IntoIterator<Item = &'e mut Expression>,
    mut visit: impl FnMut(&mut Expression),
) {
    let mut storage = purrdf_lex::allocation::Resident;
    let mut memory = purrdf_lex::allocation::Memory::new(&mut storage);
    for_each_expression_mut_with_memory(roots, &mut memory, |expression, _| {
        visit(expression);
        Ok::<(), core::convert::Infallible>(())
    }).expect("resident mutable expression walk allocation failed");
}

/// A mutable visitor failure remains separate from physical native walk refusal.
#[derive(Debug)]
pub enum MutationError<E> {
    /// The original visitor failure, retained unchanged.
    Visitor(E),
    /// Checked layout, original admission or allocator refusal.
    Storage(purrdf_lex::allocation::StorageError),
}

/// Visit the same native expression slots with fallible working-list storage.
/// The visitor runs before reading the possibly rewritten operands. EXISTS bodies
/// remain graph-pattern boundaries and are never traversed as expressions.
///
/// # Errors
/// Returns the first visitor failure or physical work-list refusal unchanged.
pub fn for_each_expression_mut_with_memory<'e, S, E>(
    roots: impl IntoIterator<Item = &'e mut Expression>,
    memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    mut visit: impl FnMut(&mut Expression, &mut purrdf_lex::allocation::Memory<'_, S>) -> Result<(), E>,
) -> Result<(), MutationError<E>>
where S: purrdf_lex::allocation::Admission + ?Sized,
{
    let mut pending: purrdf_lex::walk::WorkList<&'e mut Expression, 16> =
        purrdf_lex::walk::WorkList::new();
    for root in roots {
        pending.try_push_admitted(root, memory).map_err(MutationError::Storage)?;
    }
    pending.reverse_top(pending.len());
    while let Some(expression) = pending.pop() {
        visit(expression, memory).map_err(MutationError::Visitor)?;
        push_operands_mut_with_memory(expression, &mut pending, memory)
            .map_err(MutationError::Storage)?;
    }
    pending.release_admitted(memory).map_err(MutationError::Storage)
}
'''
s=s.replace(old,new,1)
body=body_of(s,'push_operands_mut')
body=body.replace('pending.extend(operands.iter_mut().rev());','for operand in operands.iter_mut().rev() { pending.try_push_admitted(operand, memory)?; }')
body=body.replace('pending.extend(steps.iter_mut().rev().map(|(_, operand)| operand));','for (_, operand) in steps.iter_mut().rev() { pending.try_push_admitted(operand, memory)?; }')
body=body.replace('pending.extend(list.iter_mut().rev());','for operand in list.iter_mut().rev() { pending.try_push_admitted(operand, memory)?; }')
body=re.sub(r'pending\.push\(([^;]+)\);',r'pending.try_push_admitted(\1, memory)?;',body)
new='''fn push_operands_mut_with_memory<'e, S: purrdf_lex::allocation::Admission + ?Sized>(
    expr: &'e mut Expression,
    pending: &mut purrdf_lex::walk::WorkList<&'e mut Expression, 16>,
    memory: &mut purrdf_lex::allocation::Memory<'_, S>,
) -> Result<(), purrdf_lex::allocation::StorageError> {'''+body+'\n    Ok(())\n}'
a,b,c=span(s,'push_operands_mut'); s=s[:a]+new+s[c:]
post[B]=s

# Grounded prepared values retain original owners; resident id bindings retain the
# same unchanged public behavior and do not acquire a hidden bounded lease.
s=read(P)
s=s.replace('    Ground(GroundTerm),','    Ground(GroundTerm),\n    /// An admitted id-door binding keeps its original child grant with its payload.\n    AdmittedGround(crate::workspace::SharedWorkspace<AdmittedGroundTerm>),',1)
s=s.replace('                ParameterValue::Ground(ground) => clone_ground_with_memory(ground, memory),','                ParameterValue::Ground(ground) => clone_ground_with_memory(ground, memory),\n                ParameterValue::AdmittedGround(ground) => clone_ground_with_memory(&ground.term, memory),',1)
s=s.replace('                ParameterValue::Ground(_) => None,','                ParameterValue::Ground(_) | ParameterValue::AdmittedGround(_) => None,',1)
post[P]=s

# The native probe path shares validated IRI/datatype leaves exactly as the old
# prepared reuser did, but acquires lexical storage before constructing a changed
# literal and uses the fallible original ground walk for every non-leaf case.
replacement='''    fn ground_reusing(
        self, index: usize, previous: Option<&GroundTerm>,
    ) -> Result<GroundTerm, RdfDiagnostic> {
        let mut frame = crate::workspace::LexicalFrame::new(&crate::WorkspaceCapability::resident());
        let mut memory = purrdf_lex::allocation::Memory::new(&mut frame);
        self.ground_reusing_with_memory(index, previous, &mut memory)
            .map_err(GroundFailure::into_resident_diagnostic)
    }

    fn ground_reusing_with_memory(
        self, index: usize, previous: Option<&GroundTerm>, memory: &mut RewriteMemory<'_>,
    ) -> RewriteResult<GroundTerm> {
        match (previous, self.value(index)) {
            (Some(GroundTerm::NamedNode(before)), Some(TermValue::Iri(iri)))
                if before.as_str() == iri => return Ok(GroundTerm::NamedNode(before.clone())),
            (Some(GroundTerm::Literal(before)), Some(TermValue::Literal {
                lexical_form, datatype, language: None, ..
            })) if before.language().is_none() && before.datatype().as_str() == datatype => {
                if before.value() == lexical_form { return Ok(GroundTerm::Literal(before.clone())); }
                let workspace = rewrite_workspace(memory);
                return Ok(GroundTerm::Literal(Literal::from_admitted(
                    workspace.authored_text(lexical_form)?, before.datatype().clone(), None, None,
                )));
            }
            _ => {}
        }
        self.ground_with_memory(index, memory)
    }'''
replace(P,'ground_reusing',replacement)

# Expression substitutions borrow immutable grounded entries on the operational
# path. Existing resident reference fixtures may still supply their owned Vec.
s=read(P).replace('struct ExprSubs(Vec<(Variable, GroundTerm)>);','struct ExprSubs<P = Vec<(Variable, GroundTerm)>>(P);')
s=s.replace('impl ExprSubs {','impl<P: AsRef<[(Variable, GroundTerm)]>> ExprSubs<P> {\n    fn entries(&self) -> &[(Variable, GroundTerm)] { self.0.as_ref() }',1)
start=s.index('impl<P: AsRef<[(Variable, GroundTerm)]>> ExprSubs<P>')
end=s.index('\n}\n',start)+3
section=s[start:end].replace('self.0\n','self.entries()\n').replace('self.0.iter()','self.entries().iter()')
s=s[:start]+section+s[end:]
# The bitmap is an existing optimization only; this changes no 64-bit law.
start=s.index('impl SeedColumns {');end=s.index('\n}\n',start)+3
section=s[start:end].replace('&ExprSubs,','&ExprSubs<impl AsRef<[(Variable, GroundTerm)]>>,').replace('expr_subs.0','expr_subs.entries()')
s=s[:start]+section+s[end:]
post[P]=s

# Resident signatures stay intact; each delegates the one native semantic body.
native_names = {name:name+'_with_memory' for name in [
    'push_probe_constants','push_probes','enter_push','resume_push',
    'probe_term_pattern','probe_triple_pattern','probe_call_arguments',
    'restore_probed_bindings','complete_shacl_leaf','restore_shacl_bindings',
    'drive_call_arguments','call_driver','plant_stand_alone_driver','plant_left_driver',
    'plant_scoped_driver','undriven_variables','bind_call_arguments','substitute_in_term_pattern',
    'substitute_in_graph_pattern','enter_substitution','resume_substitution','descend_into_inner',
    'finish_own_expressions','carry_past_group','drive_expression_reads','operand_schema',
    'carried_columns','note_reads','rename_reads','substitute_in_expressions','replace_exists_bodies',
    'true_literal','seed_reaches_every_read','seed_above_a_lone_sub_select','seed_every_sub_select',
    'seed_sub_selects_in','seed_both_minus_operands','seed_minus_in','highest_index',
    'join_assignments_with_prebinding','join_assignments_in','compatible_with_binding',
]}

def resident(name,signature,args,convert=''):
    native=native_names.get(name,name+'_with_memory')
    tail=f'{native}({args}'+(', ' if args else '')+'memory)'
    if convert: tail += convert
    return signature+' {\n    resident_rewrite(|memory| '+tail+')\n}'

wrappers={
    'apply_probes':('pub(crate) fn apply_probes(query: Query, probes: Vec<(Variable, GroundTerm)>) -> Query','query, &probes'),
    'push_probe_constants':('fn push_probe_constants(core: &mut GraphPattern, probes: &[(Variable, GroundTerm)])','core, probes'),
    'push_probes':('fn push_probes(pattern: &mut GraphPattern, probes: &[(Variable, GroundTerm)], at_core_root: bool)','pattern, probes, at_core_root'),
    'probe_term_pattern':('fn probe_term_pattern(term: &mut TermPattern, probes: &[(Variable, GroundTerm)], probed: &mut Vec<usize>)','term, probes, probed'),
    'probe_triple_pattern':('fn probe_triple_pattern(triple: &mut TriplePattern, probes: &[(Variable, GroundTerm)], probed: &mut Vec<usize>)','triple, probes, probed'),
    'probe_call_arguments':('fn probe_call_arguments(call: &mut PropertyFunctionCall, probes: &[(Variable, GroundTerm)]) -> Vec<usize>','call, probes'),
    'restore_probed_bindings':('fn restore_probed_bindings(leaf: &mut GraphPattern, probed: &[usize], probes: &[(Variable, GroundTerm)], at_core_root: bool)','leaf, probed, probes, at_core_root'),
    'complete_shacl_leaf':('fn complete_shacl_leaf(leaf: &mut GraphPattern, probes: &[(Variable, GroundTerm)], scope: WalkScope)','leaf, probes, scope'),
    'restore_shacl_bindings':('fn restore_shacl_bindings(leaf: &mut GraphPattern, bound: &[usize], probes: &[(Variable, GroundTerm)], scope: WalkScope)','leaf, bound, probes, scope'),
    'driven_arguments':('fn driven_arguments(call: &PropertyFunctionCall, probes: &[(Variable, GroundTerm)], rule: Unwritable, already: Option<&GraphPattern>) -> Vec<usize>','call, probes, rule, already'),
    'call_driver':('fn call_driver(call: &PropertyFunctionCall, values: &[(Variable, GroundTerm)], rule: Unwritable, already: Option<&GraphPattern>) -> Option<GraphPattern>','call, values, rule, already'),
    'drive_call_arguments':('fn drive_call_arguments(pattern: &mut GraphPattern, probes: &[(Variable, GroundTerm)], rule: Unwritable)','pattern, probes, rule'),
    'plant_stand_alone_driver':('fn plant_stand_alone_driver(call: &mut GraphPattern, seed: GraphPattern)','call, seed'),
    'plant_left_driver':('fn plant_left_driver(left: &mut GraphPattern, seed: GraphPattern)','left, seed'),
    'plant_scoped_driver':('fn plant_scoped_driver(call: &mut GraphPattern, seed: GraphPattern, kept: Vec<Variable>)','call, seed, kept'),
    'undriven_variables':('fn undriven_variables(call: &PropertyFunctionCall, seed: &GraphPattern) -> Vec<Variable>','call, seed'),
    'bind_call_arguments':('pub(crate) fn bind_call_arguments(call: &mut PropertyFunctionCall, values: &[(Variable, GroundTerm)], already: Option<&GraphPattern>) -> Option<GraphPattern>','call, values, already'),
    'substitute_in_term_pattern':('fn substitute_in_term_pattern(term: &mut TermPattern, values: &[(Variable, GroundTerm)])','term, values'),
    'substitute_in_graph_pattern':('fn substitute_in_graph_pattern(pattern: &mut GraphPattern, expr_subs: &ExprSubs, scope: WalkScope) -> SeedColumns','pattern, expr_subs, scope'),
    'finish_own_expressions':('fn finish_own_expressions(frame: &mut SubstituteFrame, expr_subs: &ExprSubs) -> SeedColumns','frame, expr_subs'),
    'carry_past_group':('fn carry_past_group(node: &mut GraphPattern, expr_subs: &ExprSubs, keys: &[Variable], carried: SeedColumns) -> SeedColumns','node, expr_subs, keys, carried'),
    'drive_expression_reads':('fn drive_expression_reads(node: &mut GraphPattern, expr_subs: &ExprSubs, scope: WalkScope, operand: SeedColumns)','node, expr_subs, scope, operand'),
    'operand_schema':('fn operand_schema(node: &GraphPattern) -> purrdf_core::small::Shared<crate::solution::VarSchema>','node'),
    'carried_columns':('fn carried_columns(node: &GraphPattern, expr_subs: &ExprSubs, reads: &Reads, scope: WalkScope) -> Vec<Variable>','node, expr_subs, reads, scope'),
    'note_reads':('fn note_reads(expr: &Expression, expr_subs: &ExprSubs, reads: &mut Reads)','expr, expr_subs, reads'),
    'rename_reads':('fn rename_reads(expr: &mut Expression, renames: &[(Variable, Variable)])','expr, renames'),
    'substitute_in_expressions':("fn substitute_in_expressions<'e>(roots: impl IntoIterator<Item = &'e mut Expression>, expr_subs: &ExprSubs, bodies: &mut Vec<GraphPattern>)",'roots, expr_subs, bodies'),
    'replace_exists_bodies':("fn replace_exists_bodies<'e>(roots: impl IntoIterator<Item = &'e mut Expression>, bodies: Vec<GraphPattern>)",'roots, bodies'),
    'true_literal':('fn true_literal() -> Expression',''),
    'seed_reaches_every_read':('fn seed_reaches_every_read(query: &Query, probes: &[(Variable, GroundTerm)]) -> bool','query, probes'),
    'seed_above_a_lone_sub_select':('fn seed_above_a_lone_sub_select(pattern: &mut GraphPattern, probes: &[(Variable, GroundTerm)])','pattern, probes'),
    'seed_every_sub_select':('fn seed_every_sub_select(root: &mut GraphPattern, probes: &[(Variable, GroundTerm)])','root, probes'),
    'seed_sub_selects_in':('fn seed_sub_selects_in(root: &mut GraphPattern, probes: &[(Variable, GroundTerm)], seeded: *const GraphPattern)','root, probes, seeded'),
    'seed_both_minus_operands':('fn seed_both_minus_operands(root: &mut GraphPattern, probes: &[(Variable, GroundTerm)])','root, probes'),
    'seed_minus_in':('fn seed_minus_in(root: &mut GraphPattern, probes: &[(Variable, GroundTerm)], in_exists: bool, minus: &mut usize)','root, probes, in_exists, minus'),
    'highest_index':('fn highest_index(pattern: &GraphPattern, prefix: &str) -> usize','pattern, prefix'),
    'join_assignments_with_prebinding':('pub(crate) fn join_assignments_with_prebinding(query: &mut Query, names: &[&str])','query, names'),
    'join_assignments_in':('fn join_assignments_in(root: &mut GraphPattern, on_descent: bool, in_exists: bool, names: &[&str], fresh: &mut usize)','root, on_descent, in_exists, names, fresh'),
    'compatible_with_binding':('fn compatible_with_binding(name: Variable, to: Variable) -> Expression','name, to'),
}

# Capture each native source body before replacing any resident forwarding door.
original={name:function(source,name) for name in wrappers}
for name,(signature,args) in wrappers.items():
    convert=''
    if name=='driven_arguments': convert='.map(|values| values.try_into_resident().expect("resident driver indices"))'
    replace(P,name,resident(name,signature,args,convert))

# A narrowed probe carrier shares real admitted backing arrays and original child
# grants; clones never deep-copy quoted triples or allocate an Rc payload.
s=read(P)
s=s.replace('Narrowed(std::rc::Rc<[(Variable, GroundTerm)]>),','Narrowed(crate::workspace::SharedWorkspace<PreparedProbes>),',1)
s=s.replace('Self::Narrowed(probes) => probes,','Self::Narrowed(probes) => probes.entries(),',1)
a,b,c=span(s,'narrowed')
s=s[:a]+'''    fn narrowed(
        &self, keep: impl Fn(&Variable) -> bool, memory: &mut RewriteMemory<'_>,
    ) -> RewriteResult<Option<Self>> {
        let entries = self.as_slice();
        if entries.iter().all(|(variable, _)| keep(variable)) { return Ok(Some(self.clone())); }
        let entries = entries.iter().filter(|(variable, _)| keep(variable));
        if entries.clone().next().is_none() { return Ok(None); }
        let workspace = rewrite_workspace(memory);
        let copied = PreparedProbes::copy_entries(entries, &workspace)?;
        Ok(Some(Self::Narrowed(crate::workspace::SharedWorkspace::new_admitted(copied, &workspace)?)))
    }'''+s[c:]
post[P]=s

# The rule retains the same pattern conversion; release its actual temporary child
# layouts only after the constructed pattern has died.
s=read(P);a,b,c=span(s,'holds')
s=s[:a]+'''    fn holds(self, ground: &GroundTerm) -> bool {
        resident_rewrite(|memory| self.holds_with_memory(ground, memory))
    }

    fn holds_with_memory(self, ground: &GroundTerm, memory: &mut RewriteMemory<'_>) -> RewriteResult<bool> {
        match self {
            Self::InPattern => {
                let before = memory.live_bytes();
                let result = term_pattern_from_ground_with_memory(ground, memory);
                let pattern = native_storage(memory, result)?;
                let unwritable = pattern.is_none();
                drop(pattern);
                let bytes = memory.live_bytes().checked_sub(before)
                    .ok_or(crate::EvalError::WorkspaceBoundOverflow)?;
                let result = memory.release_bytes(bytes);
                native_storage(memory, result)?;
                Ok(unwritable)
            }
            Self::InArgument => Ok(match Pushability::of(ground) {
                Pushability::Iri(_) | Pushability::Literal(_) => false,
                Pushability::QuotedTriple(_) | Pushability::SeedOnly => true,
            }),
        }
    }'''+s[c:]
post[P]=s

def native_from_original(name,signature,modify=lambda body:body):
    body=body_of(original.get(name,source),name)
    body=calls(body,native_names)
    body=body.replace('expr_subs.0','expr_subs.entries()').replace('expr_subs\n                    .0','expr_subs\n                    .entries()')
    body=modify(body)
    body=fallible_body(body)
    if name in ['push_probes','substitute_in_graph_pattern']:
        pass # Native loops publish only through their explicit typed return.
    elif signature.endswith('RewriteResult<()>'):
        body += '\n    Ok(())\n'
    elif name in ['enter_push','resume_push','resume_substitution']:
        start={'enter_push':'match descent {','resume_push':'match next {','resume_substitution':'match then {'}[name]
        at=body.rfind(start)
        tail=body[at:].rstrip();body=body[:at]+'Ok('+tail+')\n'
    elif name in ['enter_substitution','descend_into_inner']:
        at=body.rfind('Step::Descend(');body=body[:at]+'Ok('+body[at:].rstrip()+')\n'
    elif name=='highest_index':
        at=body.rfind('highest');body=body[:at]+'Ok(highest)\n'
    elif name=='seed_reaches_every_read':
        at=body.rfind('?');body=body[:at]+body[at+1:]
    else:
        body='    Ok({\n'+body+'\n    })\n'
    return signature+' {\n'+body+'}'

native=[]
native.append(native_from_original('push_probe_constants',
    "fn push_probe_constants_with_memory(core: &mut GraphPattern, probes: &[(Variable, GroundTerm)], memory: &mut RewriteMemory<'_>) -> RewriteResult<()>"))
native.append(native_from_original('push_probes',
    "fn push_probes_with_memory(pattern: &mut GraphPattern, probes: &[(Variable, GroundTerm)], at_core_root: bool, memory: &mut RewriteMemory<'_>) -> RewriteResult<()>",
    lambda b:b.replace('let mut open: Vec<PushFrame<\'_>> = Vec::new();','let mut open: crate::AdmittedVec<PushFrame<\'_>> = crate::AdmittedVec::new(&rewrite_workspace(memory));')))

def push_enter(b):
    b=calls(b,{'narrowed':'narrowed'})
    b=re.sub(r'open\.push\((.*?)\);',r'open.push(\1)?;',b,flags=re.S)
    # Native probe_positions records index scratch in the original frame. Destroy
    # that array as soon as the leaf has published its seed; children stay admitted.
    b=b.replace('at_core_root, memory)?;\n            None','at_core_root, memory)?;\n            let released = memory.release_vec(probed);\n            native_storage(memory, released)?;\n            None')
    return b
native.append(native_from_original('enter_push',
    "fn enter_push_with_memory<'p>(mut node: GraphPattern, probes: ProbeList<'p>, at_core_root: bool, open: &mut crate::AdmittedVec<PushFrame<'p>>, memory: &mut RewriteMemory<'_>) -> RewriteResult<Step<PushDescent<'p>, GraphPattern>>",push_enter))
native.append(native_from_original('resume_push',
    "fn resume_push_with_memory<'p>(mut frame: PushFrame<'p>, operand: GraphPattern, open: &mut crate::AdmittedVec<PushFrame<'p>>, memory: &mut RewriteMemory<'_>) -> RewriteResult<Step<PushDescent<'p>, GraphPattern>>",
    lambda b:b.replace('open.push(frame);','open.push(frame)?;').replace('frame.at_core_root, memory)?;\n                    None','frame.at_core_root, memory)?;\n                    let released = memory.release_vec(probed);\n                    native_storage(memory, released)?;\n                    None')))

# Substitute frames keep their original source order and own actual working arrays.
s=read(P)
s=s.replace('    aggregates: Vec<(Variable, AggregateParts)>,','',1)
s=s.replace('    pending_bodies: Vec<GraphPattern>,','    pending_bodies: crate::AdmittedVec<GraphPattern>,',1)
s=s.replace('    done_bodies: Vec<GraphPattern>,','    done_bodies: crate::AdmittedVec<GraphPattern>,',1)
start=s.index('impl SubstituteFrame {');end=s.index('\n}\n',start)+3
s=s[:start]+'''impl SubstituteFrame {
    fn new(node: GraphPattern, scope: WalkScope, stage: SubstituteStage, memory: &mut RewriteMemory<'_>) -> Self {
        let workspace = rewrite_workspace(memory);
        Self { node, scope, stage, operand: SeedColumns(0),
            pending_bodies: crate::AdmittedVec::new(&workspace),
            done_bodies: crate::AdmittedVec::new(&workspace) }
    }

    fn substitute_own_expressions_with_memory(
        &mut self, expr_subs: &ExprSubs<impl AsRef<[(Variable, GroundTerm)]>>, memory: &mut RewriteMemory<'_>,
    ) -> RewriteResult<Option<GraphPattern>> {
        let roots = own_expressions_with_memory(&mut self.node, memory)?;
        substitute_in_expressions_with_memory(roots, expr_subs, &mut self.pending_bodies, memory)?;
        self.pending_bodies.as_mut_slice().reverse();
        Ok(self.pending_bodies.pop())
    }

    fn replace_own_bodies_with_memory(&mut self, memory: &mut RewriteMemory<'_>) -> RewriteResult<()> {
        let bodies = std::mem::replace(&mut self.done_bodies, crate::AdmittedVec::new(&rewrite_workspace(memory)));
        let roots = own_expressions_with_memory(&mut self.node, memory)?;
        replace_exists_bodies_with_memory(roots, bodies, memory)
    }
}'''+s[end:]
# The old mutable-reference collection has one native admitted home now.
a,b,c=span(s,'own_expressions');s=s[:a]+s[c:]
post[P]=s

native.append(native_from_original('substitute_in_graph_pattern',
    "fn substitute_in_graph_pattern_with_memory(pattern: &mut GraphPattern, expr_subs: &ExprSubs<impl AsRef<[(Variable, GroundTerm)]>>, scope: WalkScope, memory: &mut RewriteMemory<'_>) -> RewriteResult<SeedColumns>",
    lambda b:b.replace('let mut open: Vec<SubstituteFrame> = Vec::new();','let mut open: crate::AdmittedVec<SubstituteFrame> = crate::AdmittedVec::new(&rewrite_workspace(memory));')))

def substitute_enter(b):
    b=calls(b,{'substitute_own_expressions':'substitute_own_expressions_with_memory'})
    b=calls(b,{'SubstituteFrame::new':'SubstituteFrame::new'},fallible=False)
    b=b.replace('open.push(frame);','open.push(frame)?;').replace('open.push(SubstituteFrame::new(node, scope, stage, memory));','open.push(SubstituteFrame::new(node, scope, stage, memory))?;')
    b=b.replace('&expr_subs.entries()','expr_subs.entries()')
    return b
native.append(native_from_original('enter_substitution',
    "fn enter_substitution_with_memory(mut node: GraphPattern, scope: WalkScope, expr_subs: &ExprSubs<impl AsRef<[(Variable, GroundTerm)]>>, open: &mut crate::AdmittedVec<SubstituteFrame>, memory: &mut RewriteMemory<'_>) -> RewriteResult<Step<SubstituteDescent, (GraphPattern, SeedColumns)>>",substitute_enter))
native.append(native_from_original('descend_into_inner',
    "fn descend_into_inner_with_memory(mut frame: SubstituteFrame, open: &mut crate::AdmittedVec<SubstituteFrame>, memory: &mut RewriteMemory<'_>) -> RewriteResult<Step<SubstituteDescent, (GraphPattern, SeedColumns)>>",
    lambda b:b.replace('open.push(frame);','open.push(frame)?;')))

def substitute_resume(b):
    b=calls(b,{'substitute_own_expressions':'substitute_own_expressions_with_memory','replace_own_bodies':'replace_own_bodies_with_memory'})
    b=b.replace('open.push(frame);','open.push(frame)?;').replace('frame.done_bodies.push(operand);','frame.done_bodies.push(operand)?;')
    b=re.sub(r'            frame\.aggregates = std::mem::take\(aggregates\).*?\n                \.collect\(\);','',b,flags=re.S)
    b=b.replace('inner, aggregates, ..','inner, ..')
    b=b.replace('&expr_subs.entries()','expr_subs.entries()')
    return b
native.append(native_from_original('resume_substitution',
    "fn resume_substitution_with_memory(mut frame: SubstituteFrame, operand: (GraphPattern, SeedColumns), expr_subs: &ExprSubs<impl AsRef<[(Variable, GroundTerm)]>>, open: &mut crate::AdmittedVec<SubstituteFrame>, memory: &mut RewriteMemory<'_>) -> RewriteResult<Step<SubstituteDescent, (GraphPattern, SeedColumns)>>",substitute_resume))

# Native bodies with manual physical buffer and driver publication follow.
expressions=(STAGE/'prebinding-rewrite-expressions.rs').read_text()
expressions=expressions[expressions.index('fn rewrite_walk'):]
scopes=(STAGE/'prebinding-rewrite-scopes.rs').read_text()
scopes=scopes[scopes.index('#[derive(Clone, Copy)]'):]

# Extend the single term-variable walk rather than retyping its triple traversal
# inside the expression collector. Callback failures retain their original type.
V='crates/sparql-algebra/src/validate.rs'
s=read(V)
replacement='''    pub fn for_each_variable<'a>(&'a self, mut visit: impl FnMut(&'a Variable)) {
        let mut admission = purrdf_lex::allocation::Resident;
        let mut memory = purrdf_lex::allocation::Memory::new(&mut admission);
        self.for_each_variable_with_memory(&mut memory, |variable, _| {
            visit(variable);
            Ok::<(), core::convert::Infallible>(())
        }).expect("resident term-variable walk allocation failed");
    }

    /// Visit the same variable occurrences under native fallible work-list admission.
    /// Repeated occurrences remain repeated; each quoted predicate is visited before
    /// its subject and object, exactly as in the resident variable walk.
    ///
    /// # Errors
    /// Returns the original visitor error or physical work-list failure.
    pub fn for_each_variable_with_memory<'a, S, E>(
        &'a self, memory: &mut purrdf_lex::allocation::Memory<'_, S>,
        mut visit: impl FnMut(&'a Variable, &mut purrdf_lex::allocation::Memory<'_, S>) -> std::result::Result<(), E>,
    ) -> std::result::Result<(), crate::walk::MutationError<E>>
    where S: purrdf_lex::allocation::Admission + ?Sized,
    {
        use crate::walk::MutationError;
        let mut pending: purrdf_lex::walk::WorkList<&'a Self, 8> = purrdf_lex::walk::WorkList::new();
        pending.try_push_admitted(self, memory).map_err(MutationError::Storage)?;
        while let Some(term) = pending.pop() {
            match term {
                Self::Variable(variable) => visit(variable, memory).map_err(MutationError::Visitor)?,
                Self::Triple(triple) => {
                    if let NamedNodePattern::Variable(variable) = &triple.predicate {
                        visit(variable, memory).map_err(MutationError::Visitor)?;
                    }
                    pending.try_push_admitted(&triple.object, memory).map_err(MutationError::Storage)?;
                    pending.try_push_admitted(&triple.subject, memory).map_err(MutationError::Storage)?;
                }
                Self::NamedNode(_) | Self::BlankNode(_) | Self::Literal(_) => {}
            }
        }
        pending.release_admitted(memory).map_err(MutationError::Storage)
    }'''
replace(V,'for_each_variable',replacement)

E='crates/sparql-eval/src/expr.rs'
s=read(E)
body=body_of(s,'collect_vars')
body=body.replace('pending.extend(', 'pending.try_extend(').replace('out.extend(', 'out.try_extend(').replace('out.insert(', 'out.push(')
# Every original standalone collection growth now propagates the actual native
# refusal. No semantic match arm is removed or narrowed.
for pattern in [r'pending\.(?:push|try_extend)\(',r'out\.(?:push|try_extend)\(']:
    for m in reversed(list(re.finditer(pattern,body))):
        p=body.find('(',m.start(),m.end());end=balanced(body,p,'(',')')
        body=body[:end]+'?'+body[end:]
for receiver in ['tp.subject','tp.object','subject','object','term']:
    borrowed='&'+receiver if '.' in receiver else receiver
    body=body.replace(receiver+'.collect_variables(out);',f'collect_term_vars_admitted({borrowed}, &mut out, &mut memory)?;')
signature='''fn collect_vars_admitted<'a>(
    roots: impl IntoIterator<Item = VarNode<'a>>, workspace: &crate::WorkspaceCapability,
) -> Result<crate::solution::VarSchema, EvalError>'''
native_collector=signature+''' {
    let mut pending = crate::AdmittedVec::new(workspace);
    pending.try_extend(roots)?;
    let mut out = crate::solution::SchemaBuilder::new(workspace);
    let mut frame = crate::workspace::LexicalFrame::new(workspace);
    let mut memory = purrdf_lex::allocation::Memory::new(&mut frame);
'''+body+'''
    out.finish()
}

fn collect_term_vars_admitted(
    term: &purrdf_sparql_algebra::TermPattern,
    out: &mut crate::solution::SchemaBuilder,
    memory: &mut purrdf_lex::allocation::Memory<'_, crate::workspace::LexicalFrame>,
) -> Result<(), EvalError> {
    use purrdf_sparql_algebra::walk::MutationError;
    match term.for_each_variable_with_memory(memory, |variable, _| {
        let _ = out.push(variable.clone())?;
        Ok::<(), EvalError>(())
    }) {
        Ok(()) => Ok(()),
        Err(MutationError::Visitor(error)) => Err(error),
        Err(MutationError::Storage(error)) => Err(memory.admission_mut().storage_error(error, "mentioned term variables")),
    }
}

/// Preserve widened EXISTS and nonnarrowing Project membership under actual
/// native work-list, schema-column, hash-array and immutable-control admission.
pub(crate) fn pattern_all_vars_admitted(
    pattern: &GraphPattern, workspace: &crate::WorkspaceCapability,
) -> Result<crate::solution::VarSchema, EvalError> {
    pattern_vars_outside_admitted(pattern, None, workspace)
}

pub(crate) fn pattern_vars_outside_admitted(
    pattern: &GraphPattern, endpoint: Option<&Variable>, workspace: &crate::WorkspaceCapability,
) -> Result<crate::solution::VarSchema, EvalError> {
    collect_vars_admitted([VarNode::Pattern(pattern, endpoint)], workspace)
}

pub(crate) fn expr_vars_admitted(
    expression: &Expression, workspace: &crate::WorkspaceCapability,
) -> Result<crate::solution::VarSchema, EvalError> {
    collect_vars_admitted([VarNode::Expression(expression)], workspace)
}
'''
replace(E,'collect_vars',native_collector)
replace(E,'expr_vars','''pub(crate) fn expr_vars(expr: &Expression, out: &mut DetHashSet<Variable>) {
    let vars = expr_vars_admitted(expr, &crate::WorkspaceCapability::resident())
        .expect("resident expression-variable collection failed");
    out.extend(vars.vars().iter().cloned());
}''')
replace(E,'pattern_vars_outside','''pub(crate) fn pattern_vars_outside(
    pattern: &GraphPattern, endpoint: Option<&Variable>, out: &mut DetHashSet<Variable>,
) {
    let vars = pattern_vars_outside_admitted(pattern, endpoint, &crate::WorkspaceCapability::resident())
        .expect("resident pattern-variable collection failed");
    out.extend(vars.vars().iter().cloned());
}''')
s=read(E).replace('[`collect_vars`]', '[`collect_vars_admitted`]')
post[E]=s

# Buffer-extension loops live at their existing builder homes, reusing each one's
# one actual checked, preadmitted growth path.
s=read(W);at=s.index('    pub(crate) fn reserve_additional(')
s=s[:at]+'''    pub(crate) fn try_extend(&mut self, values: impl IntoIterator<Item = T>) -> Result<(), EvalError> {
        for value in values { self.push(value)?; }
        Ok(())
    }

    pub(crate) fn from_resident(values: Vec<T>) -> Self {
        Self { values, allocation: None, workspace: WorkspaceCapability::resident() }
    }

'''+s[at:];post[W]=s
S='crates/sparql-eval/src/solution.rs'
s=read(S);at=s.index('    pub(crate) fn finish(self) -> Result<VarSchema, crate::EvalError>')
s=s[:at]+'''    pub(crate) fn try_extend(&mut self, values: impl IntoIterator<Item = Variable>) -> Result<(), crate::EvalError> {
        for value in values { let _ = self.push(value)?; }
        Ok(())
    }

'''+s[at:];post[S]=s

# The fast-path and fresh-name survey keep their existing complete shared walk
# visitors. Only their actual work-list owner changes.
def readonly_walk(b):
    b=b.replace(', walk_pre_post}', '}').replace(', walk_pre_post};','};')
    b=calls(b,{'walk_pre_post':'rewrite_walk'},memory='&rewrite_workspace(memory)')
    return b
native.append(native_from_original('seed_reaches_every_read',
    "fn seed_reaches_every_read_with_memory(query: &Query, probes: &[(Variable, GroundTerm)], memory: &mut RewriteMemory<'_>) -> RewriteResult<bool>",readonly_walk))
native.append(native_from_original('highest_index',
    "fn highest_index_with_memory(pattern: &GraphPattern, prefix: &str, memory: &mut RewriteMemory<'_>) -> RewriteResult<usize>",readonly_walk))

# Read/index scratch has a carrier on the native path; old recursive reference
# fixtures retain their unchanged caller-owned Vec shape.
s=read(P).replace('struct Reads {','struct Reads<R = Vec<usize>> {',1).replace('    plain: Vec<usize>,','    plain: R,',1).replace('    in_exists: Vec<usize>,','    in_exists: R,',1)
post[P]=s
replace(P,'note_reads','''fn note_reads(expr: &Expression, expr_subs: &ExprSubs, reads: &mut Reads) {
    let mut native = Reads {
        plain: crate::AdmittedVec::from_resident(std::mem::take(&mut reads.plain)),
        in_exists: crate::AdmittedVec::from_resident(std::mem::take(&mut reads.in_exists)),
    };
    resident_rewrite(|memory| note_reads_with_memory(expr, expr_subs, &mut native, memory));
    reads.plain = native.plain.try_into_resident().expect("resident reads");
    reads.in_exists = native.in_exists.try_into_resident().expect("resident EXISTS reads");
}''')
replace(P,'substitute_in_expressions','''fn substitute_in_expressions<'e>(
    roots: impl IntoIterator<Item = &'e mut Expression>, expr_subs: &ExprSubs, bodies: &mut Vec<GraphPattern>,
) {
    let mut native = crate::AdmittedVec::from_resident(std::mem::take(bodies));
    resident_rewrite(|memory| substitute_in_expressions_with_memory(roots, expr_subs, &mut native, memory));
    *bodies = native.try_into_resident().expect("resident EXISTS bodies");
}''')
replace(P,'replace_exists_bodies','''fn replace_exists_bodies<'e>(roots: impl IntoIterator<Item = &'e mut Expression>, bodies: Vec<GraphPattern>) {
    resident_rewrite(|memory| replace_exists_bodies_with_memory(roots, crate::AdmittedVec::from_resident(bodies), memory));
}''')
replace(P,'join_assignments_with_prebinding',
    resident('join_assignments_with_prebinding',wrappers['join_assignments_with_prebinding'][0],wrappers['join_assignments_with_prebinding'][1],'.map(|_| ())'))

replace(P,'apply_shacl_prebinding','''pub(crate) fn apply_shacl_prebinding(query: Query, substitutions: Prebindings<'_>) -> Result<Query, RdfDiagnostic> {
    let mut frame = crate::workspace::LexicalFrame::new(&crate::WorkspaceCapability::resident());
    let mut memory = purrdf_lex::allocation::Memory::new(&mut frame);
    apply_shacl_prebinding_with_memory(query, substitutions, &mut memory)
        .map_err(GroundFailure::into_resident_diagnostic)
}''')
replace(P,'apply_shacl_probes','''pub(crate) fn apply_shacl_probes(query: Query, probes: Vec<(Variable, GroundTerm)>) -> Query {
    resident_rewrite(|memory| apply_shacl_probes_with_memory(query, &probes, memory))
}''')
replace(P,'walk_shacl_probes','''pub(crate) fn walk_shacl_probes(query: Query, probes: Vec<(Variable, GroundTerm)>) -> Query {
    resident_rewrite(|memory| walk_shacl_probes_with_memory(query, &probes, memory))
}''')
# The named graph/service substitution is allocation free for every carrier shape.
s=read(P).replace('fn substitute_in_named_node_pattern(pattern: &mut NamedNodePattern, expr_subs: &ExprSubs)',
    'fn substitute_in_named_node_pattern(pattern: &mut NamedNodePattern, expr_subs: &ExprSubs<impl AsRef<[(Variable, GroundTerm)]>>)')
s=s.replace('AggregateExpression, AggregateParts, BlankNode,','AggregateExpression, BlankNode,')
# Private original traversal entry points now have only the fallible native body;
# their callers already go through resident adapters or native entry points above.
for name in ['enter_push','resume_push','enter_substitution','descend_into_inner','resume_substitution']:
    a,b,c=span(s,name);s=s[:a]+s[c:]
post[P]=s
replace(P,'for_each_expression', '''fn for_each_expression<'e>(
    roots: impl IntoIterator<Item = &'e Expression>, mut visit: impl FnMut(&'e Expression),
) {
    resident_rewrite(|memory| for_each_expression_with_memory(roots, |expression, _| {
        visit(expression); Ok(())
    }, memory));
}''')
replace(P,'for_each_node_expression_mut', '''fn for_each_node_expression_mut(node: &mut GraphPattern, f: &mut dyn FnMut(&mut Expression)) {
    resident_rewrite(|memory| for_each_node_expression_mut_with_memory(node, memory, |expression, _| {
        f(expression); Ok(())
    }));
}''')
s=read(P)
a,b,c=span(s,'push_operands');s=s[:a]+s[c:]
post[P]=s
replace(P,'build_probes_into','''pub(crate) fn build_probes_into(
    probes: &mut Vec<(Variable, GroundTerm)>, substitutions: Prebindings<'_>,
) -> Result<(), RdfDiagnostic> {
    let mut native = PreparedProbes::from_resident(std::mem::take(probes))
        .map_err(GroundFailure::into_resident_diagnostic)?;
    let result = build_prepared_probes(&mut native, substitutions, &crate::WorkspaceCapability::resident());
    *probes = native.try_into_resident().expect("resident prepared probe arrays");
    result.map_err(GroundFailure::into_resident_diagnostic)
}''')
s=read(P)
for name in ['ground','ground_reusing']:
    a,b,c=span(s,name);s=s[:a]+s[c:]
# Test-facing resident adapters are never available to bounded shipping callers.
for name,(signature,_) in wrappers.items():
    if signature.startswith('pub('): continue
    a,b,c=span(s,name);s=s[:a]+'#[cfg(test)]\n'+s[a:]
for name in ['for_each_expression','for_each_node_expression','for_each_node_expression_mut',
             'probe_positions','seed_row','seed_of','stand_in']:
    a,b,c=span(s,name);s=s[:a]+'#[cfg(test)]\n'+s[a:]
# Existing resident accumulating index adapters explicitly adopt their caller-owned
# buffer under the resident callback before the common native reserve body runs.
for name in ['probe_term_pattern','probe_triple_pattern']:
    a,b,c=span(s,name)
    old=s[a:c]
    old=old.replace('resident_rewrite(|memory| ','''resident_rewrite(|memory| {
        let bytes = std::alloc::Layout::array::<usize>(probed.capacity())
            .map_err(|_| crate::EvalError::WorkspaceBoundOverflow)?.size();
        let result = memory.add_bytes(bytes);
        native_storage(memory, result)?;
        ''')
    old=old[:-1].rstrip()
    # The old forwarding closure ended with `)`; make its block explicit.
    old=old[:-1]+'\n    })\n}'
    s=s[:a]+old+s[c:]
# Update private intra-doc links to their shipping native homes. The independent
# recursive oracle bodies and all original semantic assertions remain untouched.
lines=s.splitlines(True)
for i,line in enumerate(lines):
    if not line.lstrip().startswith('//'): continue
    for old,new in native_names.items():
        if old in wrappers and not wrappers[old][0].startswith('pub('):
            line=line.replace('[`'+old+'`]', '[`'+new+'`]')
    line=line.replace('[`Self::ground`]', '[`Self::ground_with_memory`]')
    line=line.replace('[`Prebindings::ground_reusing`]', '[`Prebindings::ground_reusing_with_memory`]')
    lines[i]=line
s=''.join(lines)
# Aggregate constructor names are referenced only by the independent test oracle;
# shipping mutable slots are visited through the checked aggregate's own iterator.
s=s.replace('    AggregateExpression, BlankNode, Expression, GraphPattern, GroundTerm,',
            '    BlankNode, Expression, GraphPattern, GroundTerm,',1)
s=s.replace('use purrdf_sparql_algebra::{Chain, Child};',
            'use purrdf_sparql_algebra::{Chain, Child};\n#[cfg(test)]\nuse purrdf_sparql_algebra::AggregateExpression;',1)
# New bodies go before the existing test module, leaving the independent recursive
# oracle source unmodified.
at=s.index('/// Convert a caller-owned term through the same native grounding body.')
unit='\n\n'+support+'\n\n'+leaves+'\n\n'+expressions+'\n\n'+scopes+'\n\n'+'\n\n'.join(native)+'\n\n'
s=s[:at]+unit+s[at:]
# The independent recursive oracles use only the retained test adapters below.
# Do not leave obsolete unused resident traversal/constructor doors in the native
# module after all shipping callers and reference fixtures move to the same body.
for name in ['push_probe_constants', 'probe_call_arguments', 'drive_call_arguments',
             'call_driver', 'probe_triple_pattern', 'seed_of',
             'seed_above_a_lone_sub_select', 'seed_every_sub_select',
             'seed_sub_selects_in', 'seed_both_minus_operands', 'seed_minus_in',
             'map_patterns_in_query', 'finish_own_expressions',
             'drive_expression_reads', 'highest_index', 'join_assignments_in',
             'compatible_with_binding']:
    a,b,c=span(s,name)
    before=s[:a]
    # Remove its contiguous documentation and cfg attribute with the obsolete
    # adapter, avoiding attributes accidentally attaching to the following item.
    lines=before.splitlines(True)
    while lines and (lines[-1].lstrip().startswith('///')
                     or lines[-1].strip() == '#[cfg(test)]'):
        lines.pop()
    s=''.join(lines)+s[c:]
a,b,c=span(s,'holds');s=s[:a]+'    #[cfg(test)]\n'+s[a:]
post[P]=s

# Reuse the current concrete non-latching reservation fixture for owner regressions.
fixtures=(STAGE/'prebinding-rewrite-fixtures.rs').read_text()
fixtures=fixtures[fixtures.index('#[test]'):]
s=read(W);at=s.index('mod owned_row_regressions {');brace=s.index('{',at);end=balanced(s,brace)
s=s[:end-1]+'\n\n'+fixtures+'\n'+s[end-1:];post[W]=s

# Memory resumption certifies the already-live caller payload before native growth.
s=read(L)
s+='''

#[cfg(test)]
mod resumed_memory_regressions {
    use super::{Admission, Memory, StorageError};

    #[derive(Default)]
    struct OriginalGrant { live: usize, limit: usize, attempted: usize }
    impl Admission for OriginalGrant {
        fn resize(&mut self, live: usize) -> Result<(), StorageError> {
            self.attempted = live;
            if live > self.limit { return Err(StorageError::AdmissionFailed); }
            self.live = live;
            Ok(())
        }
    }

    #[test]
    fn resume_preserves_original_payload_during_refused_buffer_growth() {
        let mut grant = OriginalGrant { limit: 44, ..OriginalGrant::default() };
        let mut original = Vec::<u8>::new();
        let live = {
            let mut memory = Memory::new(&mut grant);
            memory.reserve(&mut original, 13).expect("original admitted array");
            memory.live_bytes()
        };
        assert_eq!(live, 13);
        let mut next = Vec::<u8>::new();
        {
            let mut memory = Memory::resume(&mut grant, live);
            assert_eq!(memory.reserve(&mut next, 32), Err(StorageError::AdmissionFailed));
            assert_eq!(memory.live_bytes(), live);
            assert_eq!(next.capacity(), 0, "refused before allocating replacement");
        }
        assert_eq!(grant.attempted, 45, "original and new requested layouts overlap");
        assert_eq!(grant.live, 13, "original payload stayed covered");
        let mut memory = Memory::resume(&mut grant, live);
        memory.release_vec(original).expect("payload dies before its original admission");
        assert_eq!(memory.live_bytes(), 0);
    }
}
''';post[L]=s

def emit():
    chunks=[]
    for path,s in post.items():
        target=OUT/path
        target.parent.mkdir(parents=True,exist_ok=True)
        target.write_text(s)
        chunks.extend(difflib.unified_diff(base[path].splitlines(True),s.splitlines(True),
            fromfile='a/'+path,tofile='b/'+path))
    (STAGE/'prebinding-rewrite-owner-draft.patch').write_text(''.join(chunks))

emit()
