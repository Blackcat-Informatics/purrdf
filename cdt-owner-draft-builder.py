# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: ignored Stage proposal assembly must not invoke a compiler or mutate the sole shipping source lane.
from pathlib import Path
import difflib
import re

stage = Path('.stage/sparql-eval-complete-bounded-workspace')
exec((stage / 'native-text-owner-assembly.py').read_text().split('selected = [', 1)[0])
paths = [
    'crates/cdt/src/memory.rs', 'crates/cdt/src/lib.rs', 'crates/cdt/src/parse.rs',
    'crates/cdt/src/render.rs', 'crates/cdt/src/limits.rs', 'crates/cdt/src/ops.rs',
    'crates/cdt/src/tree.rs', 'crates/cdt/src/value.rs', 'crates/lex/src/walk.rs',
    'crates/iri/src/parse.rs', 'crates/iri/src/host.rs', 'crates/iri/src/lib.rs',
    'crates/sparql-eval/src/composite_value.rs', 'crates/sparql-eval/src/lib.rs',
    'crates/sparql-eval/src/modifier.rs',
    'crates/lex/src/allocation.rs', 'crates/lex/src/lib.rs',
]
base = {p: Path(p).read_text() if Path(p).exists() else '' for p in paths}
post = dict(base)
post[paths[0]] = (stage / 'cdt-memory-draft.rs').read_text()
post[paths[12]] = (stage / 'cdt-composite-value-draft.rs').read_text().replace('struct CompositePayload {', 'pub(crate) struct CompositePayload {')
post[paths[1]] = post[paths[1]].replace('pub mod limits;', 'pub mod limits;\npub mod memory;')

# Scanner: one grammar, parameterized only by its physical storage owner.
p = post[paths[2]].replace('use alloc::string::{String, ToString};', 'use alloc::string::String;')
p = p.replace('use crate::limits::{MAX_ELEMENTS, MAX_LEXICAL_BYTES, list_extent, map_extent};',
'''use crate::limits::{MAX_ELEMENTS, MAX_LEXICAL_BYTES, try_list_extent, try_map_extent};
use crate::memory::{Memory, ReadError, Resident, Storage};''')
p = p.replace('use crate::render::canonical_key_lexical;', 'use crate::render::try_canonical_key_lexical;')
old_parse = fn_body(p, 'parse_cdt')
inner = old_parse[old_parse.index('{') + 1:-1]
inner = inner.replace('let mut scanner = Scanner::new(lexical);', 'let mut scanner = Scanner::new(lexical, storage);')
inner = inner.replace('    Ok(value)', '    Ok((value, scanner.memory.live_bytes()))')
p = replace_fn(p, 'parse_cdt', '''pub fn parse_cdt(lexical: &str, datatype: CdtDatatype) -> Result<CdtValue, CdtError> {
    match try_parse_cdt(lexical, datatype, &mut Resident) {
        Ok((value, _)) => Ok(value),
        Err(ReadError::Lexical(error)) => Err(error),
        Err(ReadError::Storage(error)) => panic!("resident composite allocation failed: {error}"),
    }
}

/// Run the same scanner with a caller's physical admission and fallible boxes.
/// Returns the parsed value and the exact surviving requested heap bytes. The
/// caller must retain its original storage admission with the returned value;
/// lexical diagnostics likewise die before that admission is released.
///
/// # Errors
/// The existing lexical diagnostic or a distinct native physical failure.
pub fn try_parse_cdt(
    lexical: &str, datatype: CdtDatatype, storage: &mut impl Storage,
) -> Result<(CdtValue, usize), ReadError> {''' + inner + '}')
p = p.replace("struct Scanner<'a> {", "struct Scanner<'a, 'm> {\n    memory: Memory<'m>,")
p = p.replace("impl<'a> Scanner<'a> {", "impl<'a, 'm> Scanner<'a, 'm> {")
p = p.replace("fn new(input: &'a str) -> Self {", "fn new(input: &'a str, storage: &'m mut dyn Storage) -> Self {")
p = p.replace('            input,\n            bytes:', '            memory: Memory::new(storage),\n            input,\n            bytes:', 1)
scanner_start = p.index("impl<'a, 'm> Scanner")
scanner_end = p.index('/// Append a finished element', scanner_start)
scanner = p[scanner_start:scanner_end]
scanner = scanner.replace(', CdtError>', ', ReadError>')
scanner = scanner.replace('        stack.push(Frame::new(datatype));', '        self.memory.push(&mut stack, Frame::new(datatype))?;')
scanner = scanner.replace('                        stack.push(frame);', '                        self.memory.push(&mut stack, frame)?;')
scanner = scanner.replace('push_item(&mut stack, term);', 'push_item(&mut stack, term, &mut self.memory)?;')
scanner = re.sub(r'(?m)^(\s*)return Ok\(value\);', r'\1self.memory.release_vec(stack)?;\n\1return Ok(value);', scanner)
# A root is returned without boxing only to unwrap it immediately afterward.
scanner = replace_fn(scanner, 'close_frame', '''    fn close_frame(&mut self, stack: &mut Vec<Frame>) -> Result<Option<CdtValue>, ReadError> {
        let frame = stack.pop().expect("the frame stack is never empty here");
        let term = match frame {
            Frame::List(items) => {
                let extent = try_list_extent(items.iter(), &mut self.memory)?;
                let value = CdtValue::from_checked_items(items, extent);
                if stack.is_empty() { return Ok(Some(value)); }
                CdtTerm::Composite(self.memory.boxed_value(value)?)
            }
            Frame::Map { entries, .. } => {
                let value = finish_map(entries, &mut self.memory)?;
                if stack.is_empty() { return Ok(Some(value)); }
                CdtTerm::Composite(self.memory.boxed_value(value)?)
            }
            Frame::Triple(mut parts) => {
                let object = parts.pop().expect("a triple frame closes with three parts");
                let predicate = parts.pop().expect("a triple frame closes with three parts");
                let subject = parts.pop().expect("a triple frame closes with three parts");
                let triple = self.memory.boxed_triple(CdtTripleTerm { subject, predicate, object })?;
                self.memory.release_vec(parts)?;
                CdtTerm::TripleTerm(triple)
            }
        };
        push_item(stack, term, &mut self.memory)?;
        Ok(None)
    }''')
