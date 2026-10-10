from pathlib import Path
import difflib

root = Path(__file__).resolve().parents[2]
stage = Path(__file__).resolve().parent
images = {}
def read(path):
    text = (root/path).read_text()
    images[path] = [text, text]
    return text
def save(path, text): images[path][1] = text
alloc = 'use crate::allocation::{Admission, Memory, Resident, StorageError};\n'
p = 'crates/lex/src/json_escape.rs'
s = read(p)
s = s.replace('use std::borrow::Cow;', 'use std::borrow::Cow;\n'+alloc)
s = s.replace('    UnpairedLow,\n}', '    UnpairedLow,\n    /// Original physical admission or allocator refusal.\n    Storage(StorageError),\n}')
s = s.replace('        let what = match self.kind {', '        if let JsonEscapeErrorKind::Storage(error) = self.kind {\n            return write!(f, "{error} (byte {})", self.offset);\n        }\n        let what = match self.kind {')
s = s.replace('            JsonEscapeErrorKind::Truncated =>', '            JsonEscapeErrorKind::Storage(_) => unreachable!("handled above"),\n            JsonEscapeErrorKind::Truncated =>',1)
a = s.index('pub fn unescape(body:')
b = s.index('\n#[cfg(test)]',a)
old = s[a:b]
body = old.replace('pub fn unescape(body: &str)', "pub fn unescape_with_memory<'a, S: Admission + ?Sized>(body: &'a str, memory: &mut Memory<'_, S>)").replace("Result<Cow<'_, str>", "Result<Cow<'a, str>")
body = body.replace('    let mut out = String::with_capacity(body.len());', '    let mut out = String::new();\n    memory.reserve_string(&mut out, body.len()).map_err(|error| JsonEscapeError::at(JsonEscapeErrorKind::Storage(error), first))?;\n    let result = (|| {')
body = body.replace('    out.push_str(&body[..first]);','    memory.push_str(&mut out, &body[..first]).map_err(|error| JsonEscapeError::at(JsonEscapeErrorKind::Storage(error), first))?;')
body = body.replace('        out.push(decoded);','        memory.push_char(&mut out, decoded).map_err(|error| JsonEscapeError::at(JsonEscapeErrorKind::Storage(error), at))?;')
body = body.replace('        out.push_str(&body[at..run]);','        memory.push_str(&mut out, &body[at..run]).map_err(|error| JsonEscapeError::at(JsonEscapeErrorKind::Storage(error), at))?;')
body = body.replace('return Ok(Cow::Owned(out));','return Ok(());')
body = body.rstrip()[:-1] + '    })();\n    match result {\n        Ok(()) => Ok(Cow::Owned(out)),\n        Err(error) => {\n            memory.release_string(out).map_err(|storage| JsonEscapeError::at(JsonEscapeErrorKind::Storage(storage), error.offset))?;\n            Err(error)\n        }\n    }\n}\n'
wrapper = '''pub fn unescape(body: &str) -> Result<Cow<'_, str>, JsonEscapeError> {
    unescape_with_memory(body, &mut Memory::new(&mut Resident))
}

/// Decode through the original caller's admission before creating escaped text.
/// # Errors
/// Returns the original escape defect or typed physical refusal.
'''
s=s[:a]+wrapper+body+s[b:]
save(p,s)
p='crates/lex/src/json/error.rs';s=read(p)
s=s.replace('    DuplicateMember,','    DuplicateMember,\n    /// Original native buffer admission or allocator refusal.\n    Storage(crate::allocation::StorageError),')
s=s.replace('            ErrorKind::Expected(what)', '            ErrorKind::Storage(error) => write!(f, "{error}"),\n            ErrorKind::Expected(what)')
s=s.replace('                JsonEscapeErrorKind::Truncated =>','                JsonEscapeErrorKind::Storage(error) => return write!(f, "{error}"),\n                JsonEscapeErrorKind::Truncated =>')
save(p,s)
p='crates/lex/src/json/read.rs';s=read(p)
s=s.replace('use std::borrow::Cow;', 'use std::borrow::Cow;\n'+alloc)
a=s.index('    pub fn decode(&self)'); b=s.index('\n}\n',a)
body=s[a:b].replace('pub fn decode(&self)', "pub fn decode_with_memory<S: Admission + ?Sized>(&self, memory: &mut Memory<'_, S>)").replace('json_escape::unescape(self.raw)', 'json_escape::unescape_with_memory(self.raw, memory)')
s=s[:a]+'''    pub fn decode(&self) -> Result<Cow<'a, str>, Error> {
        self.decode_with_memory(&mut Memory::new(&mut Resident))
    }

    /// Decode with original admission before creating escaped text.
    /// # Errors
    /// Returns the original JSON escape or physical refusal.
'''+body+s[b:]
# One original grammar body, with resident entry points resuming their actual stack.
methods = [('step', 'Event<\'a>'),('next_key', "Option<Str<'a>>"),('begin_object','()'),('begin_array','()'),('skip_value','Range<usize>'),('check_value','Range<usize>'),('finish','()')]
for name, ret in methods:
    signature = ('fn ' if name=='step' else 'pub fn ')+name+'(&mut self) -> Result<'+ret+', Error> {'
    replacement = signature+'\n        let mut resident = Resident;\n        let mut memory = Memory::resume(&mut resident, self.open.capacity());\n        self.'+name+'_with_memory(&mut memory)\n    }\n\n    /// The same original reader law under caller-owned physical admission.\n    /// # Errors\n    /// Returns JSON syntax, limit or physical storage refusal.\n    '+('fn ' if name=='step' else 'pub fn ')+name+"_with_memory<S: Admission + ?Sized>(&mut self, memory: &mut Memory<'_, S>) -> Result<"+ret+', Error> {'
    assert signature in s,name
    s=s.replace(signature,replacement,1)
