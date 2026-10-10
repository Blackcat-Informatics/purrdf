# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: ignored Stage source-proposal assembly must not invoke the compiler or the sole qualification lane.

def masked(text):
    """Mask Rust comments, strings and chars for balanced item boundaries only."""
    out = list(text)
    at = 0
    while at < len(text):
        end = at
        if text.startswith('//', at):
            end = text.find('\n', at)
            if end < 0: end = len(text)
        elif text.startswith('/*', at):
            depth = 1
            end = at + 2
            while depth and end < len(text):
                if text.startswith('/*', end): depth += 1; end += 2
                elif text.startswith('*/', end): depth -= 1; end += 2
                else: end += 1
        else:
            raw = re.match(r'r(#+)?"', text[at:])
            if raw:
                closing = '"' + (raw.group(1) or '')
                end = text.index(closing, at + len(raw.group(0))) + len(closing)
            elif text[at] == '"':
                end = at + 1
                while end < len(text):
                    if text[end] == '\\': end += 2
                    elif text[end] == '"': end += 1; break
                    else: end += 1
            elif text[at] == "'":
                char = re.match(r"'(?:\\(?:u\{[0-9a-fA-F]+\}|x[0-9a-fA-F]{2}|.)|[^'\\\n])'", text[at:])
                if char: end = at + len(char.group(0))
        if end > at:
            for n in range(at, end):
                if out[n] != '\n': out[n] = ' '
            at = end
        else:
            at += 1
    return ''.join(out)

def item_span(text, pattern, ordinal=0):
    mask = masked(text)
    found = list(re.finditer(pattern, mask, re.M))[ordinal]
    start = found.start()
    brace = mask.index('{', found.end())
    depth = 1
    end = brace + 1
    while depth:
        if mask[end] == '{': depth += 1
        elif mask[end] == '}': depth -= 1
        end += 1
    return start, end

def fn_span(text, name, ordinal=0):
    return item_span(text, r'^[ \t]*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?fn\s+' + re.escape(name) + r'\b', ordinal)

def fn_body(text, name, ordinal=0):
    start, end = fn_span(text, name, ordinal)
    return text[start:end]

def replace_fn(text, name, value, ordinal=0):
    start, end = fn_span(text, name, ordinal)
    return text[:start] + value.rstrip() + text[end:]

def insert_after_fn(text, name, value, ordinal=0):
    start, end = fn_span(text, name, ordinal)
    return text[:end] + '\n\n' + value.rstrip() + text[end:]

def imports(text, code):
    at = text.index('\nuse ')
    return text[:at] + '\n' + code.rstrip() + text[at:]

def add_arg_calls(text, method, argument):
    mask = masked(text)
    starts = list(re.finditer(r'\.' + re.escape(method) + r'\(', mask))
    for found in reversed(starts):
        at = found.end(); depth = 1
        while depth:
            if mask[at] == '(': depth += 1
            elif mask[at] == ')': depth -= 1
            at += 1
        close = at - 1
        # Existing multi-line calls may already end in a trailing comma.
        before = text[:close].rstrip()
        separator = ' ' if before.endswith(',') else ', '
        text = before + separator + argument + text[close:]
    return text

selected = [
    'crates/text/src/query_workspace.rs', 'crates/text/src/error.rs',
    'crates/text/src/lib.rs', 'crates/text/src/score.rs',
    'crates/text/src/ranking.rs', 'crates/text/src/fixed.rs',
    'crates/text/src/index.rs', 'crates/text/src/relation.rs',
    'crates/text/src/analysis.rs', 'crates/text/src/segment.rs',
    'crates/text/src/character.rs', 'crates/text/src/stem.rs',
    'crates/lex/src/html.rs', 'crates/lex/src/unicode.rs',
    'crates/lex/src/unicode/aligned.rs', 'crates/sparql-eval/src/workspace.rs',
]
base = {path: Path(path).read_text() if Path(path).exists() else '' for path in selected}
post = dict(base)

