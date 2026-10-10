from pathlib import Path
import difflib,re
root=Path(__file__).resolve().parents[2]; stage=Path(__file__).resolve().parent
images={}
def read(p):
    s=(root/p).read_text();images[p]=[s,s];return s
def save(p,s):images[p][1]=s
p='crates/sparql-results/src/error.rs';s=read(p)
s=s.replace('    Internal(String),','    Internal(String),\n    /// Original native producer admission or allocator refusal.\n    Storage(purrdf_lex::allocation::StorageError),')
s=s.replace('            Self::Internal(msg)', '            Self::Storage(error) => write!(f, "{error}"),\n            Self::Internal(msg)')
save(p,s)
p='crates/sparql-results/src/json_read.rs';s=read(p)
s=s.replace('use std::borrow::Cow;', 'use std::borrow::Cow;\nuse purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};')
# Work only on the SELECT producer segments; ASK/provenance keep their original tree API.
start=s.index('pub fn from_json(bytes:'); end=s.index('/// Parse a SPARQL Results JSON `ASK`',start)
front=s[start:end]
a=front.index('pub fn from_json(bytes:'); b=front.index('/// Parse a SPARQL Results JSON `SELECT` document without',a)
front=front[:a]+'''pub fn from_json(bytes: &[u8]) -> Result<ParsedSolutions, Error> {
    let mut memory = Memory::new(&mut Resident);
    let mut reader = Reader::from_slice(bytes, LIMITS).map_err(|error| syntax_with_memory(error, &mut memory))?;
    let result = select_document(&mut reader, &mut memory);
    reader.release_with_memory(&mut memory).map_err(Error::Storage)?;
    result?
}

/// The original result shape refusal or physical producer failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// JSON syntax or SPARQL result shape, preserving its original message.
    Lexical(Error),
    /// Original admission, layout or allocator refusal.
    Storage(StorageError),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { Self::Lexical(error) => error.fmt(f), Self::Storage(error) => error.fmt(f) }
    }
}
impl std::error::Error for ReadError {}

/// Decode the original SELECT producer under caller-owned physical admission.
/// The caller keeps this original grant with the returned payload or refusal.
/// `max_cells` selects the existing bounded-prefix law; `None` selects the
/// full decoder's syntax-before-shape ordering.
/// # Errors
/// Returns the original syntax/shape refusal or typed storage failure.
pub fn from_json_with_memory<S: Admission + ?Sized>(bytes: &[u8], max_cells: Option<u64>, memory: &mut Memory<'_, S>) -> Result<BoundedParsedSolutions, ReadError> {
    let result = (|| {
        if let Some(max_cells) = max_cells {
            return bounded_document(bytes, max_cells, memory);
        }
        let mut reader = Reader::from_slice(bytes, LIMITS).map_err(|error| syntax_with_memory(error, memory))?;
        let result = select_document(&mut reader, memory);
        reader.release_with_memory(memory).map_err(Error::Storage)?;
        result?.map(|solutions| BoundedParsedSolutions { solutions, truncated: false })
    })();
    result.map_err(|error| match error { Error::Storage(error) => ReadError::Storage(error), error => ReadError::Lexical(error) })
}

'''+front[b:]
sig='pub fn from_json_bounded(bytes: &[u8], max_cells: u64) -> Result<BoundedParsedSolutions, Error> {'
front=front.replace(sig,sig+'''\n    bounded_document(bytes, max_cells, &mut Memory::new(&mut Resident))
}

fn bounded_document<S: Admission + ?Sized>(bytes: &[u8], max_cells: u64, memory: &mut Memory<'_, S>) -> Result<BoundedParsedSolutions, Error> {''',1)
front=front.replace('scan_variables(bytes)?','scan_variables(bytes, memory)?').replace('scan_bindings(bytes, &variables, row_limit)?','scan_bindings(bytes, &variables, row_limit, memory)?')
front=front.replace('fn scan_variables(bytes: &[u8])','fn scan_variables<S: Admission + ?Sized>(bytes: &[u8], memory: &mut Memory<\'_, S>)')
front=front.replace('fn scan_bindings(', 'fn scan_bindings<S: Admission + ?Sized>(').replace('    row_limit: Option<usize>,\n) -> BoundedRowsResult', "    row_limit: Option<usize>,\n    memory: &mut Memory<'_, S>,\n) -> BoundedRowsResult")
# Every scan owns and destroys its original reader stack.
for name in ['scan_variables','scan_bindings']:
    a=front.index('fn '+name); b=front.find('\n}\n',a)+3
    body=front[a:b]
    body=body.replace('    open_object(&mut reader,','    let result = (|| {\n    open_object(&mut reader,',1)
    body=body[:-2]+'    })();\n    reader.release_with_memory(memory).map_err(Error::Storage)?;\n    result\n}\n'
    front=front[:a]+body+front[b:]