# Native copies invoke the original native step, not resident adapters.
for name,_ in methods:
    marker='fn '+name+'_with_memory'
    a=s.index(marker); a=s.index('{',a); depth=1;b=a+1
    while depth:
        depth += (s[b]=='{')-(s[b]=='}');b+=1
    body=s[a:b]
    body=body.replace('self.next_event()?', 'self.step_with_memory(memory)?').replace('self.value()', 'self.value_with_memory(memory)').replace('self.check(start)?','self.check_with_memory(start, memory)?')
    s=s[:a]+body+s[b:]
s=s.replace('    fn value(&mut self) -> Result<Event<\'a>, Error> {', "    fn value_with_memory<S: Admission + ?Sized>(&mut self, memory: &mut Memory<'_, S>) -> Result<Event<'a>, Error> {")
s=s.replace('                self.open.push(object);', '                memory.push(&mut self.open, object).map_err(|error| self.error(ErrorKind::Storage(error)))?;')
# Public native event door and explicit destruction of reader's original stack.
at=s.index('    /// [`Reader::next_event`],')
s=s[:at]+'''    /// Pull an event with original admission before growing the nesting stack.
    /// # Errors
    /// Returns JSON syntax, limit or native physical refusal.
    pub fn next_event_with_memory<S: Admission + ?Sized>(&mut self, memory: &mut Memory<'_, S>) -> Result<Event<'a>, Error> {
        self.step_with_memory(memory)
    }

    /// Destroy the original nesting stack before releasing its grant.
    /// # Errors
    /// Returns an unbalanced original admission invariant.
    pub fn release_with_memory<S: Admission + ?Sized>(self, memory: &mut Memory<'_, S>) -> Result<(), StorageError> {
        memory.release_vec(self.open)
    }

'''+s[at:]
a=s.index('    fn check(&mut self, start:');b=s.index('    /// A string or member name',a)
s=s[:a]+'''    fn check_with_memory<S: Admission + ?Sized>(&mut self, start: usize, memory: &mut Memory<'_, S>) -> Result<(), Error> {
        // Sorted names borrow unescaped keys and own escaped keys under the
        // same grant. No opaque table allocation bypasses admission.
        let mut open: Vec<Option<Vec<Cow<'a, str>>>> = Vec::new();
        let result = (|| {
            loop {
                match self.step_with_memory(memory)? {
                    Event::BeginObject { .. } => memory.push(&mut open, self.limits.unique_members.then(Vec::new)).map_err(|error| self.error(ErrorKind::Storage(error)))?,
                    Event::BeginArray { .. } => memory.push(&mut open, None).map_err(|error| self.error(ErrorKind::Storage(error)))?,
                    Event::EndObject { .. } | Event::EndArray { .. } => {
                        let Some(names) = open.pop() else {
                            return Err(Error::new(ErrorKind::Expected("a JSON value"), start));
                        };
                        release_names(names, memory).map_err(|error| self.error(ErrorKind::Storage(error)))?;
                    }
                    Event::Key(raw) => {
                        let Some(names) = open.last_mut() else {
                            return Err(Error::new(ErrorKind::Expected("a JSON value"), start));
                        };
                        let decoded = self.decoded_with_memory(raw, memory)?;
                        if let Some(names) = names {
                            match names.binary_search_by(|name| name.as_ref().cmp(decoded.as_ref())) {
                                Ok(_) => {
                                    release_text(decoded, memory).map_err(|error| self.error(ErrorKind::Storage(error)))?;
                                    return Err(Error::new(ErrorKind::DuplicateMember, raw.at - 1));
                                }
                                Err(index) => {
                                    let required = names.len().checked_add(1).ok_or_else(|| self.error(ErrorKind::Storage(StorageError::SizeOverflow)))?;
                                    memory.reserve(names, required).map_err(|error| self.error(ErrorKind::Storage(error)))?;
                                    names.insert(index, decoded);
                                }
                            }
                        } else {
                            release_text(decoded, memory).map_err(|error| self.error(ErrorKind::Storage(error)))?;
                        }
                        continue;
                    }
                    Event::String(raw) => {
                        let decoded = self.decoded_with_memory(raw, memory)?;
                        release_text(decoded, memory).map_err(|error| self.error(ErrorKind::Storage(error)))?;
                    }
                    Event::Number { .. } | Event::Bool { .. } | Event::Null { .. } => {}
                    Event::End => return Err(Error::new(ErrorKind::Expected("a JSON value"), start)),
                }
                if open.is_empty() { return Ok(()); }
            }
        })();
        while let Some(names) = open.pop() {
            release_names(names, memory).map_err(|error| self.error(ErrorKind::Storage(error)))?;
        }
        memory.release_vec(open).map_err(|error| self.error(ErrorKind::Storage(error)))?;
        result
    }

'''+s[b:]
# Resident DOM still uses the existing decoded wrapper, native check shares it.
sig="    fn decoded(&self, raw: Str<'a>) -> Result<Cow<'a, str>, Error> {"
s=s.replace(sig, sig+"\n        self.decoded_with_memory(raw, &mut Memory::new(&mut Resident))\n    }\n\n    fn decoded_with_memory<S: Admission + ?Sized>(&self, raw: Str<'a>, memory: &mut Memory<'_, S>) -> Result<Cow<'a, str>, Error> {",1)
at=s.index('    fn decoded_with_memory'); end=s.index('    /// Require the document',at)
piece=s[at:end].replace('let decoded = raw.decode()?;', 'let decoded = raw.decode_with_memory(memory)?;').replace('        if decoded.len() > self.limits.max_string_bytes {', '        if decoded.len() > self.limits.max_string_bytes {\n            release_text(decoded, memory).map_err(|error| self.error(ErrorKind::Storage(error)))?;')
s=s[:at]+piece+s[end:]
at=s.index('/// Read `text` as one JSON document')
s=s[:at]+'''fn release_text<S: Admission + ?Sized>(text: Cow<'_, str>, memory: &mut Memory<'_, S>) -> Result<(), StorageError> {
    match text { Cow::Borrowed(_) => Ok(()), Cow::Owned(text) => memory.release_string(text) }
}

fn release_names<S: Admission + ?Sized>(names: Option<Vec<Cow<'_, str>>>, memory: &mut Memory<'_, S>) -> Result<(), StorageError> {
    if let Some(mut names) = names {
        while let Some(name) = names.pop() { release_text(name, memory)?; }
        memory.release_vec(names)?;
    }
    Ok(())
}

'''+s[at:]
save(p,s)
patch=''.join(''.join(difflib.unified_diff(old.splitlines(True),new.splitlines(True),fromfile='a/'+path,tofile='b/'+path)) for path,(old,new) in images.items())
(stage/'native-json-lex-owner-draft.patch').write_text(patch)
for path,(_,new) in images.items(): (stage/(Path(path).stem+'-native-json-postimage.rs')).write_text(new)
