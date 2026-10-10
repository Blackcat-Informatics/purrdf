# Why not Rust: This ignored Stage assembler joins authored Rust owner proposals with current source context to form a reviewable diff; it is not shipped repository tooling.
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
from pathlib import Path
import difflib

root = Path.cwd()
stage = root / '.stage/sparql-eval-complete-bounded-workspace'
files = {}

def replace(text, old, new, expected=1):
    count = text.count(old)
    if count != expected:
        raise RuntimeError(f'Expected {expected} occurrences, found {count}: {old[:90]!r}')
    return text.replace(old, new)

def section(text, start, end, new):
    a = text.index(start)
    b = text.index(end, a)
    return text[:a] + new + text[b:]

expr_path = 'crates/sparql-eval/src/expr.rs'
expr = (root / expr_path).read_text()
expr = replace(expr, '/// Materialize a solution term to an owned value.\npub(crate) fn value_of',
               '/// Resident test interpreter adapter; production retains the owned carrier.\n#[cfg(test)]\npub(crate) fn value_of')
expr = expr.replace('let av = value_of(ctx, ta)?;', 'let av = owned_value_of(ctx, ta)?;')
expr = expr.replace('let bv = value_of(ctx, tb)?;', 'let bv = owned_value_of(ctx, tb)?;')
expr = replace(expr, 'let cv = value_of(ctx, candidate)?;', 'let cv = owned_value_of(ctx, candidate)?;')
expr = replace(expr, 'value_holds_nan(&value_of(ctx, term)?)', 'value_holds_nan(&owned_value_of(ctx, term)?)')
expr = section(expr, '/// A term read as a string argument:', '/// Extract `(lexical, language)`',
               (stage / 'expr-owned-string-carrier-draft.rs').read_text() + '\n')
expr = replace(expr, 'fn string_arg(vals: &[Option<TermValue>], i: usize) -> Option<(String, Option<String>)> {\n    string_arg_value(arg(vals, i)?).map(|(s, l, _)| (s, l))\n}',
               'fn string_arg(vals: &[Option<TermValue>], i: usize) -> Option<(&str, Option<&str>)> {\n    string_arg3_ref(vals, i).map(|(lexical, language, _)| (lexical, language))\n}')
# Remove the copied triple-facet extractor. The one borrowed datatype classifier remains.
expr = section(expr, '/// Like [`string_arg`] but also returns', 'fn string_arg3_ref', '')
expr = section(expr, 'fn string_arg_value(\n', '/// Apply a pure string transform', '')
# Existing REPLACE output is a separate root-owned native publication unit; only input copying changes here.
expr = replace(expr, 'let Some((s, lang, dir)) = string_arg3(vals, 0) else {',
               'let Some((s, lang, dir)) = string_arg3_ref(vals, 0) else {')
expr = replace(expr, 'make_string_dir(ctx, replaced, lang, dir)',
               'make_string_dir(ctx, replaced, lang.map(str::to_owned), dir)')
# Producers already pre-admit their exact lexical/facet payload; transfer that same grant, not a second arena charge.
expr = replace(expr,
               '    let before = ctx.scratch.owned_value_count();\n    let term = intern_leaf(ctx, value)?;\n    if ctx.scratch.owned_value_count() > before { ctx.keep_computed(admission); }\n    Ok(term)',
               '    ctx.intern_workspace_term(crate::WorkspaceTerm::new(value, admission))', expected=2)
expr = replace(expr,
               '    let before = ctx.scratch.owned_value_count();\n    let term = intern_leaf(ctx, TermValue::Literal { lexical_form: out, datatype, language, direction })?;\n    if ctx.scratch.owned_value_count() > before { ctx.keep_computed(admission); }\n    Ok(term)',
               '    ctx.intern_workspace_term(crate::WorkspaceTerm::new(\n        TermValue::Literal { lexical_form: out, datatype, language, direction }, admission))')
files[expr_path] = expr

