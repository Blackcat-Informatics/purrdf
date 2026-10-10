from pathlib import Path
import hashlib
import json
import re

root = Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
stage = root / '.stage/sparql-eval-complete-bounded-workspace'
post = stage / 'evaluator-clippy-correction-5-postimages'
names = 'eval.rs expr.rs binop.rs deferred_exists.rs enf.rs substitute.rs remote.rs remote_http.rs service_endpoints.rs xpath_regex.rs parallel.rs scratch.rs'.split()
original = {name: (root / 'crates/sparql-eval/src' / name).read_text() for name in names}
texts = dict(original)
findings = json.loads((stage / 'evaluator-clippy-correction-5-findings.json').read_text())

# The diagnostic line numbers refer to these exact, unchanged source snapshots.
# Apply local syntax substitutions from the bottom upwards so source positions
# remain stable for every other finding in the same file.
simple = {'semicolon_if_nothing_returned', 'explicit_iter_loop', 'default_trait_access', 'ignored_unit_patterns', 'unnecessary_semicolon'}
for name in names:
    lines = texts[name].splitlines(True)
    sites = {(f['line'], f['lint']) for f in findings if f['file'] == name and f['lint'] in simple}
    for line, lint in sorted(sites, reverse=True):
        at = line - 1
        before = lines[at]
        if lint == 'semicolon_if_nothing_returned':
            if (before.rstrip().endswith(('?', ')')) and not lines[at + 1].lstrip().startswith('.')) or (name == 'enf.rs' and before.strip() == 'original = left'):
                lines[at] = before.rstrip('\n') + ';\n'
            else:
                cursor = at
                while not lines[cursor].strip() in ('})', '.expect("resident endpoint oracle note")'):
                    cursor += 1
                    if cursor > at + 12:
                        raise RuntimeError(f'multiline semicolon: {name}:{line}')
                lines[cursor] = lines[cursor].rstrip('\n') + ';\n'
        elif lint == 'explicit_iter_loop':
            match = re.search(r'\bin (.+)\.iter\(\)(?= \{)', before)
            if not match:
                raise RuntimeError(f'iteration finding not matched: {name}:{line}: {before}')
            prefix = '' if name == 'eval.rs' and match.group(1) == 'bucket' else '&'
            lines[at] = before[:match.start(1)] + prefix + match.group(1) + before[match.end(0):]
        elif lint == 'default_trait_access':
            assert 'Default::default()' in before, (name, line, before)
            lines[at] = before.replace('Default::default()', 'purrdf_lex::walk::WorkList::new()')
        elif lint == 'ignored_unit_patterns':
            assert '|(variable, _)|' in before, (name, line, before)
            lines[at] = before.replace('|(variable, _)|', '|(variable, ())|')
        elif lint == 'unnecessary_semicolon' and name != 'deferred_exists.rs':
            assert before.strip() == '};', (name, line, before)
            lines[at] = before.replace('};', '}')
    texts[name] = ''.join(lines)

def replace(name, before, after, count=1):
    actual = texts[name].count(before)
    if actual != count:
        raise RuntimeError(f'{name}: expected {count} matches, found {actual}: {before!r}')
    texts[name] = texts[name].replace(before, after)

def expectation(name, marker, lint, reason, count=1):
    indentation = re.match(r' *', marker).group()
    attribute = f'{indentation}#[expect(\n{indentation}    clippy::{lint},\n{indentation}    reason = "{reason}"\n{indentation})]\n'
    replace(name, marker, attribute + marker, count)

replace('binop.rs', '.last().map(|map| map.as_ref())', '.last().map(AsRef::as_ref)')
# Only the final use in this delivered block moves its schema; other block uses
# require their shallow immutable clone and remain unchanged.
replace('binop.rs', '''                let rows = SolutionSeq {
                    schema: out.clone(),
                    rows: output.finish()?,
                };''', '''                let rows = SolutionSeq {
                    schema: out,
                    rows: output.finish()?,
                };''')
replace('binop.rs', '''            tripped,
            joined.schema.clone(),
''', '''            tripped,
            joined.schema,
''')

replace('eval.rs', '''            entries.insert_admitted(hash, bucket, workspace)?;
        }
        Ok(id)''', '''            entries.insert_admitted(hash, bucket, workspace)?;
        }
        drop(entries);
        Ok(id)''')