# Concrete TEXT owner/error carriers. Resident public error variants are kept.
query = parts['query_workspace'] + '\n\n' + parts['query_error']
query = query.replace('    Ok(TextError::Diagnostic(admitted(', '''    if !workspace.is_bounded() {
        return Ok(match kind {
            purrdf_sparql_eval::NativeDiagnosticKind::Data => TextError::data(message.to_string()),
            purrdf_sparql_eval::NativeDiagnosticKind::Config => TextError::config(message.to_string()),
            _ => TextError::Diagnostic(admitted(purrdf_sparql_eval::NativeDiagnostic::render(kind, message, workspace))?),
        });
    }
    Ok(TextError::Diagnostic(admitted(''')
query += '''

pub(crate) fn data_error(
    message: impl core::fmt::Display, workspace: Option<&WorkspaceCapability>,
) -> TextError {
    match workspace {
        Some(workspace) => query_error(workspace, purrdf_sparql_eval::NativeDiagnosticKind::Data, message)
            .unwrap_or_else(|failure| failure),
        None => TextError::data(message.to_string()),
    }
}

pub(crate) fn arithmetic_error(
    domain: bool, message: impl core::fmt::Display, workspace: Option<&WorkspaceCapability>,
) -> TextError {
    let Some(workspace) = workspace else {
        return if domain { TextError::domain(message.to_string()) } else { TextError::overflow(message.to_string()) };
    };
    let label = if domain { "fixed-point domain error" } else { "fixed-point overflow" };
    query_error(workspace, purrdf_sparql_eval::NativeDiagnosticKind::Function,
        format_args!("{label}: {message}")).unwrap_or_else(|failure| failure)
}
'''
post[selected[0]] = query + '\n'
error = post[selected[1]].replace('pub enum TextError {', '''pub enum TextError {
    /// Native query workspace refusal, without allocating a diagnostic.
    Capacity(crate::query_workspace::CapacityFailure),
    /// Immutable native query diagnostic retaining its allocation admission.
    Diagnostic(purrdf_sparql_eval::NativeDiagnostic),''', 1)
error = error.replace('        match self {', '''        match self {
            Self::Capacity(failure) => core::fmt::Display::fmt(failure, f),
            Self::Diagnostic(diagnostic) => core::fmt::Display::fmt(diagnostic, f),''', 1)
error = error.replace('        match err {', '''        match err {
            TextError::Capacity(failure) => failure.into_eval(),
            TextError::Diagnostic(diagnostic) => Self::NativeDiagnostic(diagnostic),''', 1)
post[selected[1]] = error
post[selected[2]] = post[selected[2]].replace('mod error;', 'mod error;\nmod query_workspace;\npub use query_workspace::CapacityFailure;', 1)

# Ranking is one algorithm for resident/admitted use; all full candidates are
# covered before the existing bounded BinaryHeap accepts them.
score = imports(post[selected[3]], '''use purrdf_sparql_eval::{AdmittedVec, WorkspaceAllocation, WorkspaceCapability, NativeDiagnosticKind};
use crate::query_workspace::{admitted, overflow, query_error};''')
for name, part in [('select_counted', 'score_select'), ('distinct_terms', 'score_distinct'),
                   ('rank_terms', 'score_rank'), ('candidates', 'score_candidates'),
                   ('prepare_terms', 'score_prepare'), ('score_located', 'score_located')]:
    score = replace_fn(score, name, parts[part])
score = replace_fn(score, 'sorted', parts['score_order'])
score = replace_fn(score, 'bounded', '')
score = score.replace('score_document(index, &query, document, held, work)?', 'score_document(index, &query, document, held, work, workspace)?')
score = score.replace('''return Err(TextError::data(format!(
            "the needle holds {} distinct terms, which exceeds the profile bound of 1024",
            terms.len()
        )));''', '''return Err(query_error(workspace, NativeDiagnosticKind::Data, format_args!(
            "the needle holds {} distinct terms, which exceeds the profile bound of 1024", terms.len()))?);''')