vm_path = 'crates/sparql-eval/src/vm/mod.rs'
vm = (root / vm_path).read_text()
vm = replace(vm, 'Str(Option<helpers::StringArg>)', 'Str(Option<helpers::OwnedStringArg>)')
vm = replace(vm, 'Value(Option<TermValue>)', 'Value(Option<crate::WorkspaceTerm>)')
vm = replace(vm, '        value: TermValue,', '        value: crate::WorkspaceTerm,')
vm = replace(vm, '        vals: Vec<Option<TermValue>>,', '        vals: Arguments,')
vm = replace(vm, '    args: Vec<Option<TermValue>>,', '    args: Arguments,')
vm = replace(vm, '            std::alloc::Layout::array::<Option<TermValue>>(args_capacity),\n', '')
vm = replace(vm,
             '        let mut args = Vec::new();\n        args.try_reserve_exact(args_capacity).map_err(|_| EvalError::AllocationFailed { construct: "VM argument cells" })?;',
             '        let args = Arguments::with_capacity(args_capacity, &ctx.growth)?;')
vm = replace(vm, '            args: Vec::new(),', '            args: Arguments::new(&crate::WorkspaceCapability::resident()),')
vm = vm.replace('helpers::value_of(ctx,', 'helpers::owned_value_of(ctx,')
vm = replace(vm,
             '                    let value = helpers::apply_function(\n                        &program.calls[call as usize],\n                        args,\n                        ctx,\n                        compiled,\n                    )?;\n                    stack.push(Val::Term(value));',
             '                    let result = helpers::apply_function(\n                        &program.calls[call as usize], args, ctx, compiled,\n                    );\n                    args.clear();\n                    stack.push(Val::Term(result?));')
vm = replace(vm,
             '                    let value = match call_custom(program, call, args, ctx)? {\n                        VmStep::Value(value) => value,\n                        VmStep::Suspend(suspend) => resolve(suspend, program, row, schema, ctx)?,\n                    };\n                    stack.push(Val::Term(value));',
             '                    let result = match call_custom(program, call, args, ctx) {\n                        Ok(VmStep::Value(value)) => Ok(value),\n                        Ok(VmStep::Suspend(suspend)) => resolve(suspend, program, row, schema, ctx),\n                        Err(error) => Err(error),\n                    };\n                    args.clear();\n                    stack.push(Val::Term(result?));')
vm = replace(vm, 'let value = helpers::triple_value(subject, predicate, object);',
             'let value = helpers::triple_value_admitted(subject, predicate, object, &ctx.growth)?;')
vm = vm.replace('.map(|value| helpers::intern(ctx, value))', '.map(|value| ctx.intern_workspace_term(value))', 1)
vm = vm.replace('                            .flatten()\n                            .map(|s| (s, None, None)),', '                            .flatten(),')
vm = replace(vm, '    vals: &[Option<TermValue>],\n    ctx: &mut EvalCtx', '    vals: &mut Arguments,\n    ctx: &mut EvalCtx')
vm = replace(vm, '            vals: vals.to_vec(),', '            vals: vals.take(&ctx.growth),')
vm = replace(vm, '    vals: &mut Vec<Option<TermValue>>,', '    vals: &mut Arguments,')
vm = replace(vm, '        vals.push(term.map(|t| helpers::owned_value_of(ctx, t)).transpose()?);',
             '        vals.push(term.map(|t| helpers::owned_value_of(ctx, t)).transpose()?)?;')
# Borrowed/owned string carrier: no Cow-owned tuple with detached text allocations.
vm = section(vm, '/// A string argument as an instruction reads it:', 'fn underflow()', (stage / 'vm-string-view-draft.rs').read_text() + '\n')
vm = vm.replace('h.1.as_deref()', 'h.parts().1').replace('h.2,', 'h.parts().2,')
vm = vm.replace('n.1.as_deref()', 'n.parts().1').replace('n.2,', 'n.parts().2,')
vm = replace(vm, '(h.0.as_str(), n.0.as_str())', '(h.parts().0, n.parts().0)')
vm = vm.replace('f.1.is_none()', 'f.parts().1.is_none()').replace('p.1.is_none()', 'p.parts().1.is_none()')
vm = replace(vm, 'Some(std::borrow::Cow::Borrowed(&NO_FLAGS))', 'Some(StrArg::Borrowed(&NO_FLAGS))')
vm = replace(vm, 'let flags = flags.0.as_str();', 'let flags = flags.parts().0;')
vm = replace(vm, '&pattern.0,', 'pattern.parts().0,')
vm = replace(vm, '.is_match(&text.0)?', '.is_match(text.parts().0)?')
vm = replace(vm, 'tag.1.is_none() && range.1.is_none()', 'tag.parts().1.is_none() && range.parts().1.is_none()')
vm = replace(vm, 'helpers::lang_matches(&tag.0, &range.0)', 'helpers::lang_matches(tag.parts().0, range.parts().0)')
vm = replace(vm, '/// The value stack. On the heap rather than inline:', (stage / 'vm-owned-arguments-draft.rs').read_text() + '\n\n/// The value stack. On the heap rather than inline:')
files[vm_path] = vm