replace('eval.rs', '&format_args!(\n                    "blank-node mint prefix', 'format_args!(\n                    "blank-node mint prefix')
replace('eval.rs', 'std::ptr::eq(&**previous, &*governors)', 'std::ptr::eq(&raw const **previous, &raw const *governors)')
replace('eval.rs', '    #[must_use]\n    pub(crate) fn fork_for_worker(&self)', '    pub(crate) fn fork_for_worker(&self)')

replace('deferred_exists.rs', '''            };
            SitesRef::Owned(kept)
''', '''            };
            drop(table);
            SitesRef::Owned(kept)
''')
replace('deferred_exists.rs', '''                    if failure.is_none()
                        && let crate::blank_scope::ChildMut::Pattern(child) = child
                    {
                        if let Err(error) = pending.push(child) {
                            failure = Some(error);
                        }
                    }''', '''                    if failure.is_none()
                        && let crate::blank_scope::ChildMut::Pattern(child) = child
                        && let Err(error) = pending.push(child)
                    {
                        failure = Some(error);
                    }''')
replace('deferred_exists.rs', '''        crate::expr::correlated_box(copy, &mut memory)?
    };''', '''        let copy = crate::expr::correlated_box(copy, &mut memory)?;
        drop(frame);
        copy
    };''')
replace('deferred_exists.rs', '''            &mut memory,
        )?;
        pieces.push(Piece { body, parent })?;''', '''            &mut memory,
        )?;
        drop(frame);
        pieces.push(Piece { body, parent })?;''')
replace('deferred_exists.rs', '''                &mut memory,
            )?;
        }
        let schema =''', '''                &mut memory,
            )?;
            drop(frame);
        }
        let schema =''')
replace('deferred_exists.rs', '''                        if remember_failure(
                            pending.push(match part {
                                PatternPart::Child(child, _) => Node::Pattern(child, scope),
                                PatternPart::Expression(expr) => Node::Expression(expr, scope),
                            }),
                            &mut failure,
                        ) {
                            return true;
                        };
                        false''', '''                        remember_failure(
                            pending.push(match part {
                                PatternPart::Child(child, _) => Node::Pattern(child, scope),
                                PatternPart::Expression(expr) => Node::Expression(expr, scope),
                            }),
                            &mut failure,
                        )''')

expectation('enf.rs', 'pub(crate) enum Enf {', 'large_enum_variant', 'normalization returns the original admitted tree inline; adding a box would introduce a separate allocation and change its owner layout')

replace('expr.rs', '''            ) =>
            {
                if parse_owned_xsd(lexical_form, datatype, workspace)?
                    .as_deref()
                    .is_some_and(is_xsd_nan)
                {
                    return Ok(true);
                }
            }''', '''            ) && parse_owned_xsd(lexical_form, datatype, workspace)?
                .as_deref()
                .is_some_and(is_xsd_nan) =>
            {
                return Ok(true);
            }''')
replace('expr.rs', '.is_some_and(|kind| kind.is_numeric())', '.is_some_and(XsdDatatype::is_numeric)')
replace('expr.rs', 'adjust_datetime_to_timezone(&dt, tz)', 'adjust_datetime_to_timezone(dt, tz)')
replace('expr.rs', 'adjust_date_to_timezone(&d, tz)', 'adjust_date_to_timezone(d, tz)')
replace('expr.rs', 'adjust_time_to_timezone(&t, tz)', 'adjust_time_to_timezone(t, tz)')
replace('expr.rs', 'well_formed_langtag(&lang)', 'well_formed_langtag(lang)', 2)
replace('expr.rs', 'RdfTextDirection::from_str_token(&dir)', 'RdfTextDirection::from_str_token(dir)')
replace('expr.rs', 'typed_term(ctx, &lex, dt)', 'typed_term(ctx, lex, dt)')
replace('expr.rs', '    apply_unary_numeric(ctx, value, op)', '    apply_unary_numeric(ctx, &value, op)', 2)
replace('expr.rs', '''    value: crate::parsed_value::ParsedValue,
    op: purrdf_xsd::numeric::NumericUnaryOperator,''', '''    value: &crate::parsed_value::ParsedValue,
    op: purrdf_xsd::numeric::NumericUnaryOperator,''')
replace('expr.rs', 'if !numeric_step_admitted(ctx, numeric_unary_cost(&value))', 'if !numeric_step_admitted(ctx, numeric_unary_cost(value))')
replace('expr.rs', '''        return governed_xsd_to_term(ctx, &value);
    }
    let capability = ctx.growth.clone();''', '''        return governed_xsd_to_term(ctx, value);
    }
    let capability = ctx.growth.clone();''')
