from pathlib import Path
import difflib

root = Path(__file__).resolve().parents[2]
stage = Path(__file__).resolve().parent
path = root / 'crates/cdt/src/ops.rs'
before = path.read_text()
text = before

def replace_fn(name, new):
    global text
    start = text.index('fn ' + name + '(')
    line = text.rfind('\n', 0, start) + 1
    opening = text.index('{', start)
    # Rust functions here have balanced braces in their original strings/comments.
    depth = 1
    end = opening + 1
    while depth:
        if text[end] == '{': depth += 1
        elif text[end] == '}': depth -= 1
        end += 1
    text = text[:line] + new.strip() + text[end:]

text = text.replace('use alloc::boxed::Box;\n', '')
text = text.replace('use crate::literal::LiteralValue;', 'use crate::literal::{NativeLiteralValue, release_xsd_with_memory};\nuse crate::memory::CdtMemory;\nuse core::alloc::Layout;')

replace_fn('denotation', r'''
fn denotation(literal: &CdtLiteral, memory: &mut Memory<'_>) -> Result<Denotation, StorageError> {
    if literal.language.is_some() { return Ok(Denotation::LanguageTagged); }
    Ok(match crate::literal::resolve_literal_with_memory(&literal.lexical, &literal.datatype, memory)? {
        NativeLiteralValue::Cdt(value) => Denotation::Composite(value),
        NativeLiteralValue::Xsd(value) => Denotation::Xsd(value),
        NativeLiteralValue::IllTyped => Denotation::IllTyped,
        NativeLiteralValue::Opaque => Denotation::Unmodelled,
    })
}

impl Denotation {
    fn release(self, memory: &mut Memory<'_>) -> Result<(), StorageError> {
        match self {
            Self::Composite(value) => value.release_with_memory(memory),
            Self::Xsd(value) => release_xsd_with_memory(value, memory),
            Self::LanguageTagged | Self::IllTyped | Self::Unmodelled => Ok(()),
        }
    }
}

/// Numeric scratch is admitted from the native operation certificate before the
/// original kernel runs; it dies inside the kernel before this scope releases it.
fn xsd_relation(a: &XsdValue, b: &XsdValue, equal: bool, memory: &mut Memory<'_>) -> Result<Option<bool>, StorageError> {
    let baseline = memory.live_bytes();
    let mut original_failure = None;
    let mut admit = |layout: purrdf_xsd::exact::cost::NumericOperationLayout| {
        let next = baseline.checked_add(layout.peak_bytes()).ok_or(purrdf_xsd::bigint::LimbScratchError::SizeOverflow)?;
        let required = next.saturating_sub(memory.live_bytes());
        if let Err(error) = memory.add_bytes(required) {
            original_failure = Some(error);
            return Err(purrdf_xsd::bigint::LimbScratchError::AllocationFailed);
        }
        Ok(())
    };
    let answer = if equal {
        purrdf_xsd::ops::value_equal_admitted(a, b, &mut admit)
    } else {
        purrdf_xsd::ops::value_cmp_admitted(a, b, &mut admit).map(|order| order.map(|order| order == Ordering::Less))
    };
    drop(admit);
    memory.release_bytes(memory.live_bytes() - baseline)?;
    if let Some(error) = original_failure { return Err(error); }
    answer.map_err(|error| match error {
        purrdf_xsd::bigint::LimbScratchError::SizeOverflow => StorageError::SizeOverflow,
        purrdf_xsd::bigint::LimbScratchError::AllocationFailed
        | purrdf_xsd::bigint::LimbScratchError::Capacity { .. }
        | purrdf_xsd::bigint::LimbScratchError::Exhausted { .. }
        | purrdf_xsd::bigint::LimbScratchError::Retained { .. } => StorageError::AllocationFailed,
    })
}
''')