scanner = scanner.replace("Some(b'\\\\') => out.push(self.parse_uchar()?),", "Some(b'\\\\') => { let ch = self.parse_uchar()?; self.memory.push_char(&mut out, ch)?; },")
scanner = scanner.replace("Some(b'\\\\') => out.push(self.parse_escape()?),", "Some(b'\\\\') => { let ch = self.parse_escape()?; self.memory.push_char(&mut out, ch)?; },")
scanner = scanner.replace('out.push(ch);', 'self.memory.push_char(&mut out, ch)?;')
scanner = scanner.replace('out.push(quote as char);', 'self.memory.push_char(&mut out, quote as char)?;')
scanner = scanner.replace('match purrdf_iri::parse(&out) {\n            Ok(iri) if iri.has_scheme() => Ok(out),\n            Ok(_) =>', 'match purrdf_iri::absolute_verdict(&out) {\n            Some(true) => Ok(out),\n            Some(false) =>')
scanner = scanner.replace('            Err(_) => Err(CdtError::NotAbsoluteIri {', '            None => Err(CdtError::NotAbsoluteIri {')
scanner = scanner.replace('Ok(label.to_string())', 'Ok(self.memory.string(label)?)')
scanner = scanner.replace('datatype: datatype.to_string(),', 'datatype: self.memory.string(datatype)?,')
scanner = scanner.replace('Ok(CdtLiteral::typed(lexical, datatype))', 'Ok(CdtLiteral { lexical, datatype, language: None, direction: None })')
scanner = scanner.replace('Ok(CdtLiteral::plain(lexical))', 'Ok(CdtLiteral { lexical, datatype: self.memory.string(crate::datatype::XSD_STRING)?, language: None, direction: None })')
scanner = scanner.replace('Ok((language.to_string(), direction))', 'Ok((self.memory.string(language)?, direction))')
scanner = scanner.replace('''        Ok(CdtLiteral::typed(
            &self.input[start..self.position],
            datatype,
        ))''', '''        Ok(CdtLiteral {
            lexical: self.memory.string(&self.input[start..self.position])?,
            datatype: self.memory.string(datatype)?, language: None, direction: None,
        })''')
scanner = scanner.replace('return Ok(CdtLiteral::typed("true", XSD_BOOLEAN));', 'return self.boolean_literal("true");')
scanner = scanner.replace('return Ok(CdtLiteral::typed("false", XSD_BOOLEAN));', 'return self.boolean_literal("false");')
scanner = insert_after_fn(scanner, 'parse_boolean', '''    fn boolean_literal(&mut self, lexical: &str) -> Result<CdtLiteral, ReadError> {
        Ok(CdtLiteral {
            lexical: self.memory.string(lexical)?, datatype: self.memory.string(XSD_BOOLEAN)?,
            language: None, direction: None,
        })
    }''')
scanner = scanner.replace('''                reason: "a base direction must be `ltr` or `rtl`",
            })''', '''                reason: "a base direction must be `ltr` or `rtl`",
            }).map_err(Into::into)''')
p = p[:scanner_start] + scanner + p[scanner_end:]
p = replace_fn(p, 'push_item', '''fn push_item(stack: &mut [Frame], term: CdtTerm, memory: &mut Memory<'_>) -> Result<(), ReadError> {
    match stack.last_mut().expect("the frame stack is never empty here") {
        Frame::List(items) | Frame::Triple(items) => memory.push(items, term)?,
        Frame::Map { entries, pending } => {
            let (offset, key) = pending.take().expect("a map value is only read after its key");
            memory.push(entries, (offset, CdtEntry { key, value: term }))?;
        }
    }
    Ok(())
}''')
p = replace_fn(p, 'finish_map', '''fn finish_map(mut entries: Vec<(usize, CdtEntry)>, memory: &mut Memory<'_>) -> Result<CdtValue, ReadError> {
    // Offsets strictly follow authoring order. This total tie-break reproduces
    // stable key sorting (including the exact first duplicate diagnostic) with
    // the native allocation-free unstable sorter and no hidden merge scratch.
    entries.sort_unstable_by(|(lo, left), (ro, right)|
        crate::ops::total_key_cmp(&left.key, &right.key).then_with(|| lo.cmp(ro)));
    for window in entries.windows(2) {
        let (left_offset, left) = &window[0];
        let (right_offset, right) = &window[1];
        if left.key == right.key {
            let key = try_canonical_key_lexical(&left.key, memory)?;
            return Err(CdtError::DuplicateMapKey {
                offset: *left_offset.max(right_offset), key,
            }.into());
        }
    }
    let old_bytes = core::alloc::Layout::array::<(usize, CdtEntry)>(entries.capacity())
        .map_err(|_| crate::memory::StorageError::SizeOverflow)?.size();
    let mut output = Vec::new();
    memory.reserve(&mut output, entries.len())?;
    // Move entries while the old and destination arrays are both admitted.
    let mut source = entries.into_iter();
    for (_, entry) in source.by_ref() { output.push(entry); }
    drop(source);
    memory.release_bytes(old_bytes)?;
    let extent = try_map_extent(output.iter().map(|entry| (&entry.key, &entry.value)), memory)?;
    Ok(CdtValue::from_checked_entries(output, extent))
}''')
# Convert each direct typed lexical Err at the new ReadError boundary. ? already
# converts all unchanged lexical helper/map_err results via From<CdtError>.
mask = masked(p)
starts = list(re.finditer(r'Err\(CdtError::\w+\s*\{', mask))
for found in reversed(starts):
    brace = mask.index('{', found.start()); depth = 1; end = brace + 1
    while depth:
        if mask[end] == '{': depth += 1
        elif mask[end] == '}': depth -= 1
        end += 1
    if p[end:end+7] != '.into()': p = p[:end] + '.into()' + p[end:]