score = score.replace('TextError::overflow("a partition holds more ranked rows than a u32 can number")', '''query_error(workspace, NativeDiagnosticKind::Function,
                    format_args!("fixed-point overflow: a partition holds more ranked rows than a u32 can number")).unwrap_or_else(|failure| failure)''')
body = fn_body(score, 'score_document')
body = body.replace('    work: &mut ScoringWork,\n', '    work: &mut ScoringWork,\n    workspace: &WorkspaceCapability,\n', 1)
body = body.replace('index.field_inputs_from_counts(document, counts)?', 'index.field_inputs_from_counts_owned(document, counts, workspace)?')
body = body.replace('score = score.checked_add(query.contribution(ordinal, &fields[..field_count])?)?;',
                    'score = score.checked_add_with(query.contribution(ordinal, &fields[..field_count])?, workspace.is_bounded().then_some(workspace))?;')
body = body.replace('work.documents_scored += 1;', 'work.documents_scored = work.documents_scored.checked_add(1).ok_or_else(overflow)?;')
body = body.replace('index.ranking_profile().validate_score(score)?', 'index.ranking_profile().validate_score_owned(score, workspace)?')
score = replace_fn(score, 'score_document', body)
score = score.replace('    let mut values = Vec::new();\n    let mut allocation = None;', '    let mut allocation = None;\n    let mut values = Vec::new();')
post[selected[3]] = score

ranking = imports(post[selected[4]], '''use purrdf_sparql_eval::{AdmittedVec, WorkspaceCapability, NativeDiagnosticKind};
use crate::query_workspace::{QueryString, admitted, query_error, data_error, arithmetic_error};''')
ranking = ranking.replace('totals: Vec<u128>', 'totals: AdmittedVec<u128>', 1).replace('populations: Vec<u64>', 'populations: AdmittedVec<u64>', 1)
ranking = replace_fn(ranking, 'validate', parts['ranking_validate'])
ranking = replace_fn(ranking, 'prepare_query', parts['ranking_prepare'].replace('PreparedQuery { corpus: self, terms }', 'PreparedQuery { corpus: self, terms, workspace: workspace.clone() }'))
ranking = ranking.replace('terms: Vec<(String, u64, Fixed)>,', 'terms: AdmittedVec<(QueryString, u64, Fixed)>,\n    workspace: WorkspaceCapability,', 1)
ranking = replace_fn(ranking, 'validate_score', parts['validate_score'].replace('self.validate_score_with(score, Some(workspace))', 'self.validate_score_with(score, workspace.is_bounded().then_some(workspace))'))
ranking = ranking.replace('.zip(&self.corpus.totals)', '.zip(self.corpus.totals.iter())')
contribution = fn_body(ranking, 'contribution')
contribution = contribution.replace('        if self.corpus.documents', '        let workspace = self.workspace.is_bounded().then_some(&self.workspace);\n        if self.corpus.documents', 1)
contribution = re.sub(r'TextError::data\(\s*("[^"\n]*"),?\s*\)', r'data_error(\1, workspace)', contribution)
contribution = re.sub(r'TextError::domain\(\s*("[^"\n]*"),?\s*\)', r'arithmetic_error(true, \1, workspace)', contribution)
contribution = contribution.replace('validate_field(*input, total, documents, remaining, df)?', 'validate_field(*input, total, documents, remaining, df, workspace)?')
contribution = contribution.replace('from_count(input.term_frequency)?', 'from_count(input.term_frequency, workspace)?')
for method in ['checked_add', 'checked_sub', 'checked_mul', 'checked_div']:
    contribution = add_arg_calls(contribution, method, 'workspace').replace('.' + method + '(', '.' + method + '_with(')