replace_fn('composite_reach', r'''
fn composite_reach(term: &CdtTerm, memory: &mut Memory<'_>) -> Result<Option<Reach>, StorageError> {
    match term {
        CdtTerm::Composite(_) => Ok(Some(Reach::Syntactic)),
        CdtTerm::Literal(literal) => match denotation(literal, memory)? {
            Denotation::Composite(value) => Ok(Some(Reach::Parsed(value))),
            other => { other.release(memory)?; Ok(None) }
        },
        _ => Ok(None),
    }
}
impl Reach {
    fn release(self, memory: &mut Memory<'_>) -> Result<(), StorageError> {
        match self { Self::Syntactic => Ok(()), Self::Parsed(value) => value.release_with_memory(memory) }
    }
}
''')

replace_fn('literal_equal', r'''
fn literal_equal(a: &CdtLiteral, b: &CdtLiteral, memory: &mut Memory<'_>) -> Result<LeafEq, StorageError> {
    if a == b { return Ok(LeafEq::Answer(Ok(true))); }
    let left = denotation(a, memory)?;
    let right = denotation(b, memory)?;
    if matches!((&left, &right), (Denotation::Composite(_), Denotation::Composite(_))) {
        if let (Denotation::Composite(left), Denotation::Composite(right)) = (left, right) {
            return Ok(LeafEq::Composites(Some(left), Some(right)));
        }
        unreachable!("composite denotations established");
    }
    let answer = match (&left, &right) {
        (Denotation::IllTyped, _) | (_, Denotation::IllTyped) => Err(ill_typed()),
        (Denotation::Unmodelled, _) | (_, Denotation::Unmodelled) => Err(unmodelled()),
        (Denotation::Composite(_), _) | (_, Denotation::Composite(_)) => Ok(false),
        (Denotation::LanguageTagged, _) | (_, Denotation::LanguageTagged) => Ok(false),
        (Denotation::Xsd(x), Denotation::Xsd(y)) => Ok(xsd_relation(x, y, true, memory)?.unwrap_or(false)),
    };
    left.release(memory)?;
    right.release(memory)?;
    Ok(LeafEq::Answer(answer))
}
''')

replace_fn('composite_against_literal', r'''
fn composite_against_literal(literal: &CdtLiteral, memory: &mut Memory<'_>) -> Result<Result<Option<CdtValue>, CdtTypeError>, StorageError> {
    let denotation = denotation(literal, memory)?;
    let answer = match denotation {
        Denotation::Composite(value) => return Ok(Ok(Some(value))),
        Denotation::IllTyped => Err(ill_typed()),
        Denotation::Unmodelled => Err(unmodelled()),
        _ => Ok(None),
    };
    denotation.release(memory)?;
    Ok(answer)
}
''')

# Preserve every leaf semantic arm while threading the existing memory owner.
start = text.index('fn leaf_equal(')
end = text.index('\n/// SPARQL `<` over two elements', start)
body = text[start:end]
body = body.replace('b: &CdtTerm) -> LeafEq {', "b: &CdtTerm, memory: &mut Memory<'_>) -> Result<LeafEq, StorageError> {")
body = body.replace('LeafEq::Answer(match', 'Ok(LeafEq::Answer(match', 1)
body = body.replace('return literal_equal(p, q)', 'return literal_equal(p, q, memory)')
body = body.replace('composite_against_literal(q)', 'composite_against_literal(q, memory)?').replace('composite_against_literal(p)', 'composite_against_literal(p, memory)?')
body = body.replace('return LeafEq::Composites(None, Some(value))', 'return Ok(LeafEq::Composites(None, Some(value)))').replace('return LeafEq::Composites(Some(value), None)', 'return Ok(LeafEq::Composites(Some(value), None))')
body = body.replace('\n    })\n}', '\n    }))\n}')
text = text[:start] + body + text[end:]

