from pathlib import Path
import hashlib
import json

root = Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
stage = root / '.stage/sparql-eval-complete-bounded-workspace'
post = stage / 'algebra-clippy-correction-3-postimages'
changes = {}

def read(name):
    path = Path('crates/sparql-algebra/src') / name
    original = (root / path).read_text()
    changes[path.as_posix()] = [original, original]
    return path.as_posix()

def replace(path, old, new, count=1):
    text = changes[path][1]
    actual = text.count(old)
    if actual != count:
        raise RuntimeError(f'{path}: expected {count} occurrences, found {actual}: {old!r}')
    changes[path][1] = text.replace(old, new)

path = read('error.rs')
expectation = '''    #[expect(
        clippy::literal_string_with_formatting_args,
        reason = "these templates are validated and rendered by DiagnosticPresentation"
    )]
'''
replace(path, expectation, '')
replace(path, '    fn own_presentation_with_memory<S:', expectation + '    fn own_presentation_with_memory<S:')
replace(path, '.map_err(presentation_storage)', '.map_err(|error| presentation_storage(&error))', 2)
helper = '''fn presentation_storage(
    error: purrdf_lex::diagnostic::DiagnosticPresentationError,
) -> purrdf_lex::allocation::StorageError {
    match error {
        purrdf_lex::diagnostic::DiagnosticPresentationError::Storage(error) => error,
        _ => panic!("fixed diagnostic templates and typed arguments disagree"),
    }
}
'''
replace(path, '\n' + helper, '\n')
borrowed_helper = helper.replace('error: purrdf_', 'error: &purrdf_', 1).replace('::Storage(error) => error,', '::Storage(error) => *error,')
replace(path, '#[cfg(test)]\nmod tests {', borrowed_helper + '\n#[cfg(test)]\nmod tests {')

path = read('lexer.rs')
replace(path, "impl<'a> Spanned<'a> {", "impl Spanned<'_> {")

path = read('parser/machine.rs')
replace(path, '''                if failure.is_none() {
                    if let Err(error) = memory.push(&mut pending, child) {
                        failure = Some(error);
                    }
                }
''', '''                if failure.is_none()
                    && let Err(error) = memory.push(&mut pending, child)
                {
                    failure = Some(error);
                }
''')
replace(path, 'enum Val {', '''#[expect(
    clippy::large_enum_variant,
    reason = "production values stay inline; native buffers admit the exact Val layout before growth without allocating a separate box per value"
)]
enum Val {''')
replace(path, 'enum Step {', '''#[expect(
    clippy::large_enum_variant,
    reason = "the next machine step is an inline value; boxing Return would add a separate allocation outside the native frame-buffer layout"
)]
enum Step {''')
replace(path, '                for pending in pending_checks.drain(..) {', '''                #[expect(
                    clippy::iter_with_drain,
                    reason = "draining moves each payload while retaining its original admitted Vec buffer until release_vec destroys that buffer before refund"
                )]
                for pending in pending_checks.drain(..) {''')
replace(path, '                                order_has_aggregates |= lifted.contains(name)\n', '                                order_has_aggregates |= lifted.contains(name);\n')

path = read('parser/triples.rs')
replace(path, '                pending.try_push_admitted((inner, !inverse), memory)?\n', '                pending.try_push_admitted((inner, !inverse), memory)?;\n')
replace(path, "enum PathTranslation<'a> {", '''#[expect(
    clippy::large_enum_variant,
    reason = "path endpoints stay inline in the native worklist, whose full PathTranslation layout is admitted before spill growth; boxing would add a second owner"
)]
enum PathTranslation<'a> {''')
replace(path, 'pub(super) enum TFrame {', '''#[expect(
    clippy::large_enum_variant,
    reason = "triples frames stay inline in the native stack, whose exact TFrame array layout is admitted before growth; annotation boxes would add separate allocation owners"
)]
pub(super) enum TFrame {''')

