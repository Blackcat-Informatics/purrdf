# Stage-only proposal assembler; never shipping tooling. No build or test.
from pathlib import Path
import difflib

ROOT = Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
STAGE = ROOT / '.stage/sparql-eval-complete-bounded-workspace'
POST = STAGE / 'native-regex-compatibility-postimages'
POST.mkdir(exist_ok=True)
changes = {}

def read(path):
    text = (ROOT / path).read_text()
    changes[path] = [text, text]
    return text

def put(path, text):
    if path not in changes:
        changes[path] = ['', text]
    else:
        changes[path][1] = text

def replace(text, old, new, count=1):
    assert text.count(old) == count, (old[:100], text.count(old), count)
    return text.replace(old, new)

path = 'crates/lex/examples/gen_unicode_tables.rs'
s = read(path)
s = replace(s, 'fn category_spans() -> BTreeMap<String, Spans> {\n',
    'fn category_spans() -> BTreeMap<String, Spans> {\n    category_spans_from(&read("UnicodeData.txt"))\n}\n\nfn category_spans_from(text: &str) -> BTreeMap<String, Spans> {\n')
s = replace(s, '    for line in read("UnicodeData.txt").lines() {\n        let f: Vec<&str> = line.split(\';\').collect();\n        let point = hex(f[0]);\n        let (name, category)',
    '    for line in text.lines() {\n        let f: Vec<&str> = line.split(\';\').collect();\n        let point = hex(f[0]);\n        let (name, category)')
start = s.index('    let mut categories = category_spans();', s.index('fn xpath()'))
end = s.index('    let blocks:', start)
s = s[:start] + '    let categories = xpath_categories(category_spans());\n' + s[end:]
unit = (STAGE / 'native-regex-compatibility-generator-unit.rs').read_text()
unit = unit[unit.index('const COMPATIBILITY_VERSION'):]
anchor = '// ---------------------------------------------------------------------------\n// `xpath-dated-names`'
s = replace(s, anchor, unit + '\n' + anchor)
s = replace(s, '        "xpath" => xpath(),', '        "xpath" => xpath(),\n        "xpath-compatibility" => xpath_compatibility(),')
s = replace(s, 'ecma-ranges, xpath, xpath-dated-names or xpath-dated-blocks"',
    'ecma-ranges, xpath, xpath-compatibility, xpath-dated-names or xpath-dated-blocks"')
s = replace(s, '//! | `xpath-dated-names`', '//! | `xpath-compatibility` | `crates/rdf-core/src/xsd_regex/xpath/compatibility_tables.rs` | frozen Unicode 16 categories and simple folds for the unselected evaluator law |\n//! | `xpath-dated-names`')
s = replace(s, '//! table of `purrdf-core` is the one deliberate exception: it is pinned to the',
    '//! table and native unselected compatibility categories/folds are deliberate\n//! exceptions: they are pinned to the')
put(path, s)
path = 'scripts/check-generated.sh'
s = read(path)
s = replace(s, '  xpath-dated-names xpath-dated-blocks; do',
    '  xpath-compatibility xpath-dated-names xpath-dated-blocks; do')
s = replace(s, 'sync_file "$tmp/unicode-xpath.rs" crates/rdf-core/src/xsd_regex/xpath/unicode_tables.rs',
    'sync_file "$tmp/unicode-xpath.rs" crates/rdf-core/src/xsd_regex/xpath/unicode_tables.rs\nsync_file "$tmp/unicode-xpath-compatibility.rs" crates/rdf-core/src/xsd_regex/xpath/compatibility_tables.rs')
s = replace(s, '# The one Unicode table NOT at `purrdf_lex::unicode::UNICODE_VERSION`, and why:',
    '# The compatibility Unicode tables differ from the current workspace version.\n# Categories/simple folds and this block table share the locked regex-syntax pin.\n# The native compatibility generator also verifies both UCD16 input identities.')