replace_fn('leaf_less_than', r'''
fn leaf_less_than(a: &CdtTerm, b: &CdtTerm, memory: &mut Memory<'_>) -> Result<Result<bool, CdtTypeError>, StorageError> {
    let unordered = || CdtTypeError::undefined("SPARQL `<` is not defined for this pair of elements");
    let (CdtTerm::Literal(p), CdtTerm::Literal(q)) = (a, b) else { return Ok(Err(unordered())); };
    let left = denotation(p, memory)?;
    let right = denotation(q, memory)?;
    let answer = match (&left, &right) {
        (Denotation::IllTyped, _) | (_, Denotation::IllTyped) => Err(ill_typed()),
        (Denotation::Unmodelled, _) | (_, Denotation::Unmodelled) => Err(unmodelled()),
        (Denotation::Xsd(x), Denotation::Xsd(y)) => xsd_relation(x, y, false, memory)?.ok_or_else(unordered),
        _ => Err(unordered()),
    };
    left.release(memory)?;
    right.release(memory)?;
    Ok(answer)
}
''')

replace_fn('resolved', r'''
    fn resolved(self, parsed: Option<CdtValue>, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        match parsed {
            Some(value) => {
                self.release(memory)?;
                Ok(Self::Owned(CdtTerm::Composite(memory.boxed_value(value)?)))
            }
            None => Ok(self),
        }
    }
    fn release(self, memory: &mut Memory<'_>) -> Result<(), StorageError> {
        match self { Self::Borrowed(_) => Ok(()), Self::Owned(term) => term.release_with_memory(memory) }
    }
''')

replace_fn('components', r'''
    fn components(self, memory: &mut Memory<'_>) -> Result<[Self; 3], StorageError> {
        Ok(match self {
            Self::Borrowed(CdtTerm::TripleTerm(triple)) => [Self::Borrowed(&triple.subject), Self::Borrowed(&triple.predicate), Self::Borrowed(&triple.object)],
            Self::Owned(CdtTerm::TripleTerm(mut triple)) => {
                let result = [Self::Owned(mem::replace(&mut triple.subject, CdtTerm::Null)), Self::Owned(mem::replace(&mut triple.predicate, CdtTerm::Null)), Self::Owned(mem::replace(&mut triple.object, CdtTerm::Null))];
                drop(triple);
                memory.release_bytes(Layout::new::<crate::CdtTripleTerm>().size())?;
                result
            }
            _ => unreachable!("components are taken of a triple term"),
        })
    }
''')

replace_fn('children', r'''
    fn children(self, memory: &mut Memory<'_>) -> Result<Kids<'a>, StorageError> {
        Ok(match self {
            Self::Borrowed(CdtTerm::Composite(value)) => match value.contents() {
                CdtContents::List(items) => Kids::BorrowedList(items),
                CdtContents::Map(entries) => Kids::BorrowedMap(entries),
            },
            Self::Owned(CdtTerm::Composite(value)) => {
                let parts = (*value).into_parts();
                memory.release_bytes(Layout::new::<CdtValue>().size())?;
                match parts { CdtParts::List(items) => Kids::OwnedList(items), CdtParts::Map(entries) => Kids::OwnedMap(entries) }
            }
            _ => unreachable!("children are taken of a composite"),
        })
    }
''')

replace_fn('into_sides', r'''
    fn take(&mut self, index: usize) -> Side<'a> {
        match self {
            Self::BorrowedList(items) => Side::Borrowed(&items[index]),
            Self::BorrowedMap(entries) => Side::Borrowed(&entries[index].value),
            Self::OwnedList(items) => Side::Owned(mem::replace(&mut items[index], CdtTerm::Null)),
            Self::OwnedMap(entries) => Side::Owned(mem::replace(&mut entries[index].value, CdtTerm::Null)),
        }
    }
    fn release(self, memory: &mut Memory<'_>) -> Result<(), StorageError> {
        match self {
            Self::BorrowedList(_) | Self::BorrowedMap(_) => Ok(()),
            Self::OwnedList(mut items) => {
                for item in &mut items { mem::replace(item, CdtTerm::Null).release_with_memory(memory)?; }
                memory.release_vec(items)
            }
            Self::OwnedMap(mut entries) => {
                for entry in &mut entries {
                    mem::replace(&mut entry.value, CdtTerm::Null).release_with_memory(memory)?;
                    // Move the original key into its destruction-only release home.
                    mem::replace(&mut entry.key, CdtKey::Iri(alloc::string::String::new())).release_with_memory(memory)?;
                }
                memory.release_vec(entries)
            }
        }
    }
''')
start = text.index('/// [`Kids::into_sides`]')
end = text.index('/// What a pair on the equality work list', start)
text = text[:start] + text[end:]