tests_path = 'crates/sparql-eval/src/vm/tests.rs'
tests = (root / tests_path).read_text()
tests = replace(tests, '.and_then(|term| helpers::str_lexical_term(ctx, term).unwrap()),',
                '.and_then(|term| helpers::str_lexical_term(ctx, term).unwrap())\n                        .map(|value| value.parts().0.to_owned()),')
tests = replace(tests, '.and_then(|term| helpers::lang_lexical_term(ctx, term).unwrap()),',
                '.and_then(|term| helpers::lang_lexical_term(ctx, term).unwrap())\n                        .map(|value| value.parts().0.to_owned()),')
tests = replace(tests, 'Ok(helpers::string_arg_of_term(ctx, term).unwrap())',
                'Ok(helpers::string_arg_of_term(ctx, term).unwrap().map(|value| {\n                    let (lexical, language, direction) = value.parts();\n                    (lexical.to_owned(), language.map(str::to_owned), direction)\n                }))')
files[tests_path] = tests

# Native triple boxing uses the original small::try_boxed home and retained term layout walk.
files[expr_path] = replace(files[expr_path], '/// Extract a component of a triple term',
                          (stage / 'expr-owned-triple-draft.rs').read_text() + '\n\n/// Extract a component of a triple term')
files[expr_path] = replace(files[expr_path],
    '    let triple = triple_value(\n        arg(vals, 0).cloned(),\n        arg(vals, 1).cloned(),\n        arg(vals, 2).cloned(),\n    );\n    Ok(triple\n        .map(|triple| intern(ctx, triple))\n        .transpose()?\n        .flatten())',
    '    let subject = arg(vals, 0).map(|value| ctx.growth.clone_term(value)).transpose()?;\n'
    '    let predicate = arg(vals, 1).map(|value| ctx.growth.clone_term(value)).transpose()?;\n'
    '    let object = arg(vals, 2).map(|value| ctx.growth.clone_term(value)).transpose()?;\n'
    '    let triple = triple_value_admitted(subject, predicate, object, &ctx.growth)?;\n'
    '    triple.map(|value| ctx.intern_workspace_term(value)).transpose().map(Option::flatten)')
files[expr_path] = replace(files[expr_path], 'pub(crate) fn triple_value(\n', '#[cfg(test)]\npub(crate) fn triple_value(\n')
files[expr_path] = replace(files[expr_path],
    '    pick: impl Fn(TermValue, TermValue, TermValue) -> TermValue,',
    "    pick: impl for<'a> Fn(&'a TermValue, &'a TermValue, &'a TermValue) -> &'a TermValue,")
files[expr_path] = replace(files[expr_path],
    '            let part = pick((**s).clone(), (**p).clone(), (**o).clone());\n            Ok(intern(ctx, part)?)',
    '            let part = ctx.growth.clone_term(pick(s, p, o))?;\n            ctx.intern_workspace_term(part)')

patch = []
for path, candidate in files.items():
    original = (root / path).read_text()
    destination = stage / 'expr-vm-owner-postimage' / path
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(candidate)
    patch.extend(difflib.unified_diff(original.splitlines(keepends=True), candidate.splitlines(keepends=True),
                                   fromfile='a/' + path, tofile='b/' + path))
(stage / 'expr-vm-string-owner-draft.patch').write_text(''.join(patch))
print('Wrote Stage expr-vm-string-owner-draft.patch; proposal only, not compiled/run.')
