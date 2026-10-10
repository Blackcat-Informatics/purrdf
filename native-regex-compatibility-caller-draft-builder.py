# Stage-only proposal assembler. No shipping mutations, builds or tests.
from pathlib import Path
import difflib

ROOT = Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
STAGE = ROOT / '.stage/sparql-eval-complete-bounded-workspace'
POST = STAGE / 'native-regex-compatibility-postimages'
changes = {}

def load(path):
    text = (ROOT / path).read_text()
    changes[path] = [text, text]
    return text

def put(path, text):
    changes[path][1] = text

def rep(text, old, new, count=1):
    assert text.count(old) == count, (old[:120], text.count(old), count)
    return text.replace(old, new)

path = 'crates/sparql-eval/src/xpath_regex.rs'
s = load(path)
s = rep(s, 'use std::sync::Arc;', '#[cfg(test)]\nuse std::sync::Arc;')
s = rep(s, 'use purrdf_core::xsd_regex::{self, xpath};', 'use purrdf_core::xsd_regex::xpath;')
s = rep(s, '            xpath::Error::Flags { .. } | xpath::Error::Syntax { .. }',
    '            xpath::Error::Flags { .. } | xpath::Error::CompatibilityFlags { .. }\n                | xpath::Error::CompatibilitySource { .. } | xpath::Error::Syntax { .. }')
start = s.index('pub(crate) struct NativeProgram {')
end = s.index('// Trusted one-step native String', start)
s = s[:start] + (STAGE / 'native-regex-compatibility-caller-unit.rs').read_text() + '\n' + s[end:]
start = s.index('impl Program {\n    #[inline(never)]')
end = s.index('/// Linked request verdicts', start)
s = s[:start] + '''impl Program {
    #[inline(never)]
    pub(crate) fn is_match(&self, text: &str, workspace: &WorkspaceCapability) -> Result<Option<bool>, EvalError> {
        match self.compiled.pattern.is_match_with_storage(text, self.limits, LexicalFrame::new(workspace)) {
            Ok(result) => Ok(Some(*result.value())),
            Err(error) => error.publish_with(ErrorPublication::new()),
        }
    }

    #[inline(never)]
    pub(crate) fn replace_literal(
        &self, text: &str, replacement: &str, language: Option<&str>, direction: Option<RdfTextDirection>,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<WorkspaceTerm>, EvalError> {
        match self.compiled.pattern.replace_all_with_storage(text, replacement, self.limits,
            LexicalFrame::new(workspace), LexicalFrame::new(workspace)) {
            Ok(result) => result.publish_with(StringLiteralPublication { language, direction }).map(Some),
            Err(error) => error.publish_with(ErrorPublication::new()),
        }
    }
}

''' + s[end:]
s = rep(s, '    selection: Option<Selection>,\n    verdict:', '    verdict:')
s = rep(s, 'Self { selection: ctx.xpath_regex, verdict: cached(ctx, pattern, flags) }',
    'Self { verdict: cached(ctx, pattern, flags) }')