path = read('parser.rs')
replace(path, '''        if let Err(error) = &outcome {
            if !matches!(error, ParseError::Storage(_)) {
                // The closure has destroyed the parser and every partial AST.
                // Only this original raw error payload survives; no new grant is
                // inferred or acquired from its census.
                let retained = error.owned_text_bytes().ok_or(StorageError::SizeOverflow)?;
                let scratch = memory
                    .live_bytes()
                    .checked_sub(original_live)
                    .and_then(|bytes| bytes.checked_sub(retained))
                    .ok_or(StorageError::SizeOverflow)?;
                memory.release_bytes(scratch)?;
            }
        }
''', '''        if let Err(error) = &outcome
            && !matches!(error, ParseError::Storage(_))
        {
            // The closure has destroyed the parser and every partial AST.
            // Only this original raw error payload survives; no new grant is
            // inferred or acquired from its census.
            let retained = error.owned_text_bytes().ok_or(StorageError::SizeOverflow)?;
            let scratch = memory
                .live_bytes()
                .checked_sub(original_live)
                .and_then(|bytes| bytes.checked_sub(retained))
                .ok_or(StorageError::SizeOverflow)?;
            memory.release_bytes(scratch)?;
        }
''')
replace(path, '        tokens.extend(lexemes.drain(..).map(Some));', '''        #[expect(
            clippy::iter_with_drain,
            reason = "token payloads move while the original admitted lexeme Vec buffer remains live until release_vec destroys it before refund"
        )]
        tokens.extend(lexemes.drain(..).map(Some));''')
replace(path, "impl<'a, 'o, 'm, 's, const RDFLIB: bool> Parser<'a, 'o, 'm, 's, RDFLIB> {", "impl<'a, 'o, 's, const RDFLIB: bool> Parser<'a, 'o, '_, 's, RDFLIB> {")
replace(path, '                    self.memory.release_string(text)?\n', '                    self.memory.release_string(text)?;\n')
replace(path, '                    depth = depth.checked_add(1).ok_or(StorageError::SizeOverflow)?\n', '                    depth = depth.checked_add(1).ok_or(StorageError::SizeOverflow)?;\n')
replace(path, '                    memory.extend(&mut patterns, [&mut **left, &mut **right])?\n', '                    memory.extend(&mut patterns, [&mut **left, &mut **right])?;\n')
replace(path, '        assert_eq!(scopes.trail, [a.clone()]);', '        assert_eq!(scopes.trail.as_slice(), core::slice::from_ref(&a));')
replace(path, '            [a.clone()],\n            "the enclosing frame still sees its original identity"', '            core::slice::from_ref(&a),\n            "the enclosing frame still sees its original identity"')

path = read('serialize.rs')
replace(path, '''        if let Err(error) = body(&mut self.memory.borrow_mut()) {
            if self.failure.get().is_none() {
                self.failure.set(Some(error));
            }
        }
''', '''        if let Err(error) = body(&mut self.memory.borrow_mut())
            && self.failure.get().is_none()
        {
            self.failure.set(Some(error));
        }
''')
replace(path, 'ordinary.iter().map(|(name, _)| name.as_str())', 'ordinary.iter().map(|(name, ())| name.as_str())')
replace(path, 'hidden.iter().map(|(variable, _)| variable.clone())', 'hidden.iter().map(|(variable, ())| variable.clone())')
replace(path, 'ordinary.iter().try_fold(0usize, |total, (name, _)| {', 'ordinary.iter().try_fold(0usize, |total, (name, ())| {')
replace(path, 'groups.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));', 'groups.sort_unstable_by_key(|(key, _)| *key);')

path = read('traits.rs')
replace(path, '''                if failure.is_none() {
                    if let Err(error) = stack.try_push_admitted(token, memory) {
                        failure = Some(error);
                    }
                }
''', '''                if failure.is_none()
                    && let Err(error) = stack.try_push_admitted(token, memory)
                {
                    failure = Some(error);
                }
''')

path = read('validate.rs')
replace(path, '                    visit(variable, memory).map_err(MutationError::Visitor)?\n', '                    visit(variable, memory).map_err(MutationError::Visitor)?;\n')

path = read('walk.rs')
replace(path, '''                    if failure.is_none() {
                        if let Err(error) = stack.try_push_admitted(Step::Enter(child), memory) {
                            failure = Some(error);
                        }
                    }
''', '''                    if failure.is_none()
                        && let Err(error) = stack.try_push_admitted(Step::Enter(child), memory)
                    {
                        failure = Some(error);
                    }
''')

identities = []
for path, (original, changed) in changes.items():
    output = post / path
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(changed)
    identities.append({'path': path, 'source_sha256': hashlib.sha256(original.encode()).hexdigest()})
(stage / 'algebra-clippy-correction-3-source-identity.json').write_text(json.dumps(identities, indent=2) + '\n')
print(f'wrote {len(changes)} Stage postimages')
