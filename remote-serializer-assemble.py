from pathlib import Path
import re
root=Path.cwd(); stage=root/'.stage/sparql-eval-complete-bounded-workspace';post=stage/'remote-native-owner-postimages'
def save(path,text):
 p=post/path;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(text)
s=(root/'crates/sparql-algebra/src/serialize.rs').read_text()
s=s.replace('use crate::walk::{Flow, NodeRef, Visit, walk_pre_post};','use crate::walk::{Flow, NodeRef, Visit};\nuse crate::parser::table::ScopeTable;\nuse purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};')
a=s.index("/// The rendering's work list:");b=s.index('/// A `GROUP BY` key',a)
s=s[:a]+(stage/'remote-serializer-storage.rs').read_text()+'\n'+s[b:]
a=s.index('#[derive(Default)]\nstruct RawBlankGroup');b=s.index('/// The shared complete-query renderer',a)
old=s[a:b]
brace=old[old.index('/// The brace decisions'):old.index('/// Fresh legal names')]
brace=brace.replace('fn spine_child_needs_bracing(parent: &GraphPattern, child: &GraphPattern) -> bool {',"fn spine_child_needs_bracing_with_memory<S: Admission + ?Sized>(parent: &GraphPattern, child: &GraphPattern, memory: &mut Memory<'_, S>) -> Result<bool, StorageError> {\n    Ok(")
brace=brace.replace('join_right_needs_bracing(child)','join_right_needs_bracing_with_memory(child, memory)?')
brace=brace.rstrip();assert brace.endswith('}');brace=brace[:-1].rstrip()+')\n}\n'
s=s[:a]+(stage/'remote-serializer-names.rs').read_text()+'\n'+brace+s[b:]
a=s.index('fn select_query(');b=s.index('/// Whether `p`, with no `Project`',a)
old=s[a:b]
select=old[old.index('fn select_query('):old.index('/// Render `p` as the body')]
select=select.replace("fn select_query(inner: &GraphPattern, predicates: PredicateRendering<'_>) -> String {", "fn select_query_with_memory<S: Admission + ?Sized>(inner: &GraphPattern, predicates: PredicateRendering<'_>, memory: &mut Memory<'_, S>) -> Result<String, CarrierError> {\n    let before_names = memory.live_bytes();")
select=select.replace('let names = VariableNames::for_pattern(inner, |_| {});','let names = VariableNames::for_pattern(inner, |_| {}, memory)?;\n    let names_bytes = memory.live_bytes().checked_sub(before_names).ok_or(StorageError::SizeOverflow)?;\n    let storage = RenderStorage::new(memory);')
select=select.replace('let mut s = String::new();','let mut text = String::new();\n    let mut s = Surface { text: &mut text, storage: &storage };')
select=select.replace('emit(&mut s, Item::Subselect(inner), predicates, &names);','emit(&mut s, Item::Subselect(inner), predicates, &names)?;')
select=select.replace('emit(&mut s, Item::BracedGroup(inner), predicates, &names);','emit(&mut s, Item::BracedGroup(inner), predicates, &names)?;')
select=select.replace('    s\n}', '    storage.check()?;\n    drop(names);\n    storage.cleanup(|memory| memory.release_bytes(names_bytes));\n    storage.check()?;\n    Ok(text)\n}')
s=s[:a]+select+'''/// Render an UPDATE group through the same native surface engine.
pub(crate) fn fmt_group_body(s: &mut String, p: &GraphPattern, reserve: impl FnOnce(&mut dyn FnMut(&Variable))) -> core::fmt::Result {
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    validate_carrier_pattern_with_memory(p, &mut memory).map_err(|_| core::fmt::Error)?;
    let before = memory.live_bytes();
    let names = VariableNames::for_pattern(p, reserve, &mut memory).map_err(|_| core::fmt::Error)?;
    let bytes = memory.live_bytes().checked_sub(before).ok_or(core::fmt::Error)?;
    let storage = RenderStorage::new(&mut memory);
    emit(&mut Surface { text: s, storage: &storage }, Item::GroupBody(p), PredicateRendering::Independent, &names).map_err(|_| core::fmt::Error)?;
    drop(names); storage.cleanup(|memory| memory.release_bytes(bytes));
    storage.check().map_err(|_| core::fmt::Error)
}
/// Render a path through the same native item engine.
pub(crate) fn fmt_path(s: &mut String, path: &PropertyPathExpression) {
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let storage = RenderStorage::new(&mut memory);
    emit(&mut Surface { text: s, storage: &storage }, Item::Path(path), PredicateRendering::Independent, &VariableNames::default()).expect("resident path rendering storage");
}

'''+s[b:]
a=s.index('fn contains_property_function(');b=s.index('/// Does a [`GraphPattern::Lateral`]',a)
old=s[a:b]
old=old.replace('fn contains_property_function(p: &GraphPattern) -> bool {', "fn contains_property_function_with_memory<S: Admission + ?Sized>(p: &GraphPattern, memory: &mut Memory<'_, S>) -> Result<bool, StorageError> {")
old=old.replace('walk_pre_post(NodeRef::Pattern(p), |visit, node| match (visit, node) {','crate::walk::walk_pre_post_with_memory(NodeRef::Pattern(p), |visit, node, _| Ok::<_, StorageError>(match (visit, node) {')
old=old.replace('    });\n    found','    }), memory)?;\n    Ok(found)')
s=s[:a]+old+s[b:]
a=s.index('fn join_right_needs_bracing(');b=s.index('const fn left_operand_needs_bracing',a)
s=s[:a]+'''fn join_right_needs_bracing_with_memory<S: Admission + ?Sized>(pattern: &GraphPattern, memory: &mut Memory<'_, S>) -> Result<bool, StorageError> {
    Ok(!is_subselect_node(pattern) && (contains_property_function_with_memory(pattern, memory)? || rendering_starts_with_a_reabsorbable_left(pattern)))
}

'''+s[b:]
a=s.index('fn emit(');b=s.index('pub(crate) fn function_keyword',a)
body=s[a:b]
names=['emit','render','triple_items','group_body','property_function','property_function_args','values','subselect','order_keys','expr_bare','relational','in_list','expr_list','aggregate','path_items','path_chain','fmt_leaf_term','fmt_named_node_pattern','fmt_literal','fmt_function_name']
for name in names:
 body=re.sub(r'fn '+name+r"(<'a>)?\(",lambda m:'fn '+name+("<'a, S: Admission + ?Sized>" if m.group(1) else '<S: Admission + ?Sized>')+'(',body,count=1)