replace('expr.rs', 'numeric_unary_admitted(&value, op,', 'numeric_unary_admitted(value, op,')
replace('expr.rs', '        self.0 = layer.parent.0.clone();', '        self.0.clone_from(&layer.parent.0);')
replace('expr.rs', '''    outer_schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<SubstitutionRow, EvalError>''', '''    outer_schema: &VarSchema,
    ctx: &EvalCtx<'_, D>,
) -> Result<SubstitutionRow, EvalError>''')
replace('expr.rs', '''    let mut _collapsed_owner = None;
    let collapsed;
    let lexical = if target != XsdDatatype::String && is_string_row(datatype_iri) {
        let view = purrdf_xsd::simple::CollapsedWhitespace(lexical);
        let length = crate::workspace::display_len(&view)?;
        _collapsed_owner = Some(
            ctx.growth
                .charge(u64::try_from(length).map_err(|_| EvalError::WorkspaceBoundOverflow)?)?,
        );
        collapsed =
            crate::workspace::format_exact(&view, length, "XSD cast whitespace destination")?;
        collapsed.as_str()
    } else {
        lexical.as_str()
    };
    // One native parser; malformed lexical and physical refusal stay separate.
    match crate::parsed_value::ParsedValue::parse_coded(lexical, target, true, &ctx.growth)? {
        Ok(value) => governed_xsd_to_term(ctx, &value),
        Err(code) => {
            if target.is_numeric() {
                ctx.record_expression_error(code);
            }
            Ok(None)
        }
    }
''', '''    let collapsed = if target != XsdDatatype::String && is_string_row(datatype_iri) {
        let view = purrdf_xsd::simple::CollapsedWhitespace(lexical);
        let length = crate::workspace::display_len(&view)?;
        let allocation = ctx.growth
            .charge(u64::try_from(length).map_err(|_| EvalError::WorkspaceBoundOverflow)?)?;
        let text = crate::workspace::format_exact(
            &view,
            length,
            "XSD cast whitespace destination",
        )?;
        // Tuple fields destroy the text before releasing its original grant.
        Some((text, allocation))
    } else {
        None
    };
    let lexical = collapsed
        .as_ref()
        .map_or(lexical.as_str(), |(text, _)| text.as_str());
    // One native parser; malformed lexical and physical refusal stay separate.
    let result = match crate::parsed_value::ParsedValue::parse_coded(lexical, target, true, &ctx.growth)? {
        Ok(value) => governed_xsd_to_term(ctx, &value),
        Err(code) => {
            if target.is_numeric() {
                ctx.record_expression_error(code);
            }
            Ok(None)
        }
    };
    drop(collapsed);
    result
''')

# Attach the detailed per-row documentation to its actual owner rather than
# the intervening Memory alias, and preserve the helper alias's own meaning.
begin = texts['expr.rs'].index('/// The per-row μ used by pattern substitution')
end = texts['expr.rs'].index("pub(crate) type CorrelatedMemory<'a>", begin)
doc = texts['expr.rs'][begin:end].rstrip() + '\n'
texts['expr.rs'] = texts['expr.rs'][:begin] + '/// The original lexical account used by correlated AST construction.\n' + texts['expr.rs'][end:]
replace('expr.rs', '#[derive(Clone, Default)]\npub(crate) struct SubstitutionRow {', doc + '#[derive(Clone, Default)]\npub(crate) struct SubstitutionRow {')

replace('xpath_regex.rs', 'LexicalFrame::take_failure(self)', 'Self::take_failure(self)')
replace('xpath_regex.rs', 'LexicalFrame::workspace(self)', 'Self::workspace(self)')
replace('xpath_regex.rs', 'LexicalFrame::storage_error(self, error,', 'Self::storage_error(self, error,')
expectation('xpath_regex.rs', '    fn is_match_with_storage(', 'result_large_err', 'the original native error and allocation frame return inline; boxing after physical refusal would require another allocation')
expectation('xpath_regex.rs', "    fn replace_all_with_storage<'h>(", 'result_large_err', 'matcher and output frames retain their original grants on error; boxing a refusal would allocate after admission has already failed')
replace('xpath_regex.rs', 'std::ptr::eq(&*first, &*cloned)', 'std::ptr::eq(&raw const *first, &raw const *cloned)')
replace('xpath_regex.rs', 'std::ptr::eq(&*program, &*cloned)', 'std::ptr::eq(&raw const *program, &raw const *cloned)')

