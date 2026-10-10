from pathlib import Path
import difflib
import hashlib
import json

root = Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
stage = root / '.stage/sparql-eval-complete-bounded-workspace'
paths = ['crates/cdt/src/tree.rs', 'crates/cdt/src/term.rs', 'crates/cdt/src/memory.rs',
         'crates/cdt/tests/value_relations.rs', 'crates/sparql-eval/src/cdt_fn.rs',
         'crates/xsd/src/exact/error.rs', 'crates/xsd/src/exact/integer.rs',
         'crates/xsd/src/exact/decimal.rs', 'crates/lex/src/allocation/text.rs']
original = {path: (root / path).read_text() for path in paths}
post = dict(original)

def item_end(text, start):
    opened = text.index('{', start)
    depth = 1
    at = opened + 1
    while depth:
        if text[at] == '{': depth += 1
        elif text[at] == '}': depth -= 1
        at += 1
    return at

def method(text, marker, from_at=0):
    at = text.index(marker, from_at)
    start = text.rfind('\n', 0, at) + 1
    while start:
        previous = text.rfind('\n', 0, start - 1) + 1
        line = text[previous:start].strip()
        if line.startswith('///') or line.startswith('#['):
            start = previous
        else:
            break
    return start, item_end(text, at), text[start:at]

clone_macro = '''// The public clone entry and resident Clone use one protocol. Each payload
// retains its own native clone body, including the original iterative tree law.
macro_rules! clone_entry {
    ($(#[$meta:meta])* $type:ty, $resident:literal) => {
        impl $type {
            $(#[$meta])*
            pub fn clone_admitted(
                &self,
                storage: &mut dyn $crate::memory::Storage,
            ) -> Result<(Self, usize), $crate::memory::StorageError> {
                let mut memory = $crate::memory::Memory::new(storage);
                let value = memory.scope(|memory| self.clone_with_memory(memory))?;
                Ok((value, memory.admitted_bytes()))
            }
        }
        impl Clone for $type {
            fn clone(&self) -> Self {
                self.clone_admitted(&mut $crate::memory::Resident)
                    .expect($resident).0
            }
        }
    };
}
pub(crate) use clone_entry;

'''
tree = 'crates/cdt/src/tree.rs'
insert = post[tree].index('impl CdtTerm {\n    /// Copy under original native storage')
post[tree] = post[tree][:insert] + clone_macro + post[tree][insert:]
for path, target, resident in [(tree, 'CdtTerm', 'resident CDT clone capacity'),
                               (tree, 'CdtValue', 'resident CDT clone capacity'),
                               ('crates/cdt/src/term.rs', 'CdtLiteral', 'resident CDT literal capacity'),
                               ('crates/cdt/src/term.rs', 'CdtKey', 'resident CDT key capacity')]:
    text = post[path]
    impl_at = text.index('impl ' + target + ' {', text.index('fn resident_error') if path.endswith('term.rs') else 0)
    start, end, prefix = method(text, 'pub fn clone_admitted(', impl_at)
    docs = prefix[:prefix.rfind('\n') + 1]
    assert docs.strip().startswith('///'), (path, target, repr(prefix))
    text = text[:start] + text[end:]
    old = text.index('impl Clone for ' + target + ' {')
    old_end = item_end(text, old)
    call = 'crate::tree::clone_entry! {\n' + docs + '    ' + target + ', "' + resident + '"\n}'
    post[path] = text[:old] + call + text[old_end:]

memory = 'crates/cdt/src/memory.rs'
text = post[memory]
marker = '/// A caller\'s concrete storage owner and fallible native box factory.'
insert = text.index(marker)
text = text[:insert] + '''// A lexical leaf dies before its original capacity is refunded. The capacity
// getter remains payload-specific, while this destruction protocol has one body.
macro_rules! release_entry {
    () => {
        pub(crate) fn release_with_memory(
            self,
            memory: &mut $crate::memory::Memory<'_>,
        ) -> Result<(), $crate::memory::StorageError> {
            let bytes = self.owned_bytes()?;
            drop(self);
            memory.release_bytes(bytes)
        }
    };
}
pub(crate) use release_entry;

// Storage stays object-safe: these two typed doors share the actual native
// factory and differ only in the payload selected by their return types.
macro_rules! native_box_factories {
    ($( $(#[$meta:meta])* $name:ident : $payload:ty ),+ $(,)?) => { $(
        $(#[$meta])*
        fn $name(&mut self, value: $payload) -> Result<Box<$payload>, StorageError> {
            purrdf_lex::allocation::try_boxed(value)
                .map_err(|_| StorageError::AllocationFailed)
        }
    )+ };
}

''' + text[insert:]
start = text.index('pub trait Storage: Admission {')
end = item_end(text, start)
text = text[:start] + '''pub trait Storage: Admission {
    native_box_factories! {
        /// Fallibly allocate the already admitted nested composite payload.
        boxed_value: CdtValue,
        /// Fallibly allocate the already admitted nested triple payload.
        boxed_triple: CdtTripleTerm,
    }
}''' + text[end:]
start = text.index('impl Storage for Resident {')
end = item_end(text, start)
text = text[:start] + 'impl Storage for Resident {}' + text[end:]
post[memory] = text

term = 'crates/cdt/src/term.rs'
text = post[term]
old = '''    pub(crate) fn release_with_memory(self, memory: &mut Memory<'_>) -> Result<(), StorageError> {
        let bytes = self.owned_bytes()?;
        drop(self);
        memory.release_bytes(bytes)
    }'''
assert text.count(old) == 2
post[term] = text.replace(old, '    crate::memory::release_entry!();')

