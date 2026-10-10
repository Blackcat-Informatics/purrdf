# Why not Rust: Ignored Stage-only text assembly preserves all shipping source untouched.
W='crates/sparql-eval/src/workspace.rs'
s=read(W)
a=s.index('impl<T> From<std::sync::Arc<T>>')
s=s[:a]+'''impl<T> AsRef<T> for SharedWorkspace<T> {
    fn as_ref(&self)->&T { self }
}

'''+s[a:]
a=s.index('    /// Borrow each existing key and value without an iterator allocation.')
s=s[:a]+'''    /// Borrow membership without cloning a key or allocating a table.
    #[must_use]
    pub fn contains_key<Q: ?Sized + Eq + std::hash::Hash>(&self,key:&Q)->bool
    where K:std::borrow::Borrow<Q> { self.get(key).is_some() }

    /// Borrow payloads without a copied iterator buffer.
    pub fn values(&self)->impl Iterator<Item=&V> { self.iter().map(|(_,value)|value) }

    #[cfg(test)]
    pub(crate) fn insert(&mut self,key:K,value:V)->Option<V> {
        self.insert_admitted(key,value,&WorkspaceCapability::resident()).expect("resident table allocation failed")
    }

'''+s[a:]
post[W]=s
T='crates/sparql-algebra/src/traits.rs'
s=read(T)+'''\nimpl crate::GroundTerm {
    /// Compare the original structural scripts with native spill admission.
    /// # Errors
    /// Returns concrete layout, admission or allocator refusal.
    pub fn eq_with_memory<S:purrdf_lex::allocation::Admission+?Sized>(
        &self,other:&Self,memory:&mut purrdf_lex::allocation::Memory<'_,S>,
    )->Result<bool,purrdf_lex::allocation::StorageError> {
        nodes_eq_with_memory(NodeRef::Ground(self),NodeRef::Ground(other),memory)
    }
}
'''
post[T]=s
S='crates/sparql-eval/src/substitute.rs'
s=read(S).replace('fn blank_admitted(','pub(crate) fn blank_admitted(',1)
a=s.index('    /// The legacy resident boundary keeps its infallible allocation contract.',s.index('impl GroundFailure'))
s=s[:a]+'''    /// Move the original diagnostic or operational cause to an expression boundary.
    pub(crate) fn into_eval_error(self)->crate::EvalError {
        match self { Self::Operational(error)=>error,Self::Diagnostic(error)=>crate::EvalError::RetainedDiagnostic(error) }
    }

'''+s[a:]
post[S]=s
E='crates/sparql-eval/src/error.rs'
s=read(E)
s=s.replace('    NativeDiagnostic(NativeDiagnostic),','    NativeDiagnostic(NativeDiagnostic),\n    /// Original RDF diagnostic and immutable allocation owner.\n    RetainedDiagnostic(crate::RetainedDiagnostic),',1)
s=s.replace('            Self::NativeDiagnostic(message) => match message.kind() {','            Self::RetainedDiagnostic(_) => None,\n            Self::NativeDiagnostic(message) => match message.kind() {',1)
s=s.replace('            Self::Dataset(diagnostic) => Some(&diagnostic.code),','            Self::RetainedDiagnostic(message) => Some(&message.diagnostic().code),\n            Self::Dataset(diagnostic) => Some(&diagnostic.code),',1)
s=s.replace('            Self::NativeDiagnostic(message) => core::fmt::Display::fmt(message, f),','            Self::RetainedDiagnostic(message) => core::fmt::Display::fmt(message.diagnostic(), f),\n            Self::NativeDiagnostic(message) => core::fmt::Display::fmt(message, f),',1)
post[E]=s
post[P]=read(P).replace('.map_err(|failure| failure.into_eval_error(memory.admission_mut().workspace()))?','.map_err(crate::substitute::GroundFailure::into_eval_error)?')