s=s[:start]+front+s[end:]
start=s.index('fn select_document(');end=s.index('\n#[cfg(test)]',start)
back=s[start:end]
# All SELECT helpers now have an original Memory door; public resident entries share them.
names=['select_document','read_head','string_items','read_results','binding_rows','binding_row','string_member','binding','open_object','open_array','next_key','head_variables','string_array','bounded_results','bounded_binding_array']
def add_signature(text,name):
    pat=r'fn '+name+r"(?:<'a>)?\("
    m=re.search(pat,text);assert m,name
    a=m.start(); pos=m.end();depth=1;i=pos
    while depth:
        depth+=(text[i]=='(')-(text[i]==')');i+=1
    params=text[pos:i-1].rstrip().rstrip(',')
    gen="<'a, S: Admission + ?Sized>" if "<'a>" in m.group() else '<S: Admission + ?Sized>'
    return text[:a]+'fn '+name+gen+'('+params+", memory: &mut Memory<'_, S>"+text[i-1:]
for name in names:back=add_signature(back,name)
# Native syntax/shape prose is authored fallibly through this memory, never to_string/format first.
a=back.index('/// Build a `Format` error.');b=back.index('/// The reader\'s bounds:',a)
back=back[:a]+'''/// Build original format prose through admitted native storage.
fn fmt_with_memory<S: Admission + ?Sized>(msg: &(impl std::fmt::Display + ?Sized), memory: &mut Memory<'_, S>) -> Error {
    match memory.format(&format_args!("SPARQL-JSON: {msg}")) { Ok(text) => Error::Format(text), Err(error) => Error::Storage(error) }
}

fn fmt(msg: &str) -> Error { fmt_with_memory(msg, &mut Memory::new(&mut Resident)) }

fn syntax_with_memory<S: Admission + ?Sized>(error: json::Error, memory: &mut Memory<'_, S>) -> Error {
    match error.kind() {
        json::ErrorKind::Storage(storage) | json::ErrorKind::Escape(purrdf_lex::json_escape::JsonEscapeErrorKind::Storage(storage)) => Error::Storage(storage),
        _ => fmt_with_memory(&error, memory),
    }
}

fn syntax(error: json::Error) -> Error { syntax_with_memory(error, &mut Memory::new(&mut Resident)) }

'''+back[b:]
# split off resident helper definitions so transformed native callers never re-enter resident doors
parts=[front,back]
def calls(text):
    # Balanced expression parentheses (quoted strings can contain parentheses; use a lexical scan).
    for name in names+['scan_variables','scan_bindings','resolve_datatype']:
        pattern=re.compile(r'(?<![\w.])'+name+r'\(')
        found=list(pattern.finditer(text))
        for m in reversed(found):
            # Definitions have generic params so do not match. Exceptions are none.
            i=m.end();depth=1;quote=None;escape=False
            while depth:
                c=text[i]
                if quote:
                    if escape:escape=False
                    elif c=='\\':escape=True
                    elif c==quote:quote=None
                elif c=='"':quote='"'
                elif c=='(':depth+=1
                elif c==')':depth-=1
                i+=1
            args=text[m.end():i-1].rstrip().rstrip(',')
            # Front calls already manually carry memory.
            if args.endswith('memory'):continue
            text=text[:m.end()]+args+', memory'+text[i-1:]
    return text