replace_fn('term_equal', r'''
pub fn term_equal(a: &CdtTerm, b: &CdtTerm) -> Result<bool, CdtTypeError> {
    try_term_equal(a, b, &mut Resident).expect("resident composite equality storage")
}
/// The same SEP equality with original native storage admission.
/// # Errors
/// Physical refusal is separate from a semantic type error.
pub fn try_term_equal(a: &CdtTerm, b: &CdtTerm, storage: &mut impl Storage) -> Result<Result<bool, CdtTypeError>, StorageError> {
    let mut memory = Memory::new(storage);
    term_equal_with_memory(a, b, &mut memory)
}
fn term_equal_with_memory(a: &CdtTerm, b: &CdtTerm, memory: &mut Memory<'_>) -> Result<Result<bool, CdtTypeError>, StorageError> {
    let mut work = Vec::new();
    memory.push(&mut work, (Side::Borrowed(a), Side::Borrowed(b)))?;
    equal_worklist(work, memory)
}
''')
replace_fn('membership_equal', r'''
pub(crate) fn membership_equal(item: &CdtTerm, term: &CdtTerm) -> Result<bool, CdtTypeError> {
    membership_equal_with_memory(item, term, &mut Memory::new(&mut Resident)).expect("resident membership storage")
}
pub(crate) fn membership_equal_with_memory(item: &CdtTerm, term: &CdtTerm, memory: &mut Memory<'_>) -> Result<Result<bool, CdtTypeError>, StorageError> {
    if let (CdtTerm::Blank(p), CdtTerm::Blank(q)) = (item, term) { return Ok(Ok(p == q)); }
    term_equal_with_memory(item, term, memory)
}
''')
replace_fn('equal_worklist', r'''
fn equal_worklist(mut work: Vec<(Side<'_>, Side<'_>)>, memory: &mut Memory<'_>) -> Result<Result<bool, CdtTypeError>, StorageError> {
    let result = (|| {
        let mut withheld = None;
        while let Some((x, y)) = work.pop() {
            match shape(x.term(), y.term()) {
                Shape::Triples => {
                    let [ls, lp, lo] = x.components(memory)?;
                    let [rs, rp, ro] = y.components(memory)?;
                    memory.push(&mut work, (lo, ro))?;
                    memory.push(&mut work, (lp, rp))?;
                    memory.push(&mut work, (ls, rs))?;
                }
                Shape::Composites => {
                    let (mut left, mut right) = (x.children(memory)?, y.children(memory)?);
                    let unequal = left.is_map() != right.is_map() || left.len() != right.len()
                        || (left.is_map() && (0..left.len()).any(|index| left.key(index) != right.key(index)));
                    if !unequal {
                        for index in 0..left.len() { memory.push(&mut work, (left.take(index), right.take(index)))?; }
                    }
                    left.release(memory)?;
                    right.release(memory)?;
                    if unequal { return Ok(Ok(false)); }
                }
                Shape::Leaves => {
                    let verdict = leaf_equal(x.term(), y.term(), memory)?;
                    match verdict {
                        LeafEq::Composites(left, right) => {
                            let left = x.resolved(left, memory)?;
                            let right = y.resolved(right, memory)?;
                            memory.push(&mut work, (left, right))?;
                        }
                        LeafEq::Answer(answer) => {
                            x.release(memory)?;
                            y.release(memory)?;
                            match answer {
                                Ok(true) => {},
                                Ok(false) => return Ok(Ok(false)),
                                Err(error) => { if withheld.is_none() { withheld = Some(error); } },
                            }
                        }
                    }
                }
            }
        }
        Ok(match withheld { Some(error) => Err(error), None => Ok(true) })
    })();
    // An early semantic verdict still destroys every originally owned sibling.
    if result.is_ok() {
        while let Some((left, right)) = work.pop() { left.release(memory)?; right.release(memory)?; }
    }
    memory.release_vec(work)?;
    result
}
''')