put(path, s)
path = 'scripts/conformance-frozen/iri-unicode.sha256'
s = read(path)
first, rest = s.split('\n', 1)
s = first + '\nff58e5823bd095166564a006e47d111130813dcf8bf234ef79fa51a870edb48f  16.0.0/UnicodeData.txt\n' + rest
put(path, s)
path = 'crates/iri/unicode/PROVENANCE.md'
s = read(path)
s += '''\nAdded 2026-10-09 for the admitted native unselected regex compatibility law:\n\n- `UnicodeData.txt` — verbatim\n  `https://www.unicode.org/Public/16.0.0/ucd/UnicodeData.txt`, SHA-256\n  `ff58e5823bd095166564a006e47d111130813dcf8bf234ef79fa51a870edb48f`,\n  BLAKE3 `24dd932e1b587f076f3895081f4eb2fd41c77881b3d84a2f743157f6b3f96c40`.\n\nThe existing Rust generator's `xpath-compatibility` mode derives Unicode 16\ncategories and simple-fold classes from these two exact original input\nidentities. They preserve the unselected evaluator law of locked\n`regex-syntax` 0.8.11, while explicitly selected dated XPath laws continue to\nuse their Unicode 17 full-case relation. The shared XML terminal and Unicode\n16 block table homes are reused. No external implementation is copied.\n'''
put(path, s)
put('crates/iri/unicode/16.0.0/UnicodeData.txt',
    (STAGE / 'native-regex-compatibility-inputs/UnicodeData.txt').read_text())

generator_paths = list(changes)

path = 'crates/rdf-core/src/xsd_regex/xpath/mod.rs'
s = read(path)
s = replace(s, 'mod compile;', 'mod compile;\nmod compatibility;\nmod compatibility_tables;')
s = replace(s, 'pub use compile::{CompiledPattern, OwnedCompiledPattern, compile, compile_with_storage};',
    'pub use compile::{CompiledPattern, OwnedCompiledPattern, compile, compile_with_storage};\npub use compatibility::{CompatibilityPattern, OwnedCompatibilityPattern, compile_compatibility_with_storage};')
anchor = '/// A separately admitted compiler, storage or execution resource.'
law = '''/// The internal law carried by the one native program representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Law {
    Dated(Profile),
    Compatibility,
}

impl Law {
    pub(super) const fn is_compatibility(self) -> bool {
        matches!(self, Self::Compatibility)
    }
}

'''
s = replace(s, anchor, law + anchor)
anchor = '    /// These bounds with one named resource replaced.'
s = replace(s, anchor, '''    /// Remove application presets while keeping checked native counter ranges.
    ///
    /// The caller's physical admission still bounds every actual allocation.
    /// Counter/layout overflow remains a typed operational error. This does not
    /// silently install the dated production limits in an unselected evaluator.
    #[must_use]
    pub const fn without_presets() -> Self {
        Self { bounds: [u64::MAX; Resource::COUNT] }
    }

''' + anchor)
anchor = '    /// Invalid grammar under the explicitly selected dated profile.'
s = replace(s, anchor, '''    /// A flag outside the existing unselected compatibility alphabet.
    CompatibilityFlags {
        /// UTF-8 byte offset within the flag text.
        offset: usize,
        /// The rejected character.
        flag: char,
    },
    /// The existing compatibility surface's authored source-byte contract.
    CompatibilitySource {
        /// Actual source UTF-8 bytes.
        bytes: usize,
        /// The compatibility source limit.
        limit: usize,
    },
''' + anchor)
s = replace(s, '            Self::Syntax { offset, message } => {', '''            Self::CompatibilityFlags { offset, flag } => write!(f,
                "invalid compatibility pattern flag {flag:?} at byte {offset}"),
            Self::CompatibilitySource { bytes, limit } => write!(f,
                "compatibility pattern has {bytes} source bytes, limit {limit}"),
            Self::Syntax { offset, message } => {''')
s = replace(s, '            Self::Flags { .. }\n            | Self::Syntax',
    '            Self::Flags { .. }\n            | Self::CompatibilityFlags { .. }\n            | Self::CompatibilitySource { .. }\n            | Self::Syntax')
put(path, s)

path = 'crates/rdf-core/src/xsd_regex/xpath/compile.rs'
s = read(path)
s = replace(s, 'use super::{Budget, Error, Limits, Profile, Resource, dated_blocks, dated_names, unicode_tables};',
    'use super::{Budget, Error, Law, Limits, Profile, Resource, compatibility_tables, dated_blocks, dated_names, unicode_tables};')