front=calls(front);back=calls(back)
def native_rewrite(text):
    text=text.replace('.map_err(syntax)', '.map_err(|error| syntax_with_memory(error, memory))')
    for method in ['next_event','next_key','skip_value','check_value','finish']:
        text=text.replace('.'+method+'()', '.'+method+'_with_memory(memory)')
    text=text.replace('.decode()', '.decode_with_memory(memory)')
    # original static/dynamic authored fmt calls; standalone wrappers are protected separately below
    text=re.sub(r'(?<![\w.])fmt\(', 'fmt_with_memory(',text)
    pattern=re.compile(r'fmt_with_memory\(')
    for m in reversed(list(pattern.finditer(text))):
        if text[max(0,m.start()-3):m.start()]=='fn ':continue
        i=m.end();depth=1;quoted=False;esc=False
        while depth:
            c=text[i]
            if quoted:
                if esc:esc=False
                elif c=='\\':esc=True
                elif c=='"':quoted=False
            elif c=='"':quoted=True
            elif c=='(':depth+=1
            elif c==')':depth-=1
            i+=1
        args=text[m.end():i-1].rstrip().rstrip(',')
        if not args.endswith('memory') and 'Memory::new' not in args:
            text=text[:m.end()]+args+', memory'+text[i-1:]
    return text
