from pathlib import Path
import difflib
stage=Path('.stage/sparql-eval-complete-bounded-workspace'); changes={}
def load(path):
    old=Path(path).read_text(); changes[path]=[old,old]; return old
def save(path,s): changes[path][1]=s
def clone_wrapper(ty):
    return f'''impl Clone for {ty} {{
    fn clone(&self) -> Self {{
        let mut resident = purrdf_lex::allocation::Resident;
        self.clone_with_memory(&mut purrdf_lex::allocation::Memory::new(&mut resident))
            .expect("resident diagnostic clone allocation")
    }}
}}

'''
path='crates/lex/src/allocation.rs'; s=load(path); at=s.index('    /// Freeze an originally admitted array')
s=s[:at]+'''    /// Admit a single boxed value before fallible physical construction.
    /// Its existing payload already belongs to this memory or an independent owner.
    ///
    /// # Errors
    /// Returns original admission, layout or allocator refusal.
    pub fn boxed<T>(&mut self, value: T) -> Result<Box<T>, StorageError> {
        let bytes = Layout::new::<T>().size();
        self.add_bytes(bytes)?;
        match try_boxed(value) {
            Ok(value) => Ok(value),
            Err(_) => {
                self.release_bytes(bytes)?;
                Err(StorageError::AllocationFailed)
            }
        }
    }

'''+s[at:]; save(path,s)
path='crates/lex/src/diagnostic.rs'; s=load(path)
for ty in ['DiagnosticValue','DiagnosticParameter','DiagnosticPresentation']:
    target=('pub enum ' if ty=='DiagnosticValue' else 'pub struct ')+ty
    a=s.rfind('#[derive(',0,s.index(target)); b=s.index(')]',a)
    s=s[:a]+s[a:b].replace('Clone, ','')+s[b:]
    at=s.index('impl '+ty+' {')
    wrapper=clone_wrapper(ty).replace('purrdf_lex::allocation','crate::allocation')
    s=s[:at]+wrapper+s[at:]
native='''    /// Copy immutable typed presentation data through original physical admission.
    /// The caller retains this memory with the returned payload.
    ///
    /// # Errors
    /// Returns the first physical storage refusal before publication.
    pub fn clone_with_memory<S: crate::allocation::Admission + ?Sized>(
        &self, memory: &mut crate::allocation::Memory<'_,S>,
    ) -> Result<Self, crate::allocation::StorageError> {
        Ok(match self {
            Self::Text(value) => Self::Text(memory.string(value)?),
            Self::Unsigned(value) => Self::Unsigned(*value),
            Self::Signed(value) => Self::Signed(*value),
            Self::Boolean(value) => Self::Boolean(*value),
            Self::Character(value) => Self::Character(*value),
        })
    }

'''
s=s.replace('impl DiagnosticValue {\n','impl DiagnosticValue {\n'+native,1)
native='''    /// Copy the original name and typed value after each allocation is admitted.
    ///
    /// # Errors
    /// Returns physical refusal, retaining already admitted scratch in the caller.
    pub fn clone_with_memory<S: crate::allocation::Admission + ?Sized>(
        &self, memory: &mut crate::allocation::Memory<'_,S>,
    ) -> Result<Self, crate::allocation::StorageError> {
        Ok(Self { name: memory.string(&self.name)?, value: self.value.clone_with_memory(memory)? })
    }

'''
s=s.replace('impl DiagnosticParameter {\n','impl DiagnosticParameter {\n'+native,1)
native='''    /// Copy the validated identity, typed arguments and verbatim English.
    /// Secondary detail is bounded by the existing construction invariant.
    ///
    /// # Errors
    /// Returns original physical refusal before publishing any replacement.
    pub fn clone_with_memory<S: crate::allocation::Admission + ?Sized>(
        &self, memory: &mut crate::allocation::Memory<'_,S>,
    ) -> Result<Self, crate::allocation::StorageError> {
        let mut parameters = Vec::new();
        memory.reserve(&mut parameters,self.parameters.len())?;
        for parameter in &self.parameters { parameters.push(parameter.clone_with_memory(memory)?); }
        Ok(Self {
            message_id: memory.string(&self.message_id)?,
            parameters,
            english: memory.string(&self.english)?,
            detail: self.detail.as_deref().map(|detail| {
                let detail = detail.clone_with_memory(memory)?;
                memory.boxed(detail)
            }).transpose()?,
        })
    }

'''
s=s.replace('impl DiagnosticPresentation {\n','impl DiagnosticPresentation {\n'+native,1)
# Use the one box factory for the existing admitted detail constructor too.
old='''        memory.add_bytes(size_of::<Self>())?;
        self.detail = Some(
            crate::allocation::try_boxed(detail)
                .map_err(|_| crate::allocation::StorageError::AllocationFailed)?,
        );'''