# All resident sequence equality entries use that same native worklist.
replace_fn('list_equal', r'''
pub fn list_equal(a: &[CdtTerm], b: &[CdtTerm]) -> Result<bool, CdtTypeError> {
    let mut storage = Resident;
    let mut memory = Memory::new(&mut storage);
    list_equal_with_memory(a, b, &mut memory).expect("resident list equality storage")
}
fn list_equal_with_memory(a: &[CdtTerm], b: &[CdtTerm], memory: &mut Memory<'_>) -> Result<Result<bool, CdtTypeError>, StorageError> {
    if a.len() != b.len() { return Ok(Ok(false)); }
    let mut work = Vec::new();
    for (p, q) in a.iter().zip(b) { memory.push(&mut work, (Side::Borrowed(p), Side::Borrowed(q)))?; }
    equal_worklist(work, memory)
}
''')
replace_fn('map_equal', r'''
pub fn map_equal(a: &[CdtEntry], b: &[CdtEntry]) -> Result<bool, CdtTypeError> {
    let mut storage = Resident;
    let mut memory = Memory::new(&mut storage);
    map_equal_with_memory(a, b, &mut memory).expect("resident map equality storage")
}
fn map_equal_with_memory(a: &[CdtEntry], b: &[CdtEntry], memory: &mut Memory<'_>) -> Result<Result<bool, CdtTypeError>, StorageError> {
    if a.len() != b.len() || a.iter().zip(b).any(|(p, q)| p.key != q.key) { return Ok(Ok(false)); }
    let mut work = Vec::new();
    for (p, q) in a.iter().zip(b) { memory.push(&mut work, (Side::Borrowed(&p.value), Side::Borrowed(&q.value)))?; }
    equal_worklist(work, memory)
}
''')

replace_fn('term_less_than', r'''
pub fn term_less_than(a: &CdtTerm, b: &CdtTerm) -> Result<bool, CdtTypeError> {
    try_term_less_than(a, b, &mut Resident).expect("resident composite order storage")
}
/// The same SEP partial order with original native storage admission.
/// # Errors
/// Physical refusal is separate from a semantic type error.
pub fn try_term_less_than(a: &CdtTerm, b: &CdtTerm, storage: &mut impl Storage) -> Result<Result<bool, CdtTypeError>, StorageError> {
    let mut memory = Memory::new(storage);
    sequence_less_than(Owner::Borrowed(Seq::List(core::slice::from_ref(a))), Owner::Borrowed(Seq::List(core::slice::from_ref(b))), &mut memory)
        .map(|answer| answer.map(|verdict| verdict == Verdict::Less))
}
''')