post[paths[2]] = p
post[paths[0]] = insert_after_fn(post[paths[0]], 'live_bytes', '''    pub(crate) fn release_bytes(&mut self, bytes: usize) -> Result<(), StorageError> {
        self.remove(bytes)
    }''')

# The neutral buffer implementation has one home below CDT and regex alike.
legacy = post[paths[0]]
error_start = legacy.index('/// A physical failure')
error_end = legacy.index('/// The leaf\'s lexical diagnostic')
memory_start = legacy.index('/// One accounting body')
neutral = legacy[memory_start:]
neutral = neutral.replace("pub(crate) struct Memory<'a> {", "pub struct Memory<'a, S: Admission + ?Sized> {")
neutral = neutral.replace("storage: &'a mut dyn Storage,", "storage: &'a mut S,")
neutral = neutral.replace("impl<'a> Memory<'a> {", "impl<'a, S: Admission + ?Sized> Memory<'a, S> {")
neutral = neutral.replace("storage: &'a mut dyn Storage", "storage: &'a mut S")
neutral = neutral.replace('pub(crate)', 'pub')
for name in ['boxed_value', 'boxed_triple']:
    start, end = fn_span(neutral, name)
    neutral = neutral[:start] + neutral[end:]
neutral = insert_after_fn(neutral, 'live_bytes', '''    /// Admission callback, for native factories whose concrete payload lives above this leaf.
    pub fn admission_mut(&mut self) -> &mut S { self.storage }

    /// Admit a native payload's actual Layout before calling its fallible factory.
    pub fn add_bytes(&mut self, bytes: usize) -> Result<(), StorageError> { self.add(bytes) }''')
neutral = neutral.replace('/// One accounting body shared by scanner, measuring renderer and comparison.',
'''/// Exact live-buffer accounting shared by native scanners, compilers and walks.
/// The borrowed callback itself is stack storage, never a retained payload owner.
/// After a native result leaves, its caller retains the original admission grant
/// with that result; use `live_bytes` only after construction scratch has died.''')
neutral = insert_after_fn(neutral, 'release_vec', '''    /// Destroy owned UTF-8 storage before shrinking its original live admission.
    pub fn release_string(&mut self, text: String) -> Result<(), StorageError> {
        let bytes = text.capacity();
        drop(text);
        self.remove(bytes)
    }''')
neutral += '''

impl<S: Admission + ?Sized> fmt::Debug for Memory<'_, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Memory").field("live_bytes", &self.live).finish_non_exhaustive()
    }
}
'''
post[paths[15]] = '''// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fallible physical buffer growth for native lexical parsers and compilers.

use alloc::string::String;
use alloc::vec::Vec;
use core::alloc::Layout;
use core::fmt;

''' + legacy[error_start:error_end] + '''/// A borrowed caller's live-capacity admission for one native computation.
pub trait Admission {
    /// Admit the next exact total before allocation; shrink only after destruction.
    fn resize(&mut self, live_bytes: usize) -> Result<(), StorageError>;
}

''' + neutral
post[paths[15]] = post[paths[15]].replace('A physical failure, separate from the CDT lexical language.',
    'A physical failure, separate from any native lexical language.')