contribution = contribution.replace('.validate_score(idf.checked_mul_with(saturation, workspace)?)', '.validate_score_with(idf.checked_mul_with(saturation, workspace)?, workspace)')
ranking = replace_fn(ranking, 'contribution', contribution)
for name in ['validate_field', 'inverse_document_frequency', 'from_count']:
    body = fn_body(ranking, name)
    close = masked(body).index(') ->')
    body = body[:close] + ', workspace: Option<&WorkspaceCapability>' + body[close:]
    body = body.replace('df: u64,\n, workspace:', 'df: u64,\n    workspace:')
    body = re.sub(r'TextError::data\(\s*("[^"\n]*"),?\s*\)', r'data_error(\1, workspace)', body)
    body = re.sub(r'TextError::overflow\(\s*("[^"\n]*"),?\s*\)', r'arithmetic_error(false, \1, workspace)', body)
    if name == 'inverse_document_frequency':
        body = body.replace('from_count(documents - frequency)?', 'from_count(documents - frequency, workspace)?').replace('from_count(frequency)?', 'from_count(frequency, workspace)?')
        for method in ['checked_add', 'checked_div']:
            body = add_arg_calls(body, method, 'workspace').replace('.' + method + '(', '.' + method + '_with(')
        body = body.replace('.ln()', '.ln_with(workspace)')
    if name == 'from_count':
        body = body.replace('Fixed::from_integer(', 'Fixed::from_integer_with(')
        # The conversion expression is the only argument; append its context.
        at = body.rfind(')'); body = body[:at] + ', workspace' + body[at:]
    ranking = replace_fn(ranking, name, body)
ranking = ranking.replace('inverse_document_frequency(self.documents, frequency)?', 'inverse_document_frequency(self.documents, frequency, workspace.is_bounded().then_some(workspace))?')
post[selected[4]] = ranking

# The fixed-point success computation stays verbatim at its existing home;
# only error construction is routed through the actual optional owner.
fixed = imports(post[selected[5]], '''use purrdf_sparql_eval::WorkspaceCapability;
use crate::query_workspace::arithmetic_error;''')
for name in ['from_integer', 'checked_add', 'checked_sub', 'checked_mul', 'checked_div', 'ln']:
    old = fn_body(fixed, name)
    common = old.replace('pub fn ' + name + '(', 'pub(crate) fn ' + name + '_with(', 1)
    close = masked(common).index(') ->')
    common = common[:close] + ', workspace: Option<&WorkspaceCapability>' + common[close:]
    common = re.sub(r'TextError::overflow\(\s*("[^"\n]*"),?\s*\)', r'arithmetic_error(false, \1, workspace)', common)
    common = re.sub(r'TextError::domain\(\s*("[^"\n]*"),?\s*\)', r'arithmetic_error(true, \1, workspace)', common)
    common = common.replace('TextError::overflow(format!("{value} does not fit the fixed-point range"))', 'arithmetic_error(false, format_args!("{value} does not fit the fixed-point range"), workspace)')
    common = common.replace('''TextError::domain(format!(
                "ln is undefined at {}, which is not positive",
                self.to_decimal_lexical()
            ))''', '''arithmetic_error(true, format_args!(
                "ln is undefined at {}, which is not positive", self.decimal_display()), workspace)''')
    common = common.replace('signed(magnitude, (self.0 < 0) != (other.0 < 0))', 'signed(magnitude, (self.0 < 0) != (other.0 < 0), workspace)')
    if name == 'from_integer': wrapper = '    pub fn from_integer(value: i64) -> Result<Self, TextError> { Self::from_integer_with(value, None) }'
    elif name == 'ln': wrapper = '    pub fn ln(self) -> Result<Self, TextError> { self.ln_with(None) }'
    else: wrapper = f'    pub fn {name}(self, other: Self) -> Result<Self, TextError> {{ self.{name}_with(other, None) }}'
    fixed = replace_fn(fixed, name, wrapper + '\n\n' + common)
