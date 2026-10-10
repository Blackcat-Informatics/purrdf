from pathlib import Path
import re
stage=Path(__file__).parent
p=stage/'crates/lex/src/walk.rs'
s=p.read_text()
start=s.index('pub struct Nested<T: Dismantle>')
end=s.index('/// One token of a value',start)
part=s[start:end]
part=part.replace('pub struct Nested<T: Dismantle>(Option<Box<T>>, Option<Box<T>>);','pub struct Nested<T: Dismantle, C = Box<T>>(Option<Box<T>>, Option<C>);')
part=re.sub(r'impl<T: ([^>]+)>',r'impl<T: \1, C>',part).replace('Nested<T>', 'Nested<T, C>')
a=part.index('    /// Temporarily thread an ancestor')
b=part.index('    fn take_box',a)
part=part[:a]+'''    /// The vacant edge's continuation storage for a destructive walk.
    /// This storage must be empty again before ordinary destruction.
    pub fn dismantle_continuation(&mut self) -> &mut Option<C> {
        assert!(self.0.is_none(), "a continuation requires an evacuated child");
        &mut self.1
    }

    /// Admit the exact boxed payload layout before fallible construction.
    /// The caller retains this memory's original grant with the returned tree.
    ///
    /// # Errors
    /// Returns layout, admission or physical allocation refusal.
    pub fn try_new<S: crate::allocation::Admission + ?Sized>(
        value: T,
        memory: &mut crate::allocation::Memory<'_, S>,
    ) -> Result<Self, crate::allocation::StorageError> {
        let bytes = core::alloc::Layout::new::<T>().size();
        memory.add_bytes(bytes)?;
        match crate::allocation::try_boxed(value) {
            Ok(value) => Ok(Self::from(value)),
            Err(_) => {
                memory.release_bytes(bytes)?;
                Err(crate::allocation::StorageError::AllocationFailed)
            }
        }
    }

'''+part[b:]
part+='''impl<T: Dismantle> Nested<T> {
    /// Store an ancestor in an evacuated homogeneous child edge.
    pub fn set_dismantle_parent(&mut self, parent: Option<Box<T>>) {
        let slot = self.dismantle_continuation();
        assert!(slot.is_none(), "dismantling links require an empty slot");
        *slot = parent;
    }

    /// Remove the homogeneous continuation before ordinary destruction.
    pub fn take_dismantle_parent(&mut self) -> Option<Box<T>> {
        self.dismantle_continuation().take()
    }
}

'''
s=s[:start]+part+s[end:]
p.write_text(s)
p=stage/'crates/sparql-algebra/src/tree.rs'
s=p.read_text().replace('use crate::owned::{DropWork, Owned, reclaim};','use crate::owned::{DropOwner, Owned, release_owned};')
a=s.index('pub trait Subtree:')
b=s.index('/// Implements the traits the three list edges',a)
s=s[:a]+'''pub trait Subtree: Sized + sealed::Sealed {
    /// Dismantle this node through its existing boxes and list buffers.
    #[doc(hidden)]
    fn release(self);
}

macro_rules! subtree {
    ($($ty:ty => |$node:ident| $body:expr;)+) => {$(
        impl sealed::Sealed for $ty {}
        impl Subtree for $ty {
            fn release(self) { let $node = self; $body }
        }
    )+};
}

subtree! {
    crate::GraphPattern => |node| release_owned(Owned::Pattern(node));
    crate::Expression => |node| release_owned(Owned::Expr(node));
    crate::PropertyPathExpression => |node| release_owned(Owned::Path(node));
    crate::TriplePattern => |node| release_owned(Owned::Triple(node));
    crate::GroundTriple => |node| {
        let crate::GroundTriple { subject, predicate: _, object } = node;
        release_owned(Owned::Ground(subject));
        release_owned(Owned::Ground(object));
    };
    (crate::ArithmeticOperator, crate::Expression) => |node| release_owned(Owned::Expr(node.1));
}

macro_rules! child_node {
    ($($ty:ty => $constructor:ident),+ $(,)?) => {$(
        impl purrdf_lex::walk::Dismantle for $ty {
            fn dismantle(node: Box<Self>) {
                purrdf_lex::walk::dismantle_owned(DropOwner::$constructor(node));
            }
        }
    )+};
}

child_node!(
    crate::GraphPattern => pattern,
    crate::Expression => expression,
    crate::PropertyPathExpression => path,
    crate::TriplePattern => triple,
    crate::GroundTriple => ground,
);

fn release_all<T: Subtree>(nodes: Vec<T>) {
    for node in nodes { node.release(); }
}

/// One boxed algebra node with existing storage for a destructor continuation.
/// No continuation allocation is made; the node's original box is preserved.
pub type Child<T> = purrdf_lex::walk::Nested<T, DropOwner>;

'''+s[b:]
needle='''            /// The nodes, in order, taken out for the iterative drop; empty once taken.'''
addition='''            /// Reserve this list's buffer under its original memory grant.
            ///
            /// # Errors
            /// Returns layout, admission or physical allocation refusal.
            pub fn try_reserve<S: purrdf_lex::allocation::Admission + ?Sized>(
                &mut self, required: usize,
                memory: &mut purrdf_lex::allocation::Memory<'_, S>,
            ) -> Result<(), purrdf_lex::allocation::StorageError> {
                memory.reserve(&mut self.0, required)
            }

            /// Append through this list's original admitted allocation owner.
            ///
            /// # Errors
            /// Returns layout, admission or physical allocation refusal.
            pub fn try_push<S: purrdf_lex::allocation::Admission + ?Sized>(
                &mut self, value: T,
                memory: &mut purrdf_lex::allocation::Memory<'_, S>,
            ) -> Result<(), purrdf_lex::allocation::StorageError> {
                memory.push(&mut self.0, value)
            }

            pub(crate) fn dismantle_parts(&mut self) -> (&mut Vec<T>, &mut Option<DropOwner>) {
                (&mut self.0, &mut self.1)
            }

'''
s=s.replace(needle,addition+needle)
s=s.replace('release_all(core::mem::take(&mut self.0));','assert!(self.1.is_none(), "a list continuation must be removed before drop");\n                release_all(core::mem::take(&mut self.0));')
for name in ['Chain','NonEmpty','Args']:
 s=s.replace(f'pub struct {name}<T: Subtree>(Vec<T>);',f'pub struct {name}<T: Subtree>(Vec<T>, Option<DropOwner>);')