post[paths[15]] = post[paths[15]].replace('composite allocation layout overflow', 'native allocation layout overflow')
post[paths[15]] = post[paths[15]].replace('composite destination allocation failed', 'native destination allocation failed')
post[paths[15]] = post[paths[15]].replace('composite workspace admission refused', 'native workspace admission refused')
post[paths[0]] = '''// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The CDT lexical diagnostic and caller-supplied fallible box factories.

use alloc::boxed::Box;
use core::alloc::Layout;
use core::fmt;
use crate::{CdtError, CdtTripleTerm, CdtValue};

pub use purrdf_lex::allocation::{Admission, StorageError};
pub(crate) type Memory<'a> = purrdf_lex::allocation::Memory<'a, dyn Storage + 'a>;

''' + legacy[error_end:legacy.index('/// A caller\'s concrete storage owner')] + '''/// A caller's concrete storage owner and fallible native box factory.
/// `resize` covers live requested layouts, including both buffers during replacement.
/// Parse success transfers the surviving payload to a caller retaining its original
/// admission. Diagnostics and call locals die before admission is released on error.
/// Factories run only after `Layout::new::<Payload>()` admission and must allocate
/// exactly that payload fallibly, with no unpriced alternate destination.
pub trait Storage: Admission {
    /// Fallibly allocate the already admitted nested composite payload.
    fn boxed_value(&mut self, value: CdtValue) -> Result<Box<CdtValue>, StorageError>;
    /// Fallibly allocate the already admitted nested triple payload.
    fn boxed_triple(&mut self, value: CdtTripleTerm) -> Result<Box<CdtTripleTerm>, StorageError>;
}

pub(crate) struct Resident;

impl Admission for Resident {
    fn resize(&mut self, _live_bytes: usize) -> Result<(), StorageError> { Ok(()) }
}

impl Storage for Resident {
    fn boxed_value(&mut self, value: CdtValue) -> Result<Box<CdtValue>, StorageError> { Ok(Box::new(value)) }
    fn boxed_triple(&mut self, value: CdtTripleTerm) -> Result<Box<CdtTripleTerm>, StorageError> { Ok(Box::new(value)) }
}

pub(crate) trait CdtMemory {
    fn boxed_value(&mut self, value: CdtValue) -> Result<Box<CdtValue>, StorageError>;
    fn boxed_triple(&mut self, value: CdtTripleTerm) -> Result<Box<CdtTripleTerm>, StorageError>;
}

impl CdtMemory for Memory<'_> {
    fn boxed_value(&mut self, value: CdtValue) -> Result<Box<CdtValue>, StorageError> {
        self.add_bytes(Layout::new::<CdtValue>().size())?;
        self.admission_mut().boxed_value(value)
    }
    fn boxed_triple(&mut self, value: CdtTripleTerm) -> Result<Box<CdtTripleTerm>, StorageError> {
        self.add_bytes(Layout::new::<CdtTripleTerm>().size())?;
        self.admission_mut().boxed_triple(value)
    }
}
'''
post[paths[2]] = post[paths[2]].replace('use crate::memory::{Memory, ReadError, Resident, Storage};',
    'use crate::memory::{CdtMemory, Memory, ReadError, Resident, Storage};')
post[paths[12]] = post[paths[12]].replace('use purrdf_cdt::memory::{ReadError, Storage, StorageError};',
    'use purrdf_cdt::memory::{Admission, ReadError, Storage, StorageError};')
owner = post[paths[12]]
f = fn_body(owner, 'resize')
start, end = fn_span(owner, 'resize')
owner = owner[:start] + owner[end:]
at = owner.index("impl Storage for CompositeFrame<'_>")
owner = owner[:at] + "impl Admission for CompositeFrame<'_> {\n" + f + "\n}\n\n" + owner[at:]
post[paths[12]] = owner
post[paths[16]] = post[paths[16]].replace('pub mod walk;', 'pub mod allocation;\npub mod walk;')
(stage / 'lex-allocation-draft.rs').write_text(post[paths[15]])

# Measuring and rendering keep their existing one spelling. The native parser
# drives only the measuring sink and the key writer into pre-sized storage.
r = post[paths[3]]
r = r.replace('use crate::term::', 'use crate::memory::{Memory, Resident, StorageError};\n\nuse crate::term::', 1)
r = r.replace("fn composite<'a>(&mut self, jobs: &mut Vec<Job<'a>>, value: &'a CdtValue);", "fn composite<'a>(&mut self, jobs: &mut Vec<Job<'a>>, value: &'a CdtValue, memory: &mut Memory<'_>) -> Result<(), StorageError>;")
r = r.replace("fn composite<'a>(&mut self, jobs: &mut Vec<Job<'a>>, value: &'a CdtValue) {\n        push_value(jobs, value);\n    }", "fn composite<'a>(&mut self, jobs: &mut Vec<Job<'a>>, value: &'a CdtValue, memory: &mut Memory<'_>) -> Result<(), StorageError> {\n        push_value(jobs, value, memory)\n    }")
r = r.replace("fn composite<'a>(&mut self, _jobs: &mut Vec<Job<'a>>, value: &'a CdtValue) {\n        self.0 = self.0.saturating_add(value.extent().bytes);\n    }", "fn composite<'a>(&mut self, _jobs: &mut Vec<Job<'a>>, value: &'a CdtValue, _memory: &mut Memory<'_>) -> Result<(), StorageError> {\n        self.0 = self.0.saturating_add(value.extent().bytes);\n        Ok(())\n    }")
r = replace_fn(r, 'canonical_lexical', '''pub fn canonical_lexical(value: &CdtValue) -> String {
    let mut out = String::new();
    let mut storage = Resident;
    let mut memory = Memory::new(&mut storage);
    let mut jobs = Vec::new();
    push_value(&mut jobs, value, &mut memory).expect("resident renderer capacity");
    run(&mut out, jobs, &mut memory).expect("resident renderer capacity");
    out
}''')
r = insert_after_fn(r, 'canonical_key_lexical', '''pub(crate) fn try_canonical_key_lexical(key: &CdtKey, memory: &mut Memory<'_>) -> Result<String, StorageError> {
    let length = key_lexical_len(key);
    let mut output = String::new();
    memory.reserve_string(&mut output, length)?;
    write_key(&mut output, key);
    if output.len() != length { return Err(StorageError::SizeOverflow); }
    Ok(output)
}''')
r = replace_fn(r, 'term_lexical_len', '''pub(crate) fn term_lexical_len(term: &CdtTerm) -> usize {
    try_term_lexical_len(term, &mut Memory::new(&mut Resident)).expect("resident measure capacity")
}

pub(crate) fn try_term_lexical_len(term: &CdtTerm, memory: &mut Memory<'_>) -> Result<usize, StorageError> {
    let mut out = Measure(0);
    let mut jobs = Vec::new();
    memory.push(&mut jobs, Job::Term(term))?;
    run(&mut out, jobs, memory)?;
    Ok(out.0)
}''')
r = replace_fn(r, 'run', '''fn run<S: Sink>(out: &mut S, mut jobs: Vec<Job<'_>>, memory: &mut Memory<'_>) -> Result<(), StorageError> {
    while let Some(job) = jobs.pop() {
        match job {
            Job::Punct(text) => out.push_str(text),
            Job::Key(key) => write_key(out, key),
            Job::Term(term) => match term {
                CdtTerm::Composite(inner) => out.composite(&mut jobs, inner.as_ref(), memory)?,
                CdtTerm::TripleTerm(triple) => push_triple(&mut jobs, triple.as_ref(), memory)?,
                CdtTerm::Iri(iri) => write_iri(iri, out),
                CdtTerm::Blank(label) => write_blank(label, out),
                CdtTerm::Literal(literal) => write_literal(out, literal),
                CdtTerm::Null => out.push_str("null"),
            },
        }
    }
    memory.release_vec(jobs)
}''')
for name in ['push_value', 'push_triple']:
    f = fn_body(r, name)
    f = f.replace(") {", ", memory: &mut Memory<'_>) -> Result<(), StorageError> {", 1)
    f = re.sub(r'jobs\.push\(([^;]+)\);', r'memory.push(jobs, \1)?;', f)
    f = f[:-1] + '    Ok(())\n}'
    r = replace_fn(r, name, f)