signed = fn_body(fixed, 'signed').replace('negative: bool)', 'negative: bool, workspace: Option<&WorkspaceCapability>)', 1)
signed = signed.replace('TextError::overflow("result left the fixed-point range")', 'arithmetic_error(false, "result left the fixed-point range", workspace)')
fixed = replace_fn(fixed, 'signed', signed)
fixed += '\n\n' + parts['fixed_decimal'] + '\n'
post[selected[5]] = fixed

# Borrow prebuilt index storage and use admitted core term ordering only in
# the per-query subject lookup. Index construction is caller-owned.
index = imports(post[selected[6]], '''use crate::query_workspace::{data_error, query_error};
use purrdf_sparql_eval::NativeDiagnosticKind;''')
index = insert_after_fn(index, 'query_terms', parts['index_terms'])
index = insert_after_fn(index, 'prepared_corpus', parts['index_corpus'].replace('TextError::data("ranking partition is absent")', 'query_error(workspace, NativeDiagnosticKind::Data, "ranking partition is absent").unwrap_or_else(|failure| failure)'))
index = insert_after_fn(index, 'documents_with_subject', parts['subject_lookup'].split('\n    pub(crate) fn partition_at')[0])
old = fn_body(index, 'field_inputs_from_counts')
common = old.replace('fn field_inputs_from_counts(', 'fn field_inputs_from_counts_with(', 1)
common = common.replace('        frequencies: &[(u32, u64)],\n', '        frequencies: &[(u32, u64)],\n        workspace: Option<&purrdf_sparql_eval::WorkspaceCapability>,\n', 1)
common = common.replace('TextError::data("field input names an absent document")', 'data_error("field input names an absent document", workspace)')
wrapper = '''    pub(crate) fn field_inputs_from_counts(
        &self, document: u32, frequencies: &[(u32, u64)],
    ) -> Result<[FieldInput; MAX_FIELDS], TextError> {
        self.field_inputs_from_counts_with(document, frequencies, None)
    }

    pub(crate) fn field_inputs_from_counts_owned(
        &self, document: u32, frequencies: &[(u32, u64)], workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<[FieldInput; MAX_FIELDS], TextError> {
        self.field_inputs_from_counts_with(document, frequencies, workspace.is_bounded().then_some(workspace))
    }
'''
index = replace_fn(index, 'field_inputs_from_counts', wrapper + '\n' + common)
post[selected[6]] = index

# Native relation opening/cursor bodies. No caller index copy, no raw extraction
# of bounded rows; both resident/admitted pull protocols call the same body.
relation = imports(post[selected[7]], '''use std::mem::size_of;
use purrdf_sparql_eval::{AdmittedVec, AdmittedPfRow, WorkspaceTerm, WorkspaceAllocation, WorkspaceCapability, NativeDiagnostic, NativeDiagnosticKind};
use crate::query_workspace::{QueryString, admitted};
use crate::score::{distinct_terms_owned, score_located_owned, select_counted_owned};''')
relation = relation.replace('    Constraint, PartitionFilter, Scored, ScoringWork, distinct_terms, score_located, select_counted,', '    Constraint, Scored, ScoringWork,')
for name in ['needle_text', 'language_constraint', 'rank_bound', 'check_arity']:
    old = fn_body(relation, name)
    common = old.replace('fn ' + name + '(', 'fn ' + name + '_owned(', 1)
    close = masked(common).index(') ->')
    common = common[:close] + ', workspace: &WorkspaceCapability' + common[close:]
    common = common.replace('position: usize,\n, workspace:', 'position: usize,\n    workspace:')
    if name in ['needle_text', 'language_constraint']:
        common = common.replace('fn ' + name + '_owned(', "fn " + name + "_owned<'a>(", 1)
        common = common.replace('value: &TermValue', "value: &'a TermValue", 1)
        common = common.replace('Result<&str, EvalError>', "Result<&'a str, EvalError>")
        common = common.replace('Constraint<String>', "Constraint<&'a str>").replace('lexical_form.clone()', 'lexical_form.as_str()')
    # All primitive/borrowed formatting is priced before format. Recursive
    # TermValue Debug has a separate explicit mandatory admission seam in notes.
    common = common.replace('EvalError::function(format!(', 'NativeDiagnostic::error(NativeDiagnosticKind::Function, format_args!(')
    mask = masked(common)
    starts = list(re.finditer(r'NativeDiagnostic::error\(', mask))
    for found in reversed(starts):
        at = found.end(); depth = 1
        while depth:
            if mask[at] == '(': depth += 1
            elif mask[at] == ')': depth -= 1
            at += 1
        common = common[:at - 1] + ', workspace' + common[at - 1:]
    relation = replace_fn(relation, name, common)