expectation('remote.rs', '    pub fn try_into_resident(self) -> Result<ResolvedBindings, Self> {', 'result_large_err', 'refusal returns the intact original bindings and grants without allocating a new error carrier or detaching bounded storage')
replace('remote.rs', '_control', 'control', 4)
replace('remote.rs', 'clone_service_slice(patterns, memory, |v, m| v.clone_with_memory(m))', 'clone_service_slice(patterns, memory, TriplePattern::clone_with_memory)')
replace('remote.rs', 'clone_service_slice(expression, memory, |v, m| v.clone_with_memory(m))', 'clone_service_slice(expression, memory, purrdf_sparql_algebra::OrderExpression::clone_with_memory)')
replace('remote.rs', '/// Remove every column of a `Values` block whose cell, in ANY row, is a blank node or a\n', '/// The original ordered column and row buffers of a forwarded VALUES block.\ntype ForwardedValues = (Vec<Variable>, Vec<Vec<Option<GroundTerm>>>);\n\n/// Remove every column of a `Values` block whose cell, in ANY row, is a blank node or a\n')
replace('remote.rs', ') -> Result<(Vec<Variable>, Vec<Vec<Option<GroundTerm>>>), EvalError> {', ') -> Result<ForwardedValues, EvalError> {')
replace('remote.rs', '''            if let Some(Some(term)) = row.get(column) {
                if ground_term_has_blank_node_with_memory(term, memory)? {
                    keep_column = false;
                    break;
                }
            }''', '''            if let Some(Some(term)) = row.get(column)
                && ground_term_has_blank_node_with_memory(term, memory)?
            {
                keep_column = false;
                break;
            }''')
replace('remote.rs', '                construct: *construct,', '                construct,', 2)
replace('remote.rs', '    resolved: AdmittedResolvedBindings,', '    resolved: &AdmittedResolvedBindings,')
replace('remote.rs', 'ingest_admitted(response, ctx)', 'ingest_admitted(&response, ctx)', 2)

expectation('remote_http.rs', '    pub fn try_into_resident(self) -> Result<Vec<u8>, Self> {', 'result_large_err', 'bounded refusal returns the original bytes and their original grant inline, with no allocation after the extraction was refused')
replace('remote_http.rs', '_storage', 'storage', 4)
replace('remote_http.rs', '''    fn resolve_native(
        &self,
        request: ServiceRequest<'_>,
        workspace: crate::WorkspaceCapability,
    ) -> Result<crate::remote::AdmittedResolvedBindings, crate::remote::ServiceResolutionError>
''', '''    fn resolve_native(
        &self,
        request: ServiceRequest<'_>,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<crate::remote::AdmittedResolvedBindings, crate::remote::ServiceResolutionError>
''')
replace('remote_http.rs', 'self.resolve_native(request, workspace)', 'self.resolve_native(request, &workspace)')
begin = texts['remote_http.rs'].index('    fn resolve_native(')
end = texts['remote_http.rs'].index('\nimpl<F> HttpTransport', begin)
body = texts['remote_http.rs'][begin:end].replace('&workspace', 'workspace')
texts['remote_http.rs'] = texts['remote_http.rs'][:begin] + body + texts['remote_http.rs'][end:]

replace('service_endpoints.rs', '''                    if let Some(child) = child {
                        if let Err(error) = pending.try_push_admitted(child, &mut memory) {
                            failure = Some(
                                memory
                                    .admission_mut()
                                    .storage_error(error, "SERVICE presence scan"),
                            );
                            return true;
                        }
                    }''', '''                    if let Some(child) = child
                        && let Err(error) = pending.try_push_admitted(child, &mut memory)
                    {
                        failure = Some(
                            memory
                                .admission_mut()
                                .storage_error(error, "SERVICE presence scan"),
                        );
                        return true;
                    }''')
replace('service_endpoints.rs', '''                        if failure.is_none() {
                            if let Err(error) =
                                frames.try_push_admitted(SummaryFrame::Enter(part), &mut memory)
                            {
                                failure = Some(
                                    memory
                                        .admission_mut()
                                        .storage_error(error, "SERVICE summary frames"),
                                );
                            }
                        }''', '''                        if failure.is_none()
                            && let Err(error) =
                                frames.try_push_admitted(SummaryFrame::Enter(part), &mut memory)
                        {
                            failure = Some(
                                memory
                                    .admission_mut()
                                    .storage_error(error, "SERVICE summary frames"),
                            );
                        }''')