post[paths[3]] = r

# Extent computation shares the existing measuring writer and element walk.
l = post[paths[4]].replace('use crate::render::{key_lexical_len, term_lexical_len};',
'''use crate::render::{key_lexical_len, try_term_lexical_len};
use crate::memory::{Memory, Resident, StorageError};''')
for name in ['term_elements', 'list_extent', 'map_extent']:
    old = fn_body(l, name)
    if name == 'term_elements':
        body = old[old.index('{') + 1:-1]
        body = body.replace('let mut work: Vec<&CdtTerm> = alloc::vec![term];', 'let mut work = Vec::new();\n    memory.push(&mut work, term)?;')
        body = re.sub(r'work\.push\(([^;]+)\);', r'memory.push(&mut work, \1)?;', body)
        body = body.replace('    elements\n', '    memory.release_vec(work)?;\n    Ok(elements)\n')
        new = '''pub(crate) fn term_elements(term: &CdtTerm) -> usize {
    try_term_elements(term, &mut Memory::new(&mut Resident)).expect("resident extent capacity")
}

fn try_term_elements(term: &CdtTerm, memory: &mut Memory<'_>) -> Result<usize, StorageError> {''' + body + '}'
    else:
        body = old[old.index('{') + 1:-1]
        body = body.replace('term_elements(term)', 'try_term_elements(term, memory)?')
        body = body.replace('term_elements(value)', 'try_term_elements(value, memory)?')
        body = body.replace('term_lexical_len(term)', 'try_term_lexical_len(term, memory)?')
        body = body.replace('term_lexical_len(value)', 'try_term_lexical_len(value, memory)?')
        body = body.replace('    Extent {', '    Ok(Extent {')
        body = body.rstrip()[:-1] + '})\n'
        if name == 'list_extent':
            new = '''pub(crate) fn list_extent<'a>(items: impl IntoIterator<Item = &'a CdtTerm>) -> Extent {
    try_list_extent(items, &mut Memory::new(&mut Resident)).expect("resident extent capacity")
}

pub(crate) fn try_list_extent<'a>(
    items: impl IntoIterator<Item = &'a CdtTerm>, memory: &mut Memory<'_>,
) -> Result<Extent, StorageError> {''' + body + '}'
        else:
            new = '''pub(crate) fn map_extent<'a>(pairs: impl IntoIterator<Item = (&'a CdtKey, &'a CdtTerm)>) -> Extent {
    try_map_extent(pairs, &mut Memory::new(&mut Resident)).expect("resident extent capacity")
}

pub(crate) fn try_map_extent<'a>(
    pairs: impl IntoIterator<Item = (&'a CdtKey, &'a CdtTerm)>, memory: &mut Memory<'_>,
) -> Result<Extent, StorageError> {''' + body + '}'
    l = replace_fn(l, name, new)
post[paths[4]] = l