for name in ['agrees', 'bound_values', 'language_term']:
    relation = replace_fn(relation, name, '')
relation = insert_after_fn(relation, 'check_arity_owned', parts['bound_terms'])
holding = relation.index("struct Holding<'i>")
end = relation.index('\n}', holding) + 2
old = relation[holding:end]
new = old.replace('key: PartitionKey', "key: &'i PartitionKey").replace("located: Vec<(usize, &'i [(u32, u64)])>", "located: AdmittedVec<(usize, &'i [(u32, u64)])>")
relation = relation[:holding] + new + relation[end:]
relation = replace_fn(relation, 'holdings', parts['search_owned_helpers'])
relation = replace_fn(relation, 'scored_in_place', '')
relation = replace_fn(relation, 'ranked', '')
opening = '''    fn open(&self, args: &PfArgs<'_>, ceiling: Option<u64>) -> Result<Box<dyn PfCursor>, EvalError> {
        self.open_owned(args, ceiling, WorkspaceCapability::default())
    }

    fn open_admitted(&self, args: &PfArgs<'_>, ceiling: Option<u64>, workspace: WorkspaceCapability) -> Result<Box<dyn PfCursor>, EvalError> {
        self.open_owned(args, ceiling, workspace)
    }'''
relation = replace_fn(relation, 'open', opening, 1)
relation = replace_fn(relation, 'open', opening, 0)
start = relation.index('#[derive(Debug)]\nstruct SearchCursor')
end = relation.index('// ---------------------------------------------------------------------------\n// Positional matching', start)
relation = relation[:start] + parts['search_cursor'] + '\n\n' + relation[end:]
relation = insert_after_fn(relation, 'new', parts['occurrence_open'], 1)
start = relation.index('#[derive(Debug)]\nstruct OccurrenceCursor')
end = relation.index('\n// ---------------------------------------------------------------------------', start)
relation = relation[:start] + parts['occurrence_cursor'] + '\n\n' + relation[end:]
post[selected[7]] = relation

# The actual analyzer uses locally admitted storage and stops its callback on
# first failure. Caller-only aligned projection APIs remain resident.
analysis = imports(post[selected[8]], '''use purrdf_sparql_eval::{AdmittedVec, WorkspaceAllocation, WorkspaceCapability, NativeDiagnosticKind};
use crate::query_workspace::{QueryString, admitted, overflow, query_error};''')
analysis = analysis.replace('TaggedScalar, compose_tagged, decompose_tagged', 'TaggedScalar, compose_tagged', 1)
analysis = analysis.replace('    word: String,\n}', '    word: String,\n    ranges_allocation: Option<WorkspaceAllocation>,\n    word_allocation: Option<WorkspaceAllocation>,\n}', 1)
analysis = insert_after_fn(analysis, 'terms', parts['analyzer_terms'])
analysis = analysis.replace('self.analyze_into(input, &mut buffers, &mut sink)', 'self.analyze_into(input, &mut buffers, &mut |token| { sink(token); Ok(()) })', 1)
analysis = analysis.replace('self.analyze_into(input, scratch, &mut sink)', 'self.analyze_into(input, scratch, &mut |token| { sink(token); Ok(()) })', 1)
analyze = fn_body(analysis, 'analyze_into')
analyze = analyze.replace("sink: &mut impl FnMut(Token<'_>)", "sink: &mut impl FnMut(Token<'_>) -> Result<(), TextError>", 1)
analyze = analyze.replace('        self.fill_word_ranges(', '''        admitted(scratch.normalization.workspace.reserve_vec(&mut scratch.ranges,
            &mut scratch.ranges_allocation, normalized.chars().count()))?;
        if self.dictionary.is_some() {
            scratch.segmentation.reserve_for_query(normalized, &scratch.normalization.workspace)?;
        }
        self.fill_word_ranges(''', 1)