assert old in s
s=s.replace(old,'        self.detail = Some(memory.boxed(detail)?);',1); save(path,s)
path='crates/iri/src/base.rs'; s=load(path); target='pub enum BaseInScope'; a=s.rfind('#[derive(',0,s.index(target)); b=s.index(')]',a); s=s[:a]+s[a:b].replace('Clone, ','')+s[b:]
at=s.index('impl BaseInScope {'); s=s[:at]+clone_wrapper('BaseInScope')+s[at:]
native='''    /// Preserve the exact base and provenance through before-copy admission.
    ///
    /// # Errors
    /// Returns the original physical refusal.
    pub fn clone_with_memory<S: Admission + ?Sized>(&self, memory: &mut Memory<'_,S>)
        -> core::result::Result<Self,purrdf_lex::allocation::StorageError> {
        Ok(match self {
            Self::Absent => Self::Absent,
            Self::InForce { iri, origin } => Self::InForce { iri: memory.string(iri)?, origin: *origin },
        })
    }

'''
s=s.replace('impl BaseInScope {\n','impl BaseInScope {\n'+native,1); save(path,s)
path='crates/iri/src/error.rs'; s=load(path); s=s.replace('#[derive(Clone, PartialEq, Eq)]','#[derive(PartialEq, Eq)]',1); at=s.index('impl IriError {'); s=s[:at]+clone_wrapper('IriError')+s[at:]
native='''    /// Copy every original typed IRI failure field under before-copy admission.
    /// Text stays verbatim; no parse or diagnostic re-rendering changes identity.
    ///
    /// # Errors
    /// Returns physical storage refusal before the copied error is published.
    pub fn clone_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
        &self, memory: &mut purrdf_lex::allocation::Memory<'_,S>,
    ) -> core::result::Result<Self,purrdf_lex::allocation::StorageError> {
        Ok(match self {
            Self::Empty => Self::Empty,
            Self::MissingScheme => Self::MissingScheme,
            Self::BadScheme(text) => Self::BadScheme(memory.string(text)?),
            Self::BadPercentEncoding(offset) => Self::BadPercentEncoding(*offset),
            Self::DisallowedChar(character,offset) => Self::DisallowedChar(*character,*offset),
            Self::BadAuthority(text) => Self::BadAuthority(memory.string(text)?),
            Self::NonAbsoluteBase(text) => Self::NonAbsoluteBase(memory.string(text)?),
            Self::NoBase { reference } => Self::NoBase { reference: memory.string(reference)? },
            Self::NotAbsoluteByGrammar { reference,base } => Self::NotAbsoluteByGrammar {
                reference: memory.string(reference)?, base: base.clone_with_memory(memory)?,
            },
        })
    }

'''
s=s.replace('impl IriError {\n','impl IriError {\n'+native,1); save(path,s)
path='crates/rdf-core/src/diagnostic.rs'; s=load(path)
for ty in ['RdfLocation','RdfDiagnostic']:
    target='pub struct '+ty; a=s.rfind('#[derive(',0,s.index(target)); b=s.index(')]',a); s=s[:a]+s[a:b].replace('Clone, ','')+s[b:]
    at=s.index('impl '+ty+' {'); s=s[:at]+clone_wrapper(ty)+s[at:]
native='''    /// Copy physical and logical locations without allocating ahead of admission.
    ///
    /// # Errors
    /// Returns the original physical refusal, preserving the source location.
    pub fn clone_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
        &self, memory: &mut purrdf_lex::allocation::Memory<'_,S>,
    ) -> Result<Self,purrdf_lex::allocation::StorageError> {
        Ok(Self {
            path: self.path.as_deref().map(|text| memory.string(text)).transpose()?,
            line: self.line, column: self.column,
            logical: self.logical.as_deref().map(|text| memory.string(text)).transpose()?,
            subject: self.subject.as_deref().map(|text| memory.string(text)).transpose()?,
            gts_term_id: self.gts_term_id, gts_quad_index: self.gts_quad_index,
            gts_reifier_id: self.gts_reifier_id, gts_frame_index: self.gts_frame_index,
            gts_segment_index: self.gts_segment_index,
        })
    }

'''
s=s.replace('impl RdfLocation {\n','impl RdfLocation {\n'+native,1)
native='''    /// Copy original typed diagnostic data through its actual allocation owner.
    /// All boxes and text are admitted before physical allocation or copying.
    ///
    /// # Errors
    /// Returns the first physical refusal before the replacement is published.
    pub fn clone_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
        &self, memory: &mut purrdf_lex::allocation::Memory<'_,S>,
    ) -> Result<Self,purrdf_lex::allocation::StorageError> {
        Ok(Self {
            severity: self.severity, code: memory.string(&self.code)?, message: memory.string(&self.message)?,
            detail: self.detail.as_deref().map(|text| memory.string(text)).transpose()?,
            location: self.location.as_deref().map(|location| {
                let location = location.clone_with_memory(memory)?; memory.boxed(location)
            }).transpose()?,
            presentation: self.presentation.as_deref().map(|presentation| {
                let presentation = presentation.clone_with_memory(memory)?; memory.boxed(presentation)
            }).transpose()?,
        })
    }

'''
s=s.replace('impl RdfDiagnostic {\n','impl RdfDiagnostic {\n'+native,1); save(path,s)
out=stage/'native-diagnostic-clone-postimages'; out.mkdir(exist_ok=True)
patch=''
for path,(old,new) in changes.items():
    target=out/path; target.parent.mkdir(parents=True,exist_ok=True); target.write_text(new)
    patch+=''.join(difflib.unified_diff(old.splitlines(True),new.splitlines(True),fromfile='a/'+path,tofile='b/'+path))
(stage/'native-diagnostic-clone-owner-draft.patch').write_text(patch)