# Syntactic total order, not the distinct partial value relation. Every CmpJob
# growth is admitted in the original loop; inputs borrow their original owners.
o = post[paths[5]].replace('use crate::error::CdtTypeError;', 'use crate::error::CdtTypeError;\nuse crate::memory::{Memory, Resident, Storage, StorageError};')
o = replace_fn(o, 'total_term_cmp', '''pub fn total_term_cmp(a: &CdtTerm, b: &CdtTerm) -> Ordering {
    try_total_term_cmp(a, b, &mut Resident).expect("resident composite comparison capacity")
}

/// Native syntactic term ordering through the caller's physical storage owner.
/// # Errors
/// Checked layout, admission or allocator refusal, separate from comparability.
pub fn try_total_term_cmp(a: &CdtTerm, b: &CdtTerm, storage: &mut impl Storage) -> Result<Ordering, StorageError> {
    let mut memory = Memory::new(storage);
    let mut jobs = Vec::new();
    memory.push(&mut jobs, CmpJob::Pair(a, b))?;
    run_cmp(jobs, &mut memory)
}''')
o = replace_fn(o, 'total_value_cmp', '''pub fn total_value_cmp(a: &CdtValue, b: &CdtValue) -> Ordering {
    try_total_value_cmp(a, b, &mut Resident).expect("resident composite comparison capacity")
}

/// Native syntactic composite ordering through the same existing relation.
/// # Errors
/// Checked layout, admission or allocator refusal, separate from comparability.
pub fn try_total_value_cmp(a: &CdtValue, b: &CdtValue, storage: &mut impl Storage) -> Result<Ordering, StorageError> {
    let mut memory = Memory::new(storage);
    let mut jobs = Vec::new();
    push_value_cmp(&mut jobs, a, b, &mut memory)?;
    run_cmp(jobs, &mut memory)
}''')
f = fn_body(o, 'run_cmp')
f = f.replace("fn run_cmp(mut jobs: Vec<CmpJob<'_>>) -> Ordering {", "fn run_cmp(mut jobs: Vec<CmpJob<'_>>, memory: &mut Memory<'_>) -> Result<Ordering, StorageError> {")
f = re.sub(r'jobs\.push\(([^;]+)\);', r'memory.push(&mut jobs, \1)?;', f)
f = f.replace('push_value_cmp(&mut jobs, p.as_ref(), q.as_ref());', 'push_value_cmp(&mut jobs, p.as_ref(), q.as_ref(), memory)?;')
f = f.replace('return by_rank;', 'memory.release_vec(jobs)?;\n                    return Ok(by_rank);')
f = f.replace('return decided;', 'memory.release_vec(jobs)?;\n            return Ok(decided);')
f = f.replace('    Ordering::Equal\n}', '    memory.release_vec(jobs)?;\n    Ok(Ordering::Equal)\n}')
o = replace_fn(o, 'run_cmp', f)
o = replace_fn(o, 'push_value_cmp', '''fn push_value_cmp<'a>(
    jobs: &mut Vec<CmpJob<'a>>, a: &'a CdtValue, b: &'a CdtValue, memory: &mut Memory<'_>,
) -> Result<(), StorageError> {
    match (a.contents(), b.contents()) {
        (CdtContents::List(_), CdtContents::Map(_)) => memory.push(jobs, CmpJob::Decided(Ordering::Less))?,
        (CdtContents::Map(_), CdtContents::List(_)) => memory.push(jobs, CmpJob::Decided(Ordering::Greater))?,
        (CdtContents::List(left), CdtContents::List(right)) => {
            memory.push(jobs, CmpJob::Decided(left.len().cmp(&right.len())))?;
            for (p, q) in left.iter().zip(right.iter()).rev() { memory.push(jobs, CmpJob::Pair(p, q))?; }
        }
        (CdtContents::Map(left), CdtContents::Map(right)) => {
            memory.push(jobs, CmpJob::Decided(left.len().cmp(&right.len())))?;
            for (p, q) in left.iter().zip(right.iter()).rev() {
                memory.push(jobs, CmpJob::Pair(&p.value, &q.value))?;
                memory.push(jobs, CmpJob::KeyPair(&p.key, &q.key))?;
            }
        }
    }
    Ok(())
}''')
post[paths[5]] = o

# One allocation-free destruction kernel generalized to both homogeneous
# Nested<T> trees and existing heterogeneous CDT boxes, without public shape
# changes or a second algorithm body.
w = post[paths[8]]
w = replace_fn(w, 'dismantle_tree', '''pub fn dismantle_tree<T: DismantleTree>(node: Box<T>) {
    dismantle_owned(DismantlingBox(node));
}

/// Destructive child/ancestor access using only a node's existing vacant slots.
/// An implementation removes one child, stores an older parent in the vacancy,
/// and recovers that continuation when resumed. No link may survive normal drop.
pub trait DismantleOwned: Sized {
    /// Remove the next owned child, leaving its slot available for a parent.
    fn take_child(&mut self) -> Option<Self>;
    /// Store the older ancestor in the most recently evacuated child slot.
    fn store_parent(&mut self, parent: Option<Self>);
    /// Remove that continuation before the node resumes normal destruction.
    fn take_parent(&mut self) -> Option<Self>;
}

/// The one allocation-free iterative destruction loop for existing owned trees.
pub fn dismantle_owned<T: DismantleOwned>(mut node: T) {
    let mut parent = None;
    loop {
        if let Some(child) = node.take_child() {
            node.store_parent(parent.take());
            parent = Some(node);
            node = child;
            continue;
        }
        drop(node);
        let Some(mut resumed) = parent.take() else { break; };
        parent = resumed.take_parent();
        node = resumed;
    }
}

struct DismantlingBox<T: DismantleTree>(Box<T>);

impl<T: DismantleTree> DismantleOwned for DismantlingBox<T> {
    fn take_child(&mut self) -> Option<Self> {
        self.0.next_child().map(|slot| Self(slot.take().expect("selected a populated dismantling child")))
    }
    fn store_parent(&mut self, parent: Option<Self>) {
        self.0.last_child().expect("removed a child").set_dismantle_parent(parent.map(|node| node.0));
    }
    fn take_parent(&mut self) -> Option<Self> {
        self.0.last_child().expect("the resumed node removed a child").take_dismantle_parent().map(Self)
    }
}''')
post[paths[8]] = w
v = post[paths[7]]
v = insert_after_fn(v, 'take_parts', '''    /// Destruction-only mutable slots. No borrowed public value is mutated;
    /// the shared dismantling loop drains the contents before normal drop.
    pub(crate) fn dismantling_parts(&mut self) -> &mut CdtParts { &mut self.parts }''')