s=s.replace('Self(self.0.clone())', 'Self(self.0.clone(), None)')
s=s.replace('Self(nodes)', 'Self(nodes, None)').replace('Self(vec![first])','Self(vec![first], None)').replace('Self(Vec::new())','Self(Vec::new(), None)').replace('Self(nodes.into())','Self(nodes.into(), None)').replace('Self(iter.into_iter().collect())','Self(iter.into_iter().collect(), None)')
for name in ['Chain','NonEmpty']:
 s=s.replace(f'{name}(self.into_vec().into_iter().map(f).collect())',f'{name}(self.into_vec().into_iter().map(f).collect(), None)')
 s=s.replace(f'{name}(self.0.iter().map(f).collect())',f'{name}(self.0.iter().map(f).collect(), None)')
 s=s.replace(f'.map({name})',f'.map(|nodes| {name}(nodes, None))')
s=s.replace('''implements [`Drop`] itself: it moves the subtree it owns into a work list and
//! dismantles that list in a loop, moving each node's own children onto the list
//! before letting the node go. Every node the loop drops has already given up its
//! children, so no drop recurses more than a fixed number of frames, however deep
//! the tree. Quoted ground and pattern triples instead rotate their original child
//! slots through the shared allocation-free dismantling body; their drop needs no
//! work-list allocation, including after admission failure.''','''implements [`Drop`] itself. Destruction threads ancestor continuations through
//! evacuated child and list edges, then runs the workspace's allocation-free
//! dismantling loop over the original boxes and vector buffers. Every node is
//! emptied before ordinary destruction. Cleanup therefore takes bounded stack and
//! allocates no work list, including when construction failed admission.''')
p.write_text(s)