grant = 'crates/cdt/tests/value_relations.rs'
text = post[grant]
at = text.index('impl NativeGrant {')
end = item_end(text, at) - 1
text = text[:end] + '''    fn boxed<T>(&self, value: T) -> Result<Box<T>, StorageError> {
        if self.refuse_box {
            return Err(StorageError::AllocationFailed);
        }
        purrdf_lex::allocation::try_boxed(value).map_err(|_| StorageError::AllocationFailed)
    }
''' + text[end:]
start = text.index('impl Storage for NativeGrant {')
end = item_end(text, start)
text = text[:start] + '''impl Storage for NativeGrant {
    fn boxed_value(&mut self, value: CdtValue) -> Result<Box<CdtValue>, StorageError> {
        self.boxed(value)
    }
    fn boxed_triple(&mut self, value: CdtTripleTerm) -> Result<Box<CdtTripleTerm>, StorageError> {
        self.boxed(value)
    }
}''' + text[end:]
post[grant] = text

fn = 'crates/sparql-eval/src/cdt_fn.rs'
text = post[fn]
at = text.index("impl<'a> CdtStorage<'a> {")
end = item_end(text, at) - 1
text = text[:end] + '''    fn boxed<T>(
        &mut self,
        value: T,
        construct: &'static str,
    ) -> Result<Box<T>, purrdf_cdt::memory::StorageError> {
        purrdf_lex::allocation::try_boxed(value).map_err(|_| {
            self.frame.latch_failure(EvalError::AllocationFailed { construct });
            purrdf_cdt::memory::StorageError::AllocationFailed
        })
    }
''' + text[end:]
start = text.index("impl purrdf_cdt::memory::Storage for CdtStorage<'_> {")
end = item_end(text, start)
text = text[:start] + '''impl purrdf_cdt::memory::Storage for CdtStorage<'_> {
    fn boxed_value(
        &mut self,
        value: CdtValue,
    ) -> Result<Box<CdtValue>, purrdf_cdt::memory::StorageError> {
        self.boxed(value, "native composite value box")
    }
    fn boxed_triple(
        &mut self,
        value: CdtTripleTerm,
    ) -> Result<Box<CdtTripleTerm>, purrdf_cdt::memory::StorageError> {
        self.boxed(value, "native composite triple box")
    }
}''' + text[end:]
post[fn] = text

error = 'crates/xsd/src/exact/error.rs'
post[error] += '''
// Each exact datatype keeps its own lexical reader. Both public entry protocols
// select the same original destination policy and preserve their error channels.
macro_rules! lexical_entry {
    (
        $type:ty;
        try_from_lexical { $(#[$native:meta])* }
        from_str { $(#[$resident:meta])* }
    ) => {
        impl $type {
            $(#[$native])*
            pub fn try_from_lexical(lexical: &str) -> Result<Self, $crate::exact::ExactParseError> {
                Self::parse_with_storage(lexical, &$crate::bigint::scratch::Fallible)
            }
        }
        impl ::std::str::FromStr for $type {
            type Err = $crate::exact::ExactError;
            $(#[$resident])*
            fn from_str(lexical: &str) -> Result<Self, $crate::exact::ExactError> {
                Self::parse_with_storage(lexical, &$crate::bigint::scratch::Unbounded)
                    .map_err(|error| error.into_value_error(lexical)
                        .expect("unbounded integer storage"))
            }
        }
    };
}
pub(crate) use lexical_entry;
'''
for path, target in [('crates/xsd/src/exact/integer.rs', 'Integer'),
                     ('crates/xsd/src/exact/decimal.rs', 'Decimal')]:
    text = post[path]
    start, end, prefix = method(text, 'pub fn try_from_lexical(')
    native_docs = prefix[:prefix.rfind('\n') + 1]
    text = text[:start] + text[end:]
    start = text.index('impl FromStr for ' + target + ' {')
    end = item_end(text, start)
    _, _, prefix = method(text, 'fn from_str(', start)
    resident_docs = prefix[:prefix.rfind('\n') + 1]
    entry = 'super::error::lexical_entry! {\n    ' + target + ';\n    try_from_lexical {\n' + native_docs + '    }\n    from_str {\n' + resident_docs + '    }\n}'
    text = text[:start] + entry + text[end:]
    text = text.replace('use std::str::FromStr;\n', '', 1)
    post[path] = text

owned = 'crates/lex/src/allocation/text.rs'
text = post[owned]
start = text.index('impl From<String> for OwnedText {')
end = item_end(text, text.index('impl From<Arc<str>> for OwnedText {', start))
text = text[:start] + '''// Caller spelling conversion has one body; the typed From doors consume an
// owned String/Arc or copy a borrowed slice through its standard Arc conversion.
macro_rules! caller_text_from {
    ($($source:ty),+ $(,)?) => { $(
        impl From<$source> for OwnedText {
            fn from(value: $source) -> Self {
                Self(TextStorage::Caller(value.into()))
            }
        }
    )+ };
}
caller_text_from!(String, &str, Arc<str>);
''' + text[end:]
post[owned] = text

out = stage / 'lower-home-consolidation-postimages'
pre = stage / 'lower-home-consolidation-preimages'
for path in paths:
    for folder, contents in [(out, post[path]), (pre, original[path])]:
        destination = folder / (path + '.txt')
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(contents)
identities = [{'path':path, 'preimage_sha256':hashlib.sha256(original[path].encode()).hexdigest(),
               'postimage_sha256':hashlib.sha256(post[path].encode()).hexdigest()}
              for path in paths]
(stage / 'lower-home-consolidation-identities.json').write_text(json.dumps(identities, indent=2) + '\n')
print('Nine current-source Stage postimages written; no shipping or ledger mutation.')