# Protect fmt/syntax helpers from generic call rewriting.
ma=back.index('/// Build original format prose');mb=back.index('/// The reader\'s bounds',ma)
helpers=back[ma:mb];back=back[:ma]+'@@HELPERS@@\n'+back[mb:]
front=native_rewrite(front);back=native_rewrite(back).replace('@@HELPERS@@\n',helpers)
# Generated public wrapper has a local &mut Memory, avoid borrowing it as value during conversion.
front=front.replace('syntax_with_memory(error, memory))?;\n    let result = select_document(&mut reader, &mut memory)', 'syntax_with_memory(error, &mut memory))?;\n    let result = select_document(&mut reader, &mut memory)')
# Calls already carrying explicit Memory were not touched by rewrite.
back=back.replace('strings.push(item.decode_with_memory(memory).map_err(|error| syntax_with_memory(error, memory))?.into_owned());', 'let text = owned_text(item.decode_with_memory(memory).map_err(|error| syntax_with_memory(error, memory))?, memory)?;\n                memory.push(strings, text).map_err(Error::Storage)?;')
back=back.replace('Ok(row) => rows.push(row),','Ok(row) => memory.push(&mut rows, row).map_err(Error::Storage)?,')
back=back.replace('    let mut row = vec![None; variables.len()];', '    let mut row = Vec::new();\n    memory.reserve(&mut row, variables.len()).map_err(Error::Storage)?;\n    row.resize_with(variables.len(), || None);')
back=back.replace('    scratch.bound.resize(variables.len(), false);','    memory.reserve(&mut scratch.bound, variables.len()).map_err(Error::Storage)?;\n    scratch.bound.resize(variables.len(), false);')
# Head column names must be looked up before releasing escaped arbitrary keys.
a=back.index('    while let Some(key) = next_key(',back.index('fn binding_row'));b=back.index('        if scratch.bound[index]',a)
back=back[:a]+'''    while let Some(raw) = reader.next_key_with_memory(memory).map_err(|error| syntax_with_memory(error, memory))? {
        let key = raw.decode_with_memory(memory).map_err(|error| syntax_with_memory(error, memory))?;
        let index = variables.iter().position(|variable| variable == key.as_ref());
        release_text(key, memory)?;
        let Some(index) = index else {
            reader.check_value_with_memory(memory).map_err(|error| syntax_with_memory(error, memory))?;
            continue;
        };
'''+back[b:]
a=back.index('        let cell = binding(',back.index('fn binding_row'));b=back.index('        match cell {',a)
back=back[:a]+'''        let start = reader.offset();
        let cell = binding(reader, &mut scratch.open, memory)?;
        let end = reader.offset();
        // Move the original cell to its first column. Repeated head names use
        // the same binding producer over the source span, admitting each actual
        // copy before birth rather than cloning an unpriced term tree.
        for column in index..variables.len() {
            if variables[column] == variables[index] {
                scratch.bound[column] = true;
                if column != index && cell.is_ok() {
                    let mut duplicate = Reader::new(&reader.text()[start..end], LIMITS);
                    let decoded = binding(&mut duplicate, &mut scratch.open, memory);
                    duplicate.release_with_memory(memory).map_err(Error::Storage)?;
                    row[column] = Some(decoded??);
                }
            }
        }
'''+back[b:]
back=back.replace('            _ => {}\n        }\n    }\n    Ok(match refusal', '            Ok(term) => row[index] = Some(term),\n            Err(error) => release_error(error, memory)?,\n        }\n    }\n    Ok(match refusal',1)
# Original term constructor doors and binding boxes.
back=back.replace('open.push(Open::Binding(Box::default()));','memory.reserve(open, open.len().checked_add(1).ok_or(Error::Storage(StorageError::SizeOverflow))?).map_err(Error::Storage)?;\n    let fields = memory.boxed(BindingFields::default()).map_err(Error::Storage)?;\n    open.push(Open::Binding(fields));')
back=back.replace('open.push(Open::Triple(Box::default()));','memory.reserve(open, open.len().checked_add(1).ok_or(Error::Storage(StorageError::SizeOverflow))?).map_err(Error::Storage)?;\n                                let parts = memory.boxed(TripleParts::default()).map_err(Error::Storage)?;\n                                open.push(Open::Triple(parts));')
back=back.replace('Open::Binding(fields) => {\n                    let term = fields.finish();','Open::Binding(mut fields) => {\n                    let term = fields.finish(memory);\n                    drop(fields);\n                    memory.release_bytes(size_of::<BindingFields<\'_>>()).map_err(Error::Storage)?;')
# Replace original finishing methods rather than introducing a separate RDF decoder.
a=back.index('    fn finish(self) -> Shaped<TermValue> {',back.index('impl TripleParts'));b=back.index('\n}\n\nimpl BindingFields',a)
back=back[:a]+'''    fn finish<S: Admission + ?Sized>(&mut self, memory: &mut Memory<'_, S>) -> Shaped<TermValue> {
        let s = self.subject.take().ok_or_else(|| fmt_with_memory("triple has no subject", memory))??;
        let p = self.predicate.take().ok_or_else(|| fmt_with_memory("triple has no predicate", memory))??;
        let o = self.object.take().ok_or_else(|| fmt_with_memory("triple has no object", memory))??;
        if !matches!(p, TermValue::Iri(_)) { return Err(fmt_with_memory("triple-term predicate is not an IRI", memory)); }
        Ok(TermValue::Triple {
            s: TermBox::try_new(s, memory).map_err(Error::Storage)?,
            p: TermBox::try_new(p, memory).map_err(Error::Storage)?,
            o: TermBox::try_new(o, memory).map_err(Error::Storage)?,
        })
    }
'''+back[b:]
a=back.index('    fn finish(self) -> Shaped<TermValue> {',back.index('impl BindingFields'));b=back.index('\n}\n\n/// A member\'s value',a)
back=back[:a]+'''    fn finish<S: Admission + ?Sized>(&mut self, memory: &mut Memory<'_, S>) -> Shaped<TermValue> {
        let Some(kind) = Member::text(self.kind.take()) else { return Err(fmt_with_memory("binding has no string `type`", memory)); };
        let result = (|| {
            match kind.as_ref() {
                "uri" => Ok(TermValue::Iri(binding_text(self.value.take(), memory)?)),
                "bnode" => Ok(TermValue::Blank { label: binding_text(self.value.take(), memory)?, scope: BlankScope::DEFAULT }),
                "literal" | "typed-literal" => {
                    let lexical_form = binding_text(self.value.take(), memory)?;
                    let language = Member::text(self.language.take());
                    if let Some(lang) = language.as_deref() {
                        if let Err(error) = purrdf_iri::langtag::parse_with(lang, purrdf_iri::langtag::Profile::ConcreteSyntaxLangtagBounded) {
                            return Err(fmt_with_memory(&format_args!("invalid language tag `{lang}` ({}: {})", error.diagnostic_code(), error.message()), memory));
                        }
                    }
                    let direction_token = Member::text(self.its_dir.take().or_else(|| self.dir.take()));
                    let direction = match direction_token.as_deref() {
                        Some(token) => Some(RdfTextDirection::from_str_token(token).ok_or_else(|| fmt_with_memory(&format_args!("unknown base direction `{token}`"), memory))?),
                        None => None,
                    };
                    if let Some(token) = direction_token { release_text(token, memory)?; }
                    let datatype = Member::text(self.datatype.take());
                    let datatype = match datatype {
                        Some(value) => owned_text(value, memory)?,
                        None => resolve_datatype(None, language.is_some(), direction.is_some(), memory)?,
                    };
                    Ok(TermValue::Literal { lexical_form, datatype, language: language.map(|lang| owned_text(lang, memory)).transpose()?, direction })
                }
                "triple" => match self.value.take() {
                    Some(BindingValue::Triple(mut parts)) => {
                        let term = parts.finish(memory);
                        release_parts(*parts, memory)?;
                        memory.release_bytes(size_of::<TripleParts>()).map_err(Error::Storage)?;
                        term
                    }
                    _ => Err(fmt_with_memory("triple binding has no object `value`", memory)),
                },
                other => Err(fmt_with_memory(&format_args!("unknown binding type `{other}`"), memory)),
            }
        })();
        release_text(kind, memory)?;
        release_fields(self, memory)?;
        result
    }
'''+back[b:]
# Generic original datatype factory.
a=back.index('fn resolve_datatype(');b=back.index('/// Build original format prose',a)
back=back[:a]+'''fn resolve_datatype<S: Admission + ?Sized>(datatype: Option<&str>, has_lang: bool, has_dir: bool, memory: &mut Memory<'_, S>) -> Result<String, Error> {
    memory.string(match datatype { Some(dt) => dt, None if has_lang => language_datatype_iri(has_dir), None => XSD_STRING }).map_err(Error::Storage)
}

'''+back[b:]
# Member names needed by structural decoding have stable static identities.
a=back.index('fn next_key<');b=back.index('/// Decode the first `vars`',a)
back=back[:a]+'''fn next_key<S: Admission + ?Sized>(reader: &mut Reader<'_>, memory: &mut Memory<'_, S>) -> Result<Option<&'static str>, Error> {
    let Some(raw) = reader.next_key_with_memory(memory).map_err(|error| syntax_with_memory(error, memory))? else { return Ok(None); };
    let key = raw.decode_with_memory(memory).map_err(|error| syntax_with_memory(error, memory))?;
    let identity = match key.as_ref() {
        "boolean" => "boolean", "head" => "head", "vars" => "vars", "results" => "results", "bindings" => "bindings",
        "type" => "type", "xml:lang" => "xml:lang", "its:dir" => "its:dir", "dir" => "dir", "datatype" => "datatype", "value" => "value",
        "subject" => "subject", "predicate" => "predicate", "object" => "object", _ => "",
    };
    release_text(key, memory)?;
    Ok(Some(identity))
}

'''+back[b:]
back=back.replace('items.push(item.decode_with_memory(memory).map_err(|error| syntax_with_memory(error, memory))?.into_owned())', 'memory.push(&mut items, owned_text(item.decode_with_memory(memory).map_err(|error| syntax_with_memory(error, memory))?, memory)?).map_err(Error::Storage)?')
# Avoid nested mutable memory borrowing on push arguments.
back=back.replace('Event::String(item) => memory.push(&mut items, owned_text(item.decode_with_memory(memory).map_err(|error| syntax_with_memory(error, memory))?, memory)?).map_err(Error::Storage)?,', 'Event::String(item) => { let decoded = item.decode_with_memory(memory).map_err(|error| syntax_with_memory(error, memory))?; let text = owned_text(decoded, memory)?; memory.push(&mut items, text).map_err(Error::Storage)?; },')
back=back.replace('            rows.push(binding_row(reader, variables, &mut scratch, memory)??);', '            let row = binding_row(reader, variables, &mut scratch, memory)??;\n            memory.push(&mut rows, row).map_err(Error::Storage)?;')
back=back.replace('    Ok((rows, truncated))', '    release_scratch(scratch, memory)?;\n    Ok((rows, truncated))')
# Full decoder shared scratch has no live open boxes on successful row return.
back=back.replace('    Ok(rows.map(|rows| ParsedSolutions { variables, rows }))', '    release_scratch(scratch, memory)?;\n    Ok(rows.map(|rows| ParsedSolutions { variables, rows }))')
back=back.replace('            binding_rows(&mut rows, &variables, &mut scratch, memory)?','            let result = binding_rows(&mut rows, &variables, &mut scratch, memory);\n            rows.release_with_memory(memory).map_err(Error::Storage)?;\n            result?')
# Utility destruction releases only actual payload capacities, never used to admit a factory.
at=back.index('/// Build original format prose')
back=back[:at]+'''fn owned_text<S: Admission + ?Sized>(text: Cow<'_, str>, memory: &mut Memory<'_, S>) -> Result<String, Error> {
    match text { Cow::Borrowed(text) => memory.string(text).map_err(Error::Storage), Cow::Owned(text) => Ok(text) }
}

fn release_text<S: Admission + ?Sized>(text: Cow<'_, str>, memory: &mut Memory<'_, S>) -> Result<(), Error> {
    match text { Cow::Borrowed(_) => Ok(()), Cow::Owned(text) => memory.release_string(text).map_err(Error::Storage) }
}

fn binding_text<S: Admission + ?Sized>(value: Option<BindingValue<'_>>, memory: &mut Memory<'_, S>) -> Result<String, Error> {
    match value { Some(BindingValue::String(text)) => owned_text(text, memory), value => {
        if let Some(value) = value { release_binding_value(value, memory)?; }
        Err(fmt_with_memory("binding has no string `value`", memory))
    } }
}

fn release_error<S: Admission + ?Sized>(error: Error, memory: &mut Memory<'_, S>) -> Result<(), Error> {
    match error {
        Error::MalformedTerm(text) | Error::Format(text) | Error::InvalidNamespace(text) | Error::Internal(text) | Error::Write { message: text, .. } => memory.release_string(text).map_err(Error::Storage),
        Error::Storage(_) => Ok(()),
    }
}

fn release_term<S: Admission + ?Sized>(term: TermValue, memory: &mut Memory<'_, S>) -> Result<(), Error> {
    // Destruction accounting only: every payload was originally born under
    // this grant. Use the original native visitor before dropping its owner.
    let mut bytes = 0usize;
    let flow = term.visit_terms_with_memory(|term, _| {
        let own = match term {
            TermValue::Iri(text) | TermValue::Blank { label: text, .. } => Some(text.capacity()),
            TermValue::Literal { lexical_form, datatype, language, .. } => lexical_form.capacity().checked_add(datatype.capacity()).and_then(|n| n.checked_add(language.as_ref().map_or(0, String::capacity))),
            TermValue::Triple { .. } => size_of::<TermValue>().checked_mul(3),
        };
        match own.and_then(|own| bytes.checked_add(own)) {
            Some(total) => { bytes=total; std::ops::ControlFlow::Continue(()) },
            None => std::ops::ControlFlow::Break(StorageError::SizeOverflow),
        }
    }, memory).map_err(Error::Storage)?;
    if let std::ops::ControlFlow::Break(error) = flow { return Err(Error::Storage(error)); }
    drop(term);
    memory.release_bytes(bytes).map_err(Error::Storage)
}

fn release_parts<S: Admission + ?Sized>(parts: TripleParts, memory: &mut Memory<'_, S>) -> Result<(), Error> {
    for term in [parts.subject, parts.predicate, parts.object].into_iter().flatten() {
        match term { Ok(term) => release_term(term, memory)?, Err(error) => release_error(error, memory)? }
    }
    Ok(())
}

fn release_binding_value<S: Admission + ?Sized>(value: BindingValue<'_>, memory: &mut Memory<'_, S>) -> Result<(), Error> {
    match value {
        BindingValue::String(text) => release_text(text, memory),
        BindingValue::Triple(parts) => {
            release_parts(*parts, memory)?;
            memory.release_bytes(size_of::<TripleParts>()).map_err(Error::Storage)
        }
        BindingValue::Other => Ok(()),
    }
}

fn release_fields<S: Admission + ?Sized>(fields: &mut BindingFields<'_>, memory: &mut Memory<'_, S>) -> Result<(), Error> {
    for member in [fields.kind.take(), fields.language.take(), fields.its_dir.take(), fields.dir.take(), fields.datatype.take()].into_iter().flatten() {
        if let Member::String(text) = member { release_text(text, memory)?; }
    }
    if let Some(value) = fields.value.take() { release_binding_value(value, memory)?; }
    Ok(())
}

fn release_scratch<S: Admission + ?Sized>(mut scratch: RowScratch<'_>, memory: &mut Memory<'_, S>) -> Result<(), Error> {
    while let Some(open) = scratch.open.pop() {
        match open {
            Open::Binding(mut fields) => { release_fields(&mut fields, memory)?; drop(fields); memory.release_bytes(size_of::<BindingFields<'_>>()).map_err(Error::Storage)?; },
            Open::Triple(parts) => { release_parts(*parts, memory)?; memory.release_bytes(size_of::<TripleParts>()).map_err(Error::Storage)?; },
        }
    }
    memory.release_vec(scratch.open).map_err(Error::Storage)?;
    memory.release_vec(scratch.bound).map_err(Error::Storage)
}

'''+back[at:]
# Replace current segments (front has changed length since first splice).
start=s.index('pub fn from_json(bytes:');end=s.index('/// Parse a SPARQL Results JSON `ASK`',start)
s=s[:start]+front+s[end:]
start=s.index('fn select_document(');end=s.index('\n#[cfg(test)]',start)
s=s[:start]+back+s[end:]
# The syntactic `.finish()` rewrite applies only to Reader, not binding fields.
s=s.replace('fn fmt_with_memory(&self, f:', 'fn fmt(&self, f:')
s=s.replace('Open::Binding(fields) => {\n                    let term = fields.finish_with_memory(memory);', 'Open::Binding(mut fields) => {\n                    let term = fields.finish(memory);\n                    drop(fields);\n                    memory.release_bytes(size_of::<BindingFields<\'_>>()).map_err(Error::Storage)?;')
s=s.replace('Some(total) => { bytes=total; std::ops::ControlFlow::Continue(()) },', 'Some(total) => { bytes=total; Ok(std::ops::ControlFlow::Continue(())) },')
s=s.replace('None => std::ops::ControlFlow::Break(StorageError::SizeOverflow),', 'None => Ok(std::ops::ControlFlow::Break(StorageError::SizeOverflow)),')
save(p,s)
p='crates/sparql-results/src/lib.rs';s=read(p).replace('    provenance_from_json,','    provenance_from_json, from_json_with_memory, ReadError,');save(p,s)
patch=''.join(''.join(difflib.unified_diff(old.splitlines(True),new.splitlines(True),fromfile='a/'+path,tofile='b/'+path)) for path,(old,new) in images.items())
(stage/'native-json-results-owner-draft.patch').write_text(patch)
for path,(_,new) in images.items(): (stage/(Path(path).stem+'-native-results-postimage.rs')).write_text(new)