s = replace(s, 'fn parse(profile: Profile, flags:', 'fn parse(law: Law, flags:')
s = replace(s, "'q' if profile == Profile::Xpath31 => result.quoted = true,",
    "'q' if law != Law::Dated(Profile::Xpath20) => result.quoted = true,")
s = replace(s, '''                    return Err(Error::Flags {
                        offset,
                        flag,
                        profile,
                    });''', '''                    return Err(match law {
                        Law::Dated(profile) => Error::Flags { offset, flag, profile },
                        Law::Compatibility => Error::CompatibilityFlags { offset, flag },
                    });''')
s = replace(s, "    Table(&'static [(u32, u32)]),", "    Table { ranges: &'static [(u32, u32)], folded: bool },\n    Block { lo: u32, hi: u32, folded: bool },")
anchor = '#[derive(Debug, Clone, Copy)]\nenum LeadStep'
fold = '''pub(super) fn case_variants_for(law: Law, ch: char) -> &'static [u32] {
    if law.is_compatibility() {
        compatibility_tables::CASE_VARIANTS
            .binary_search_by_key(&(ch as u32), |&(point, _)| point)
            .map_or(&[], |index| compatibility_tables::CASE_VARIANTS[index].1)
    } else {
        case_variants(ch)
    }
}

'''
s = replace(s, anchor, fold + anchor)
s = replace(s, '    pub(super) profile: Profile,', '    pub(super) law: Law,')
s = replace(s, '        self.profile == profile && self.source == pattern && self.flags == flags',
    '        self.law == Law::Dated(profile) && self.source == pattern && self.flags == flags')
s = replace(s, '        self.profile\n', '''        match self.law {
            Law::Dated(profile) => profile,
            Law::Compatibility => unreachable!("compatibility payload has no public dated view"),
        }
''')
start = s.index('pub fn compile_with_storage<')
end = s.index('/// Recognize the complete grammar', start)
old = s[start:end]
body = old[old.index('    let verdict = '):]
body = body.replace('compile_budget(profile,', 'compile_budget(law,')
body = body.replace('Ok(super::OwnedPatternValue::new(pattern, storage))', 'Ok(super::OwnedPatternValue::new(wrap(pattern), storage))')
new = '''pub fn compile_with_storage<S: purrdf_lex::allocation::Admission>(
    profile: Profile, pattern: &str, flags: &str, limits: Limits, storage: S,
) -> Result<OwnedCompiledPattern<S>, super::OwnedPatternError<S>> {
    compile_owned_law(Law::Dated(profile), pattern, flags, limits, storage, |pattern| pattern)
}

pub(super) fn compile_owned_law<T, S: purrdf_lex::allocation::Admission>(
    law: Law, pattern: &str, flags: &str, limits: Limits, mut storage: S,
    wrap: impl FnOnce(CompiledPattern) -> T,
) -> Result<super::OwnedPatternValue<T, S>, super::OwnedPatternError<S>> {
''' + body
s = s[:start] + new + s[end:]
s = replace(s, '    compile_budget(profile, pattern, flags, Budget::new(limits))',
    '    compile_budget(Law::Dated(profile), pattern, flags, Budget::new(limits))')
s = replace(s, 'fn compile_budget(\n    profile: Profile,', 'fn compile_budget(\n    law: Law,')
s = replace(s, '    let modes = Modes::parse(profile, flags, &mut budget)?;', '''    if law.is_compatibility() && pattern.len() > crate::xsd_regex::MAX_SOURCE_BYTES {
        return Err(Error::CompatibilitySource {
            bytes: pattern.len(), limit: crate::xsd_regex::MAX_SOURCE_BYTES,
        });
    }
    let modes = Modes::parse(law, flags, &mut budget)?;''')