analyze = analyze.replace('return Err(TextError::data("token positions exceed u32"));', 'return Err(query_error(&scratch.normalization.workspace, NativeDiagnosticKind::Data, "token positions exceed u32")?);')
analyze = analyze.replace('                scratch.word.clear();', '''                scratch.word.clear();
                admitted(scratch.normalization.workspace.reserve_string(&mut scratch.word,
                    &mut scratch.word_allocation, word.len()))?;''', 1)
analyze = analyze.replace('crate::stem::english_in_place(&mut scratch.word);', 'crate::stem::english_in_place_owned(&mut scratch.word, &scratch.normalization.workspace)?;')
analyze = analyze.replace('                position: position as u32,\n            });', '                position: position as u32,\n            })?;', 1)
analysis = replace_fn(analysis, 'analyze_into', analyze)
start = analysis.index('#[derive(Debug, Default)]\nstruct NormalizationScratch')
end = analysis.index('\n}', start) + 2
analysis = analysis[:start] + parts['normalization_owners'] + analysis[end:]
analysis = replace_fn(analysis, 'normalize_into', parts['normalize_into'])
analysis = replace_fn(analysis, 'emit_scalars', '')
old = fn_body(analysis, 'normalize_run')
accent_at = old.index('    let scripts = accent.scripts();')
accent_end = old.index('    compose_tagged(', accent_at)
accent = 'fn retain_accents<M>(scalars: &mut Vec<TaggedScalar<M>>, accent: AccentFold) {\n' + old[accent_at:accent_end] + '}\n'
analysis = replace_fn(analysis, 'normalize_run', parts['normalize_run'] + '\n\n' + accent)
analysis = replace_fn(analysis, 'admit_source_size', '')
post[selected[8]] = analysis

segment = imports(post[selected[9]], '''use purrdf_sparql_eval::{WorkspaceAllocation, WorkspaceCapability};
use crate::query_workspace::{admitted, overflow};''')
segment = segment.replace('    paths: Vec<Path>,\n}', '    paths: Vec<Path>,\n    owners: SegmentationOwners,\n}', 1)
segment = segment.replace('impl SegmentationScratch {', '''#[derive(Debug, Default)]
struct SegmentationOwners {
    boundaries: Option<WorkspaceAllocation>,
    clean_offsets: Option<WorkspaceAllocation>,
    clean: Option<WorkspaceAllocation>,
    fallbacks: Option<WorkspaceAllocation>,
    barriers: Option<WorkspaceAllocation>,
    paths: Option<WorkspaceAllocation>,
}

impl SegmentationScratch {
''' + parts['segmentation_capacity'], 1)
post[selected[9]] = segment

character = imports(post[selected[10]], '''use purrdf_sparql_eval::{AdmittedVec, WorkspaceCapability};
use crate::query_workspace::{QueryString, admitted};''')
character = replace_fn(character, 'runs', parts['han_runs'])
# Query selection is shared as well as the run scanner, without raw extraction.
character = replace_fn(character, 'query_terms', '''pub(crate) fn query_terms(analyzer: &Analyzer, input: &str) -> Result<Vec<String>, TextError> {
    Ok(query_terms_owned(analyzer, input, &WorkspaceCapability::default())?
        .into_iter().map(|term| term.as_str().to_owned()).collect())
}''')
post[selected[10]] = character