# Native ownership release for each original lexicographic frame.
point = text.index('\n    fn len(&self)', text.index('impl Owner'))
text = text[:point] + r'''
    fn release(self, memory: &mut Memory<'_>) -> Result<(), StorageError> {
        match self {
            Self::Borrowed(_) => Ok(()),
            Self::List(items) => Kids::OwnedList(items).release(memory),
            Self::Map(entries) => Kids::OwnedMap(entries).release(memory),
        }
    }
''' + text[point:]
start = text.index('    fn descend(')
end = text.index('\n}\n\n/// A frame', start)
body = text[start:end]
body = body.replace('index: usize) -> Self', "index: usize, memory: &mut Memory<'_>) -> Result<Self, StorageError>")
body = body.replace('return Self::Borrowed(Seq::of(inner))', 'return Ok(Self::Borrowed(Seq::of(inner)))')
body = body.replace('CdtTerm::Composite(value) => Self::parsed(*value),', 'CdtTerm::Composite(value) => { let value = *value; memory.release_bytes(Layout::new::<CdtValue>().size())?; Ok(Self::parsed(value)) },')
body = body.replace('reach: Reach) -> Self', "reach: Reach, memory: &mut Memory<'_>) -> Result<Self, StorageError>")
body = body.replace('self.descend(index)', 'self.descend(index, memory)').replace('Reach::Parsed(value) => Self::parsed(value)', 'Reach::Parsed(value) => Ok(Self::parsed(value))')
text = text[:start] + body + text[end:]

replace_fn('sequence_less_than', r'''
fn sequence_less_than(left: Owner<'_>, right: Owner<'_>, memory: &mut Memory<'_>) -> Result<Result<Verdict, CdtTypeError>, StorageError> {
    let mut stack = Vec::new();
    memory.push(&mut stack, Frame { left, right, index: 0 })?;
    let result = (|| {
        loop {
            let Some(top) = stack.last_mut() else { return Ok(Ok(Verdict::Equal)); };
            let index = top.index;
            let shortest = top.left.len().min(top.right.len());
            if index == shortest {
                match top.left.len().cmp(&top.right.len()) {
                    Ordering::Less => return Ok(Ok(Verdict::Less)),
                    Ordering::Greater => return Ok(Ok(Verdict::NotLess)),
                    Ordering::Equal => {},
                }
                let frame = stack.pop().expect("original active frame");
                frame.left.release(memory)?;
                frame.right.release(memory)?;
                match stack.last_mut() {
                    None => return Ok(Ok(Verdict::Equal)),
                    Some(parent) => { parent.index += 1; continue; },
                }
            }
            if let (Some(left), Some(right)) = (top.left.key(index), top.right.key(index)) {
                match total_key_cmp(left, right) {
                    Ordering::Less => return Ok(Ok(Verdict::Less)),
                    Ordering::Greater => return Ok(Ok(Verdict::NotLess)),
                    Ordering::Equal => {},
                }
            }
            let (x, y) = (top.left.value(index), top.right.value(index));
            if matches!(x, CdtTerm::Blank(_)) || matches!(y, CdtTerm::Blank(_)) {
                return Ok(Err(CdtTypeError::undefined("SPARQL `<` has no answer where a blank node stands, not even against the very same blank node")));
            }
            let reaches = (composite_reach(x, memory)?, composite_reach(y, memory)?);
            match reaches {
                (Some(left), Some(right)) => {
                    let left = top.left.reached(index, left, memory)?;
                    let right = top.right.reached(index, right, memory)?;
                    if left.is_map() != right.is_map() {
                        left.release(memory)?;
                        right.release(memory)?;
                        return Ok(Err(list_against_map()));
                    }
                    memory.push(&mut stack, Frame { left, right, index: 0 })?;
                }
                (left, right) => {
                    if let Some(reach) = left { reach.release(memory)?; }
                    if let Some(reach) = right { reach.release(memory)?; }
                    match term_equal_with_memory(x, y, memory)? {
                        Err(error) => return Ok(Err(error)),
                        Ok(true) => { top.index += 1; },
                        Ok(false) => match leaf_less_than(x, y, memory)? {
                            Err(error) => return Ok(Err(error)),
                            Ok(true) => return Ok(Ok(Verdict::Less)),
                            Ok(false) => return Ok(Ok(Verdict::NotLess)),
                        },
                    }
                }
            }
        }
    })();
    if result.is_ok() {
        while let Some(frame) = stack.pop() { frame.left.release(memory)?; frame.right.release(memory)?; }
    }
    memory.release_vec(stack)?;
    result
}
''')