post[paths[7]] = v
t = post[paths[6]]
begin = t.index('/// Move the nodes `parts` owns onto `work`')
end = t.index('// ── Clone', begin)
t = t[:begin] + '''impl purrdf_lex::walk::DismantleOwned for CdtTerm {
    fn take_child(&mut self) -> Option<Self> {
        match self {
            Self::Composite(value) => match value.dismantling_parts() {
                CdtParts::List(items) => loop {
                    let slot = items.last_mut()?;
                    let child = mem::replace(slot, Self::Null);
                    if owns_nodes(&child) { return Some(child); }
                    drop(child);
                    items.pop();
                },
                CdtParts::Map(entries) => loop {
                    let slot = &mut entries.last_mut()?.value;
                    let child = mem::replace(slot, Self::Null);
                    if owns_nodes(&child) { return Some(child); }
                    drop(child);
                    entries.pop();
                },
            },
            Self::TripleTerm(triple) => loop {
                let child = mem::replace(&mut triple.object, Self::Null);
                if owns_nodes(&child) { return Some(child); }
                drop(child);
                triple.object = mem::replace(&mut triple.predicate, Self::Null);
                triple.predicate = mem::replace(&mut triple.subject, Self::Null);
                if matches!((&triple.subject, &triple.predicate, &triple.object),
                    (Self::Null, Self::Null, Self::Null)) { return None; }
            },
            Self::Iri(_) | Self::Blank(_) | Self::Literal(_) | Self::Null => None,
        }
    }

    fn store_parent(&mut self, parent: Option<Self>) {
        let parent = parent.unwrap_or(Self::Null);
        match self {
            Self::Composite(value) => match value.dismantling_parts() {
                CdtParts::List(items) => *items.last_mut().expect("removed a child") = parent,
                CdtParts::Map(entries) => entries.last_mut().expect("removed a child").value = parent,
            },
            Self::TripleTerm(triple) => triple.object = parent,
            _ => unreachable!("only an owning node removes a child"),
        }
    }

    fn take_parent(&mut self) -> Option<Self> {
        let parent = match self {
            Self::Composite(value) => match value.dismantling_parts() {
                CdtParts::List(items) => items.pop().expect("a parent occupies the last slot"),
                CdtParts::Map(entries) => entries.pop().expect("a parent occupies the last slot").value,
            },
            Self::TripleTerm(triple) => {
                let parent = mem::replace(&mut triple.object, Self::Null);
                triple.object = mem::replace(&mut triple.predicate, Self::Null);
                triple.predicate = mem::replace(&mut triple.subject, Self::Null);
                parent
            }
            _ => unreachable!("only an owning node resumes"),
        };
        owns_nodes(&parent).then_some(parent)
    }
}

impl Drop for CdtValue {
    fn drop(&mut self) {
        // Each existing top-level term is the root of the same shared loop. The
        // loop empties every existing box before its ordinary Drop can recurse.
        match self.take_parts() {
            CdtParts::List(items) => {
                for item in items { purrdf_lex::walk::dismantle_owned(item); }
            }
            CdtParts::Map(entries) => {
                for entry in entries { purrdf_lex::walk::dismantle_owned(entry.value); }
            }
        }
    }
}

impl Drop for CdtTripleTerm {
    fn drop(&mut self) {
        for slot in [&mut self.subject, &mut self.predicate, &mut self.object] {
            purrdf_lex::walk::dismantle_owned(mem::replace(slot, CdtTerm::Null));
        }
    }
}

''' + t[end:]
t = t.replace('Each moves the nodes it owns onto a work\n//!   list and dismantles that list in a loop, taking every popped node\'s children\n//!   before the node itself goes,', 'Each threads ancestors through already evacuated term\n//!   slots using the one allocation-free lexical dismantling loop,')
post[paths[6]] = t