s = replace(s, '        profile,\n        modes,', '        law,\n        modes,')
s = replace(s, '        profile,\n        source,', '        law,\n        source,')
s = replace(s, '    profile: Profile,\n    modes: Modes,', '    law: Law,\n    modes: Modes,')
s = replace(s, 'let variants = case_variants(ch);', 'let variants = case_variants_for(self.law, ch);')
s = replace(s, '!capturing && self.profile == Profile::Xpath20', '!capturing && self.law == Law::Dated(Profile::Xpath20)')
s = replace(s, '''                Token::Backreference(number) => {
                    let node''', '''                Token::Backreference(number) => {
                    if self.law.is_compatibility() {
                        return Err(self.syntax(offset, "backreferences are not supported by the compatibility law"));
                    }
                    let node''')
# The canonical (x+)? lowering uses the same repeat nodes and preserves the
# legacy nullable-star priority, including captures from the first empty body.
s = replace(s, '        let node = self.node(Node::Repeat {\n            body,\n            min,\n            max,\n            greedy,', '''        let (body, min, max) = if self.law.is_compatibility()
            && min == Count::Finite(0) && max.is_none() {
            let plus = self.node(Node::Repeat {
                body, min: Count::Finite(1), max: None, greedy, follow: None,
            })?;
            (plus, Count::Finite(0), Some(Count::Finite(1)))
        } else { (body, min, max) };
        let node = self.node(Node::Repeat {
            body,
            min,
            max,
            greedy,''')
start = s.index('    fn property(&mut self,')
end = s.index('    fn member(', start)
s = s[:start] + '''    fn property(&mut self, name: &str, offset: usize) -> Result<usize, Error> {
        self.budget.charge_wide(Resource::CompileSteps, (name.len() as u128) * 10)?;
        let folded = self.law.is_compatibility() && self.modes.insensitive;
        if self.law.is_compatibility() && name.starts_with("Is") {
            let Some((lo, hi)) = crate::xsd_regex::blocks::lookup(name) else {
                return Err(self.syntax(offset, "unknown Unicode category or Is-prefixed block"));
            };
            return self.set(Set::Block { lo, hi, folded });
        }
        let found = if name.starts_with("Is") {
            let Law::Dated(profile) = self.law else { unreachable!("compatibility blocks returned above"); };
            block(profile, name)
        } else {
            let categories = if self.law.is_compatibility() {
                compatibility_tables::CATEGORIES
            } else { unicode_tables::CATEGORIES };
            table(categories, name)
        };
        let Some(ranges) = found else {
            return Err(self.syntax(offset, "unknown Unicode category or Is-prefixed block"));
        };
        self.set(Set::Table { ranges, folded })
    }

    fn word(&mut self, offset: usize) -> Result<usize, Error> {
        if !self.law.is_compatibility() { return self.set(Set::Word); }
        // Folding applies to P/Z/C before negation, exactly as in the original
        // translated [^\\p{P}\\p{Z}\\p{C}] class. Difference/negation are never
        // folded after construction.
        let punctuation = self.property("P", offset)?;
        let separators = self.property("Z", offset)?;
        let controls = self.property("C", offset)?;
        let first = self.set(Set::Union(punctuation, separators))?;
        let excluded = self.set(Set::Union(first, controls))?;
        self.set(Set::Complement(excluded))
    }

''' + s[end:]
s = replace(s, "Token::Escape('&' | '~') => {", "Token::Escape('&' | '~') if !self.law.is_compatibility() => {")
s = replace(s, '''                let ranges = if chars {
                    dated_names::NAME
                } else {
                    dated_names::NAME_START
                };
                (self.set(Set::Table(ranges))?, negated)''', '''                let ranges = match (self.law.is_compatibility(), chars) {
                    (true, true) => purrdf_iri::terminals::xml_name_char_ranges(),
                    (true, false) => purrdf_iri::terminals::xml_name_start_char_ranges(),
                    (false, true) => dated_names::NAME,
                    (false, false) => dated_names::NAME_START,
                };
                (self.set(Set::Table { ranges,
                    folded: self.law.is_compatibility() && self.modes.insensitive })?, negated)''')