replace_fn('list_less_than', r'''
pub fn list_less_than(a: &[CdtTerm], b: &[CdtTerm]) -> Result<bool, CdtTypeError> {
    sequence_less_than(Owner::Borrowed(Seq::List(a)), Owner::Borrowed(Seq::List(b)), &mut Memory::new(&mut Resident))
        .expect("resident list order storage").map(|verdict| verdict == Verdict::Less)
}
''')
replace_fn('map_less_than', r'''
pub fn map_less_than(a: &[CdtEntry], b: &[CdtEntry]) -> Result<bool, CdtTypeError> {
    sequence_less_than(Owner::Borrowed(Seq::Map(a)), Owner::Borrowed(Seq::Map(b)), &mut Memory::new(&mut Resident))
        .expect("resident map order storage").map(|verdict| verdict == Verdict::Less)
}
''')
replace_fn('value_less_than', r'''
pub fn value_less_than(a: &CdtValue, b: &CdtValue) -> Result<bool, CdtTypeError> {
    let (left, right) = (Seq::of(a), Seq::of(b));
    if left.is_map() != right.is_map() { return Err(list_against_map()); }
    sequence_less_than(Owner::Borrowed(left), Owner::Borrowed(right), &mut Memory::new(&mut Resident))
        .expect("resident value order storage").map(|verdict| verdict == Verdict::Less)
}
''')

# Failure cleanup observes only the existing original grant AFTER the native
# kernel's owned locals have died. It never admits or adopts a resident result.
text = text.replace("fn term_equal_with_memory(a: &CdtTerm, b: &CdtTerm, memory: &mut Memory<'_>) -> Result<Result<bool, CdtTypeError>, StorageError> {", "fn term_equal_with_memory(a: &CdtTerm, b: &CdtTerm, memory: &mut Memory<'_>) -> Result<Result<bool, CdtTypeError>, StorageError> {\n    relation_scope(memory, |memory| term_equal_kernel(a, b, memory))\n}\nfn term_equal_kernel(a: &CdtTerm, b: &CdtTerm, memory: &mut Memory<'_>) -> Result<Result<bool, CdtTypeError>, StorageError> {")
text = text.replace("fn sequence_less_than(left: Owner<'_>, right: Owner<'_>, memory: &mut Memory<'_>) -> Result<Result<Verdict, CdtTypeError>, StorageError> {", "fn sequence_less_than(left: Owner<'_>, right: Owner<'_>, memory: &mut Memory<'_>) -> Result<Result<Verdict, CdtTypeError>, StorageError> {\n    relation_scope(memory, |memory| sequence_less_than_kernel(left, right, memory))\n}\nfn sequence_less_than_kernel(left: Owner<'_>, right: Owner<'_>, memory: &mut Memory<'_>) -> Result<Result<Verdict, CdtTypeError>, StorageError> {")
point = text.index('// ── Equality')
text = text[:point] + r'''
fn relation_scope<T>(memory: &mut Memory<'_>, body: impl FnOnce(&mut Memory<'_>) -> Result<T, StorageError>) -> Result<T, StorageError> {
    let baseline = memory.live_bytes();
    let result = body(memory);
    if result.is_err() {
        let released = memory.live_bytes().checked_sub(baseline).ok_or(StorageError::SizeOverflow)?;
        memory.release_bytes(released)?;
    }
    result
}

''' + text[point:]

stage.joinpath('ops-native-cdt-postimage.rs').write_text(text)
patch = ''.join(difflib.unified_diff(before.splitlines(True), text.splitlines(True), fromfile='a/crates/cdt/src/ops.rs', tofile='b/crates/cdt/src/ops.rs'))
stage.joinpath('native-cdt-ops-owner-draft.patch').write_text(patch)
print('native CDT relations packet', len(patch), 'bytes')