body=body.replace('&mut String',"&mut Surface<'_, '_, '_, S>")
body=body.replace("&mut Items<'a>","&mut Items<'a, '_, '_, '_, S>")
body=body.replace('let mut stack = Items::with(first);','let mut stack = Items::with(first, s.storage);')
body=body.replace('    names: &VariableNames,\n) {\n    let mut stack', '    names: &VariableNames,\n) -> Result<(), StorageError> {\n    let mut stack',1)
body=body.replace('        render(s, item, &mut stack, predicates, names);','        render(s, item, &mut stack, predicates, names);\n        s.storage.check()?;',1)
body=body.replace('        stack.reverse_top(stack.len() - queued);\n    }\n}', '        stack.reverse_top(stack.len() - queued);\n    }\n    drop(stack);\n    s.storage.check()\n}',1)
body=body.replace('join_right_needs_bracing(p)', 's.storage.run(|memory| join_right_needs_bracing_with_memory(p, memory)).unwrap_or(false)')
body=body.replace('select_exprs.push((variable, expression));','s.storage.run(|memory| memory.push(&mut select_exprs, (variable, expression)));')
body=body.replace('having.push(expr);','s.storage.run(|memory| memory.push(&mut having, expr));')
body=body.replace('Some(vars.to_vec())','s.storage.run(|memory| memory.collect(vars.iter().cloned()))')
body=body.replace('Some(crate::parser::visible_variables(cur))','s.storage.run(|memory| crate::parser::visible_variables_with_memory(cur, memory))')
start=body.index('    let as_targets:');end=body.index('    let emit_filtered_vars',start)
body=body[:start]+'''    let mut as_targets = ScopeTable::default();
    for (variable, _) in &select_exprs { s.storage.run(|memory| as_targets.insert(*variable, (), memory)); }
'''+body[end:]
body=body.replace('as_targets.contains(v)', 'as_targets.get(&v).is_some()')
body=body.replace('        for expr in having {','        for &expr in &having {')
start=body.index('fn subselect');end=body.index('/// Queue `ORDER BY`',start)
sub=body[start:end];last=sub.rfind('\n}')
sub=sub[:last]+'''\n    s.storage.cleanup(|memory| memory.release_vec(select_exprs));
    s.storage.cleanup(|memory| memory.release_vec(having));
    s.storage.cleanup(|memory| as_targets.release(memory));
    if let Some(variables) = no_project_vars { s.storage.cleanup(|memory| memory.release_vec(variables)); }
'''+sub[last:]
body=body[:start]+sub+body[end:]
s=s[:a]+body+s[b:]
s=s.replace('    validate_carrier_pattern(inner)?;\n    Ok(select_query(inner, PredicateRendering::Independent))','    resident_carrier(inner, PredicateRendering::Independent)',1)
s=s.replace('    validate_carrier_pattern(inner)?;\n    Ok(select_query(inner, PredicateRendering::Configured(options)))','    resident_carrier(inner, PredicateRendering::Configured(options))',1)
a=s.index('pub(crate) fn validate_carrier_pattern(');b=s.index('/// An ordinary blank can retain blank syntax',a)
s=s[:a]+(stage/'remote-serializer-public.rs').read_text()+'\n'+s[b:]
save('crates/sparql-algebra/src/serialize.rs',s)
s=(root/'crates/sparql-algebra/src/lib.rs').read_text().replace('    pattern_to_select_query, pattern_to_select_query_with_options, try_pattern_to_select_query,','    CarrierError, try_pattern_to_select_query_with_memory,\n    pattern_to_select_query, pattern_to_select_query_with_options, try_pattern_to_select_query,',1)
save('crates/sparql-algebra/src/lib.rs',s)
s=(root/'crates/sparql-algebra/src/parser/table.rs').read_text()
s=s.replace('pub(crate) fn iter(&self) -> impl Iterator<Item = (&K, &V)> {','pub(crate) fn iter(&self) -> impl Iterator<Item = (&K, &V)> + Clone {',1)
p=s.index('    /// Destroy this arena before releasing')
s=s[:p]+'''    /// Exact original arena capacity, for a containing owner's destruction.
    pub(crate) fn storage_bytes(&self) -> Result<usize, StorageError> {
        core::alloc::Layout::array::<Slot<K, V>>(self.slots.capacity()).map(|layout| layout.size()).map_err(|_| StorageError::SizeOverflow)
    }

'''+s[p:]
save('crates/sparql-algebra/src/parser/table.rs',s)
print('serializer postimage: ',len((post/'crates/sparql-algebra/src/serialize.rs').read_text()))