replace('service_endpoints.rs', '&"substituting an IRI endpoint', '"substituting an IRI endpoint')
replace('service_endpoints.rs', '&"SERVICE block column', '"SERVICE block column')
replace('service_endpoints.rs', '/// A short description of a non-IRI term, for the error naming it.\n\n', '')

# Keep the resident wrappers' owned-input contract and explicitly destroy their
# consumed probes before returning the independently rewritten tree.
for door in ['apply_probes', 'apply_shacl_probes', 'walk_shacl_probes']:
    callee = door + '_with_memory'
    replace('substitute.rs', f'    resident_rewrite(|memory| {callee}(query, &probes, memory))\n', f'    let rewritten = resident_rewrite(|memory| {callee}(query, &probes, memory));\n    drop(probes);\n    rewritten\n')
replace('substitute.rs', 'debug_assert!(bodies.next().is_none(), "every taken body goes back");', 'debug_assert_eq!(bodies.size_hint(), (0, Some(0)), "every taken body goes back");')
replace('substitute.rs', 'debug_assert!(done.next().is_none());', 'debug_assert_eq!(done.size_hint(), (0, Some(0)));')
replace('substitute.rs', 'return Ok(descend_into_inner_with_memory(frame, open, memory)?);', 'return descend_into_inner_with_memory(frame, open, memory);', 2)
expectation('substitute.rs', '    enum Then {', 'large_enum_variant', 'the next substitution step moves the original admitted tree inline; boxing it would add a separate allocation to the native rewrite')
expectation('substitute.rs', '    fn try_into_resident(self) -> Result<GroundTerm, Self> {', 'result_large_err', 'refused extraction returns the intact original term and its child-box grant; no allocation may be introduced after refusal')
replace('substitute.rs', 'resident_rewrite(|memory| true_literal_with_memory(memory))', 'resident_rewrite(true_literal_with_memory)')

# The old walker docs remained above unrelated declarations after their native
# migration. Preserve each explanation at the function it actually describes.
for doc, marker in [
    ('/// Enter `node`: rewrite it in place when it is a leaf, or take its first operand out\n/// and hold the node open until the operand comes back.\n', "fn enter_push_with_memory<'p>("),
    ("/// Put the rewritten `operand` back into `frame`'s node, and go on: enter the node's\n/// next operand, or finish the node.\n", "fn resume_push_with_memory<'p>("),
    ("/// The expressions `node` itself evaluates, in the order the substitution walk reads\n/// them: its `FILTER`, `BIND` or `UNFOLD` expression, its `OPTIONAL` condition, its\n/// sort keys, or — for a `GROUP BY`, whose aggregates are held apart in `aggregates`\n/// while the walk is inside them — each aggregate's arguments and then its own `FOLD`\n/// sort keys, aggregate by aggregate.\n", "fn own_expressions_with_memory<'n>("),
    ('/// Enter `node`: substitute in it when it is a leaf, or take its first operand out and\n/// hold the node open until the operand comes back.\n', 'fn enter_substitution_with_memory('),
    ("/// Take `frame`'s inner pattern out and enter it in the node's own scope.\n", 'fn descend_into_inner_with_memory('),
    ("/// Put the substituted `operand` back into `frame`'s node, and go on: enter the node's\n/// next operand, or finish the node and report what it carries.\n", 'fn resume_substitution_with_memory('),
]:
    replace('substitute.rs', doc + '\n', '')
    replace('substitute.rs', marker, doc + marker)
replace('substitute.rs', "/// Push the expressions `expr` holds onto `pending`, last first, so the work list pops\n/// them in source order. Wildcard-free: an expression variant added later must say\n/// here what it holds.\n\n", '')
replace('substitute.rs', "    /// A `GROUP BY`'s aggregates, taken apart while their expressions are substituted.\n\n", '')

# Close every remaining incidental blank after an actual doc comment in the
# assigned homes. Blank paragraphs between doc lines become explicit doc lines.
for name in names:
    texts[name] = re.sub(r'(^[ \t]*///[^\n]*\n)\n+(?=[ \t]*(?:///|#\[|(?:pub(?:\([^)]*\))? )?(?:fn|type|struct|enum)\b))', r'\1', texts[name], flags=re.M)