s = replace(s, '(self.set(Set::Word)?, negated)', '(self.word(offset)?, negated)')
s = replace(s, "    fn scalar(token: &Token<'_>) -> Option<char> {", "    fn scalar(&self, token: &Token<'_>) -> Option<char> {")
s = replace(s, "            Token::Escape('d' | 'D' | '&' | '~') => None,", "            Token::Escape('d' | 'D') => None,\n            Token::Escape('&' | '~') if !self.law.is_compatibility() => None,")
s = s.replace('Self::scalar(', 'self.scalar(')
s = replace(s, '[Set::Table(_)]', '[Set::Table { .. }]')
put(path, s)

path = 'crates/rdf-core/src/xsd_regex/xpath/match.rs'
s = read(path)
s = replace(s, 'Set, case_variants};', 'Set, case_variants, case_variants_for};')
s = replace(s, 'Limits, Profile, Refusal, Resource, unicode_tables};', 'Limits, Law, Profile, Refusal, Resource, unicode_tables};')
s = replace(s, '    #[cfg(all(test, not(target_arch = "wasm32")))]\n    pub(super) fn may_start',
    '    pub(super) fn may_start')
s = replace(s, '    #[cfg(all(test, not(target_arch = "wasm32")))]\n    Pike(Pike', '    Pike(Pike')
s = s.replace('            #[cfg(all(test, not(target_arch = "wasm32")))]\n            Machine::Pike', '            Machine::Pike')
s = replace(s, '    fn with_context(ctx: Ctx<\'a>) -> Self {\n        let fallback', '''    fn with_context(ctx: Ctx<'a>) -> Self {
        if ctx.program.law.is_compatibility() {
            return Self { machine: Machine::Pike(Pike::new(ctx)) };
        }
        let fallback''')
s = replace(s, '                && position < self.input.len()',
    '                && (position < self.input.len() || self.program.law.is_compatibility())')
s = replace(s, '(position == self.input.len() && !self.input.ends_with(\'\\n\'))',
    '(position == self.input.len() && (self.program.law.is_compatibility() || !self.input.ends_with(\'\\n\')))')
s = replace(s, r'''                    if start >= self.input.len() {
                        return Ok(None);
                    }
                    if self.input[..start].ends_with('\n') {
                        return Ok(Some(start));
                    }''', '''                    if start > self.input.len() {
                        return Ok(None);
                    }
                    if self.at_start(start) {
                        return Ok(Some(start));
                    }
                    if start == self.input.len() { return Ok(None); }''')
s = replace(s, '                    let variants = case_variants(ch);',
    '                    let variants = case_variants_for(self.program.law, ch);')
s = replace(s, '            Set::Table(ranges) => purrdf_iri::terminals::in_ranges(ch as u32, ranges),', '''            Set::Table { ranges, folded } => {
                if purrdf_iri::terminals::in_ranges(ch as u32, ranges) { true }
                else if folded {
                    let variants = case_variants_for(self.program.law, ch);
                    self.budget.charge_wide(Resource::MatchSteps, variants.len() as u128)?;
                    variants.iter().any(|&point| purrdf_iri::terminals::in_ranges(point, ranges))
                } else { false }
            }
            Set::Block { lo, hi, folded } => {
                if (lo..=hi).contains(&(ch as u32)) { true }
                else if folded {
                    let variants = case_variants_for(self.program.law, ch);
                    self.budget.charge_wide(Resource::MatchSteps, variants.len() as u128)?;
                    variants.iter().any(|point| (lo..=hi).contains(point))
                } else { false }
            },''')
s = replace(s, '(self.program.profile == Profile::Xpath20 || ch != \'\\r\')',
    '(self.program.law == Law::Dated(Profile::Xpath20) || ch != \'\\r\')')
put(path, s)

path = 'crates/rdf-core/src/xsd_regex/xpath/pike.rs'
s = read(path)
s = replace(s, '    #[cfg(all(test, not(target_arch = "wasm32")))]\n    pub(super) fn find_from',
    '    pub(super) fn find_from')
s = replace(s, '    #[cfg(all(test, not(target_arch = "wasm32")))]\n    fn search(', '    fn search(')
s = replace(s, '    fn visit(&mut self, point: Pc, level: u32) -> Result<bool, Error> {\n', '''    fn visit(&mut self, point: Pc, level: u32) -> Result<bool, Error> {
        // Compatibility merges an epsilon control point at the same input
        // position independent of nullable progress, retaining its first tags.
        let level = if self.ctx.program.law.is_compatibility() { 0 } else { level };
''')
s = replace(s, '''                            let stalled =
                                links[node].stall > link.stall && self.level <= link.stall;''', '''                            let stalled = !program.law.is_compatibility()
                                && links[node].stall > link.stall && self.level <= link.stall;''')