stem = post[selected[11]]
old = fn_body(stem, 'english_in_place')
common = old.replace('pub fn english_in_place(word: &mut String)', 'fn english_with<E>(word: &mut String, create: impl FnOnce(&str) -> Result<Stem, E>) -> Result<(), E>', 1)
common = common.replace('            return;', '            return Ok(());').replace('        return;', '        return Ok(());')
common = common.replace('Stem::new(input)', 'create(input)?')
common = common[:-1] + '    Ok(())\n}'
wrapper = '''pub fn english_in_place(word: &mut String) {
    let result: Result<(), std::convert::Infallible> = english_with(word, |input| Ok(Stem::new(input)));
    match result { Ok(()) => {}, Err(never) => match never {} }
}'''
stem = replace_fn(stem, 'english_in_place', wrapper + '\n\n' + common + '\n\n' + parts['stem_owned'])
old = fn_body(stem, 'new')
common = old.replace('fn new(input: &str)', 'fn from_letters(input: &str, mut letters: Vec<Letter>)', 1)
common = common.replace('        let mut letters: Vec<Letter> = Vec::with_capacity(input.len());\n', '')
stem = replace_fn(stem, 'new', '''    fn new(input: &str) -> Self { Self::from_letters(input, Vec::with_capacity(input.len())) }

''' + common)
post[selected[11]] = stem

# The HTML reader and Unicode scalar laws remain single implementations; caller
# storage is filled only after exact pre-counts and native source admission.
html = post[selected[12]]
for name in ['reference', 'numeric']:
    body = fn_body(html, name)
    body = body.replace('diagnostics: &mut Vec<Diagnostic>', 'diagnostics: &mut impl FnMut(Diagnostic)')
    body = body.replace('diagnostics.push(', 'diagnostics(')
    html = replace_fn(html, name, body)
html = html.replace('reference(input, start, mode, &mut result.diagnostics)', 'reference(input, start, mode, &mut |diagnostic| result.diagnostics.push(diagnostic))', 1)
html = insert_after_fn(html, 'resolve_strict', parts['html_preallocated'])
post[selected[12]] = html
unicode = post[selected[13]]
unicode = unicode.replace('pub use aligned::{TaggedScalar, compose_tagged, decompose_tagged};', 'pub use aligned::{TaggedScalar, compose_tagged, decompose_tagged, decomposed_len, decompose_tagged_preallocated};', 1)
unicode = insert_after_fn(unicode, 'canonical_order', parts['unicode_order'])
post[selected[13]] = unicode
post[selected[14]] = replace_fn(post[selected[14]], 'decompose_tagged', parts['unicode_decompose'])

workspace = post[selected[15]]
if '    pub fn literal(' not in workspace:
    workspace = workspace.replace('impl WorkspaceCapability {', 'impl WorkspaceCapability {\n' + parts['native_literal'], 1)
workspace = workspace.replace('impl WorkspaceCapability {', '''impl WorkspaceCapability {
    /// Whether allocations admitted by this capability retain an operational account.
    #[must_use]
    pub const fn is_bounded(&self) -> bool { self.owner.is_some() }
''', 1)
post[selected[15]] = workspace

patch = ''.join(''.join(difflib.unified_diff(
    base.get(path, '').splitlines(True), post[path].splitlines(True),
    fromfile='a/' + path, tofile='b/' + path,
)) for path in selected)
(stage / 'native-text-owner-draft.patch').write_text(patch)
print('native-text-owner-draft.patch: standard unified diff; ' + str(len(selected)) + ' source homes; no shipping write')