# IRI validation: exact same scanner, scalar verdict, no owned diagnostic String.
# This is necessary even for malformed IRIs (host/scheme errors previously
# formatted their own unpriced diagnostic before CDT supplied its lexical error).
i = post[paths[9]]
at = i.index('/// The component spans')
i = i[:at] + '''/// Absolute/relative/invalid verdict through the same scanner, without owning
/// the parsed text or rendering an error which this caller will discard.
#[must_use]
pub fn absolute_verdict(s: &str) -> Option<bool> {
    scan_with(s, Mode::Iri, Diagnostics::Verdict).ok().map(|spans| spans.scheme.is_some())
}

/// Internal diagnostic storage policy; it does not change any grammar decision.
#[derive(Clone, Copy)]
pub(crate) enum Diagnostics { Owned, Verdict }

impl Diagnostics {
    pub(crate) fn message(self, message: core::fmt::Arguments<'_>) -> String {
        match self { Self::Owned => message.to_string(), Self::Verdict => String::new() }
    }
}

''' + i[at:]
f = fn_body(i, 'scan')
f = f.replace('fn scan(s: &str, mode: Mode) -> Result<Spans>', 'fn scan_with(s: &str, mode: Mode, diagnostics: Diagnostics) -> Result<Spans>')
f = f.replace('validate_scheme(&s[..colon])?', 'validate_scheme_with(&s[..colon], diagnostics)?')
f = f.replace('validate_authority(&s[astart..aend], astart, mode)?', 'validate_authority_with(&s[astart..aend], astart, mode, diagnostics)?')
i = replace_fn(i, 'scan', '''fn scan(s: &str, mode: Mode) -> Result<Spans> {
    scan_with(s, mode, Diagnostics::Owned)
}

''' + f)
f = fn_body(i, 'validate_scheme')
f = f.replace('fn validate_scheme(s: &str) -> Result<()>', 'fn validate_scheme_with(s: &str, diagnostics: Diagnostics) -> Result<()>')
f = f.replace('s.to_owned()', 'diagnostics.message(format_args!("{s}"))')
i = replace_fn(i, 'validate_scheme', f)
f = fn_body(i, 'validate_authority')
f = f.replace('fn validate_authority(s: &str, base_off: usize, mode: Mode) -> Result<()>', 'fn validate_authority_with(s: &str, base_off: usize, mode: Mode, diagnostics: Diagnostics) -> Result<()>')
f = f.replace('"trailing characters after IP-literal".to_owned()', 'diagnostics.message(format_args!("trailing characters after IP-literal"))')
f = f.replace('"unterminated IP-literal \'[\'".to_owned()', 'diagnostics.message(format_args!("unterminated IP-literal \'[\'"))')
f = f.replace('crate::host::validate_host(host, host_off, mode)?', 'crate::host::validate_host_with(host, host_off, mode, diagnostics)?')
i = replace_fn(i, 'validate_authority', f)
post[paths[9]] = i
h = post[paths[10]].replace('use crate::parse::validate_component;', 'use crate::parse::{Diagnostics, validate_component};')
f = fn_body(h, 'validate_host')
f = f.replace('pub(crate) fn validate_host(s: &str, base_off: usize, mode: Mode)', 'pub(crate) fn validate_host_with(s: &str, base_off: usize, mode: Mode, diagnostics: Diagnostics)')
f = f.replace('validate_ip_literal(inner, base_off + 1)', 'validate_ip_literal_with(inner, base_off + 1, diagnostics)')
h = replace_fn(h, 'validate_host', f)
f = fn_body(h, 'validate_ip_literal')
f = f.replace('fn validate_ip_literal(inner: &str, base_off: usize)', 'fn validate_ip_literal_with(inner: &str, base_off: usize, diagnostics: Diagnostics)')
f = f.replace('IriError::BadAuthority(format!(', 'IriError::BadAuthority(diagnostics.message(format_args!(')
# Each outer constructor closes the original format! and now additionally closes
# the lazy owned/verdict message call; there are exactly three native branches.
f = f.replace('            ))', '            )))')
f = f.replace('        )))', '        ))))') if False else f
f = f.replace('        )))\n    }', '        ))))\n    }')
h = replace_fn(h, 'validate_ip_literal', f)
post[paths[10]] = h
post[paths[11]] = post[paths[11]].replace('pub use parse::{Iri, is_absolute, parse, parse_uri};', 'pub use parse::{Iri, absolute_verdict, is_absolute, parse, parse_uri};')

# The actual bounded ORDER projection and comparator use the owned native path.
post[paths[13]] = post[paths[13]].replace('mod parsed_value;', 'mod parsed_value;\nmod composite_value;')
m = post[paths[14]].replace('Composite(purrdf_cdt::CdtValue),', 'Composite(crate::composite_value::CompositeValue),')
f = fn_body(m, 'project_shallow_admitted')
start = f.index('            if matches!(datatype.as_str()')
end = f.index('            let parsed =', start)
f = f[:start] + '''            if let Some(composite) = crate::composite_value::CompositeValue::parse(
                lexical_form, datatype, workspace,
            )? { return Ok(SortKey::Composite(composite)); }
''' + f[end:]
m = replace_fn(m, 'project_shallow_admitted', f)
f = fn_body(m, 'project_shallow')
f = f.replace('return SortKey::Composite(composite);', 'return SortKey::Composite(crate::composite_value::CompositeValue::resident(composite));')
m = replace_fn(m, 'project_shallow', f)
m = m.replace('''        // Native CDT parser/order admission remains a required integration owner.
        (SortKey::Composite(x), SortKey::Composite(y)) => purrdf_cdt::total_value_cmp(x, y),''', '''        (SortKey::Composite(x), SortKey::Composite(y)) => x.total_cmp(y, workspace)?,''')
post[paths[14]] = m

# Save the proposal postimage only inside the selected ignored Stage directory.
out = stage / 'native-cdt-owner-postimage'
for p in paths:
    target = out / p
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(post[p])
chunks = []
for p in paths:
    if base[p] == post[p]: continue
    chunks.append('diff --git a/' + p + ' b/' + p + '\n')
    if not base[p]: chunks.append('new file mode 100644\n')
    chunks.extend(difflib.unified_diff(base[p].splitlines(True), post[p].splitlines(True),
        fromfile='a/' + p if base[p] else '/dev/null', tofile='b/' + p, n=3))
(stage / 'native-cdt-owner-draft.patch').write_text(''.join(chunks))
print('Stage-only CDT proposal:', len(paths), 'homes;', sum(1 for p in paths if base[p] != post[p]), 'changed; no compiler or tests invoked')