s = replace(s, '        links[body].empty && links[body].counters > links[repeat].counters',
    '        !program.law.is_compatibility()\n            && links[body].empty && links[body].counters > links[repeat].counters')
anchor = '''        if !can_repeat {
            return Ok(can_stop.then_some(Pc::Exit(node)));
        }
        if program.links.nodes[body].counters > link.counters {
            // Leaving instead clears the count again.
            self.set(link.counters as usize, store_count(min, max, count))?;
        }
        if !can_stop {'''
new = '''        if !can_repeat {
            return Ok(can_stop.then_some(Pc::Exit(node)));
        }
        if program.links.nodes[body].counters > link.counters {
            // Leaving instead clears the count again.
            self.set(link.counters as usize, store_count(min, max, count))?;
        }
        if program.law.is_compatibility() && can_stop
            && !self.visit(Pc::Enter(node), 0)? {
            // Merge BEFORE enqueuing the lower-priority exit. An empty loop
            // must not replace that exit's original captures with later tags.
            return Ok(None);
        }
        if !can_stop {'''
s = replace(s, anchor, new)
put(path, s)

path = 'crates/rdf-core/src/xsd_regex/xpath/replace.rs'
s = read(path)
s = replace(s, '''        let mut empty = Vm::with_budget(self, "", budget);
        if empty.find_from(0)?.is_some() {
            return Err(Error::EmptyMatch);
        }
        let budget = empty.into_budget()?;''', '''        let budget = if self.law.is_compatibility() { budget } else {
            let mut empty = Vm::with_budget(self, "", budget);
            if empty.find_from(0)?.is_some() { return Err(Error::EmptyMatch); }
            empty.into_budget()?
        };''')
s = replace(s, '        let mut position = 0;\n        let mut changed',
    '        let mut position = 0;\n        let mut search = Some(0);\n        let mut previous_nonempty_end = None;\n        let mut changed')
s = replace(s, '            let Some(captures) = vm.find_from(position)? else {',
    '            let found = search.map(|start| vm.find_from(start)).transpose()?.flatten();\n            let Some(captures) = found else {')
s = replace(s, '            debug_assert!(matched.end > matched.start, "empty-match guard was passed");', '''            let empty = matched.start == matched.end;
            debug_assert!(!empty || self.law.is_compatibility(), "dated empty-match guard was passed");
            if empty && previous_nonempty_end == Some(matched.end) {
                // The original find iterator suppresses an empty match directly
                // after a nonempty one. The output cursor stays at that end.
                search = input[matched.end..].chars().next()
                    .map(|ch| matched.end + ch.len_utf8());
                vm.release_captures(captures)?;
                continue;
            }''')
s = replace(s, '            position = matched.end;\n            changed = true;', '''            position = matched.end;
            search = if empty {
                input[matched.end..].chars().next().map(|ch| matched.end + ch.len_utf8())
            } else { Some(matched.end) };
            previous_nonempty_end = (!empty).then_some(matched.end);
            changed = true;''')
put(path, s)

path = 'crates/rdf-core/src/xsd_regex/xpath/compatibility.rs'
put(path, (STAGE / 'native-regex-compatibility-native-api.rs').read_text())

def emit(paths, name):
    out = []
    for path in paths:
        before, after = changes[path]
        if before == after:
            continue
        target = POST / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(after)
        out.append('diff --git a/' + path + ' b/' + path + '\n')
        out.extend(difflib.unified_diff(before.splitlines(True), after.splitlines(True),
            fromfile='a/' + path if before else '/dev/null', tofile='b/' + path))
    (STAGE / name).write_text(''.join(out))

emit(generator_paths, 'native-regex-compatibility-generator.patch')
emit([path for path in changes if path not in generator_paths],
    'native-regex-compatibility-law-draft.patch')