# One stack-only policy record at the original scratch interning home groups
# the independent identity/ownership hooks; no callback law is duplicated.
policy = '''/// Identity and original-payload policies for the one store-once body.
struct InternValueHooks<H, Q, R, B> {
    hash_value: H,
    equal: Q,
    storage_error: R,
    track_value: B,
}

'''
marker = '/// A per-query interner for terms computed during evaluation.\n'
replace('scratch.rs', marker, policy + marker)
native_hooks = '''            |value| workspace.term_hash(value),
            |left, right| workspace.terms_equal(left, right),
            |_| crate::EvalError::AllocationFailed {
                construct: "computed term identity traversal",
            },
            |value, labels, bytes| reserve_value_blanks_admitted(value, labels, bytes, workspace),'''
replace('scratch.rs', native_hooks, '''            InternValueHooks {
                hash_value: |value: &TermValue| workspace.term_hash(value),
                equal: |left: &TermValue, right: &TermValue| workspace.terms_equal(left, right),
                storage_error: |_| crate::EvalError::AllocationFailed {
                    construct: "computed term identity traversal",
                },
                track_value: |value: &TermValue, labels: &mut BlankLabels, bytes: &mut u64| {
                    reserve_value_blanks_admitted(value, labels, bytes, workspace)
                },
            },''', 2)
replace('scratch.rs', '''            |dataset, value| dataset.term_id_by_value(value),
            |_| Ok(()),
            |value| Ok(purrdf_hash::fixed::hash_one(value)),
            |left, right| Ok(left == right),
            |error| panic!("resident scratch proxy traversal allocation: {error}"),
            |value, labels, bytes| {
                reserve_value_blanks(value, labels, bytes);
                Ok(())
            },''', '''            D::term_id_by_value,
            |_| Ok(()),
            InternValueHooks {
                hash_value: |value: &TermValue| Ok(purrdf_hash::fixed::hash_one(value)),
                equal: |left: &TermValue, right: &TermValue| Ok(left == right),
                storage_error: |error| panic!("resident scratch proxy traversal allocation: {error}"),
                track_value: |value: &TermValue, labels: &mut BlankLabels, bytes: &mut u64| {
                    reserve_value_blanks(value, labels, bytes);
                    Ok(())
                },
            },''')
replace('scratch.rs', '''    fn intern_value_with<D: DatasetView, E>(
        &mut self,
        dataset: &D,
        value: TermValue,
        lookup: impl FnOnce(&D, &TermValue) -> Result<Option<D::Id>, E>,
        admit: impl FnOnce(&mut Self) -> Result<(), E>,
        hash_value: impl FnOnce(&TermValue) -> Result<u64, E>,
        mut equal: impl FnMut(&TermValue, &TermValue) -> Result<bool, E>,
        storage_error: impl FnOnce(std::collections::TryReserveError) -> E,
        track_value: impl FnOnce(&TermValue, &mut BlankLabels, &mut u64) -> Result<(), E>,
    ) -> Result<SolutionTerm<D::Id>, E> {
''', '''    fn intern_value_with<D, E, H, Q, R, B>(
        &mut self,
        dataset: &D,
        value: TermValue,
        lookup: impl FnOnce(&D, &TermValue) -> Result<Option<D::Id>, E>,
        admit: impl FnOnce(&mut Self) -> Result<(), E>,
        hooks: InternValueHooks<H, Q, R, B>,
    ) -> Result<SolutionTerm<D::Id>, E>
    where
        D: DatasetView,
        H: FnOnce(&TermValue) -> Result<u64, E>,
        Q: FnMut(&TermValue, &TermValue) -> Result<bool, E>,
        R: FnOnce(std::collections::TryReserveError) -> E,
        B: FnOnce(&TermValue, &mut BlankLabels, &mut u64) -> Result<(), E>,
    {
        let InternValueHooks {
            hash_value,
            mut equal,
            storage_error,
            track_value,
        } = hooks;
''')
replace('scratch.rs', '&"foreign dataset term id"', '"foreign dataset term id"')

identities = []
for name in names:
    path = Path('crates/sparql-eval/src') / name
    output = post / path
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(texts[name])
    identities.append({'path': path.as_posix(), 'source_sha256': hashlib.sha256(original[name].encode()).hexdigest()})
(stage / 'evaluator-clippy-correction-5-source-identity.json').write_text(json.dumps(identities, indent=2) + '\n')
print(f'wrote {len(names)} Stage postimages')