start = s.index('#[inline(never)]\npub(crate) fn resolve')
end = s.index('#[cfg(all(test, not(target_arch', start)
s = s[:start] + '''fn request(selection: Option<Selection>) -> (Option<xpath::Profile>, xpath::Limits) {
    selection.map_or((None, xpath::Limits::without_presets()),
        |Selection { profile, limits }| (Some(profile), limits))
}

#[inline(never)]
pub(crate) fn resolve<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>, pattern: &str, flags: &str, linked: Option<&LinkedPattern>,
) -> Result<Option<Program>, EvalError> {
    let (profile, limits) = request(ctx.xpath_regex);
    limits.admit_pattern(pattern).map_err(|refusal| EvalError::XPathRegex(refusal.into()))?;
    if let Some(linked) = linked
        && let Ok(Some(program)) = &linked.verdict
        && program.compiled.pattern.matches_source(profile, pattern, flags) {
        program.compiled.pattern.admit(limits).map_err(EvalError::XPathRegex)?;
        return Ok(Some(Program::new(program.compiled.clone(), limits)));
    }
    // Request failures are not artifacts. A changed law/current request or a
    // failed original request is resolved again in the current physical owner.
    cached(ctx, pattern, flags)
}

fn cached<D: DatasetView + Sync>(ctx: &mut EvalCtx<'_, D>, pattern: &str, flags: &str) -> Result<Option<Program>, EvalError> {
    let (profile, limits) = request(ctx.xpath_regex);
    limits.admit_pattern(pattern).map_err(|refusal| EvalError::XPathRegex(refusal.into()))?;
    // Original flags are compared after lookup. The hint cannot select another
    // source or law, and no per-row owned pattern/flag key is allocated.
    let key = purrdf_hash::fixed::FixedState::default().hash_one((profile, pattern));
    if let Some(compiled) = ctx.xpath_regex_cache.get(&key)
        && compiled.pattern.matches_source(profile, pattern, flags) {
        compiled.pattern.admit(limits).map_err(EvalError::XPathRegex)?;
        return Ok(Some(Program::new(compiled, limits)));
    }
    let compiled = match profile {
        Some(profile) => compile_owned(profile, pattern, flags, limits, &ctx.growth)?,
        None => compile_compatibility_owned(pattern, flags, limits, &ctx.growth)?,
    };
    let Some(compiled) = compiled else { return Ok(None); };
    let bytes = compiled.storage.live_bytes().checked_add(size_of::<u64>()).ok_or(EvalError::WorkspaceBoundOverflow)?;
    ctx.xpath_regex_cache.insert_admitted(key, compiled.clone(), bytes, &ctx.growth)?;
    Ok(Some(Program::new(compiled, limits)))
}

''' + s[end:]
s = rep(s, '.hash_one((Profile::Xpath31, "a"))', '.hash_one((Some(Profile::Xpath31), "a"))')
s = rep(s, 'cloned.pattern.matches_source(xpath::Profile::Xpath31,', 'cloned.pattern.matches_source(Some(xpath::Profile::Xpath31),')
s = rep(s, 'Program::Native(compiled, xpath::Limits::new())', 'Program::new(compiled, xpath::Limits::new())')
put(path, s)

path = 'crates/sparql-eval/src/eval.rs'
s = load(path)
start = s.index('    /// Per-query cache for SPARQL `REGEX`/`REPLACE` pattern+flag compilations,')
end = s.index('    /// Explicit native law', start)
s = s[:start] + s[end:]
s = rep(s, '            regex_cache: DetHashMap::default(),\n', '', count=3)
s = rep(s, '(**Fresh**', '(**Fresh**') if '(**Fresh**' in s else s
s = rep(s, '**Fresh** (`regex_cache`,', '**Fresh** (`xpath_regex_cache`,')
put(path, s)

path = 'crates/sparql-eval/src/expr.rs'
s = load(path)
start = s.index('/// The compiled pattern for `(pattern, flags)`, from the per-query cache.')
end = s.index('/// The language-tag grammar', start)
s = s[:start] + '''/// The original resident compatibility compiler, retained only as a test oracle.
/// Production REGEX/REPLACE use the one admitted native program through
/// `xpath_regex::resolve`; this helper never participates in a bounded query.
#[cfg(test)]
pub(crate) fn build_regex(pattern: &str, flags: &str) -> Option<purrdf_core::xsd_regex::CompiledPattern> {
    purrdf_core::xsd_regex::compile(pattern, flags).ok()
}

''' + s[end:]
s = rep(s, 'fn regex_cache_reuses_compiled_pattern_and_failures()', 'fn regex_cache_reuses_success_and_never_caches_request_failure()')
start = s.index('        // Total `(pattern, flags)` entries')
end = s.index('\n\n        assert_eq!(', start)
s = s[:start] + '        let entries = |ctx: &EvalCtx<\'_, Arc<RdfDataset>>| ctx.xpath_regex_cache.stats().entries;' + s[end:]
s = rep(s, '        assert_eq!(entries(&ctx), 2);', '        assert_eq!(entries(&ctx), 1);')
put(path, s)

path = 'crates/sparql-eval/src/vm/tests.rs'
s = load(path)
s = rep(s, 'helpers::cached_regex(ctx, &pattern, &flags)', 'helpers::build_regex(&pattern, &flags)')
put(path, s)

out = []
for path, (before, after) in changes.items():
    target = POST / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(after)
    out.append('diff --git a/' + path + ' b/' + path + '\n')
    out.extend(difflib.unified_diff(before.splitlines(True), after.splitlines(True),
        fromfile='a/' + path, tofile='b/' + path))
(STAGE / 'native-regex-compatibility-caller-draft.patch').write_text(''.join(out))
