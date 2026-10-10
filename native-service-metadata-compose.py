from pathlib import Path
import difflib,re
root=Path(__file__).resolve().parents[2];stage=Path(__file__).resolve().parent
images={}
def read(p):
    s=(root/p).read_text(); images[p]=[s,s];return s
def save(p,s): images[p][1]=s
p='crates/lex/src/allocation/shared.rs';s=read(p)
s=s.replace('pub struct SharedText(SharedOwned<String>);', '''pub struct SharedText(TextStorage);
#[derive(Clone)]
enum TextStorage {
    Owned(SharedOwned<String>),
    Static(&'static str),
}''')
s=s.replace('impl SharedText {', '''impl SharedText {
    /// Borrow immutable metadata known before an invocation, without allocating.
    #[must_use]
    pub const fn from_static(text: &'static str) -> Self { Self(TextStorage::Static(text)) }
''',1)
s=s.replace('SharedOwned::try_from_admitted(text, owner).map(Self)', 'SharedOwned::try_from_admitted(text, owner).map(|value| Self(TextStorage::Owned(value)))')
s=s.replace('        self.0.as_str()', '        match &self.0 { TextStorage::Owned(text) => text.as_str(), TextStorage::Static(text) => text }')
s=s.replace('        self.0.try_clone().map(Self)', '        match &self.0 { TextStorage::Owned(text) => text.try_clone().map(|text| Self(TextStorage::Owned(text))), TextStorage::Static(text) => Ok(Self::from_static(text)) }')
at=s.index('impl Eq for SharedText {}')+len('impl Eq for SharedText {}')
s=s[:at]+'''
impl PartialOrd for SharedText {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> { Some(self.cmp(other)) }
}
impl Ord for SharedText {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering { self.as_str().cmp(other.as_str()) }
}
'''+s[at:]
save(p,s)
p='crates/sparql-eval/src/property_fn.rs';s=read(p)
s=s.replace('        reason: String,\n    },\n}\n\n/// The pair of facts', '        reason: purrdf_lex::allocation::SharedText,\n    },\n}\n\n/// The pair of facts')
at=s.index('/// The pair of facts one invocation attests')
s=s[:at]+'''impl ServiceLevel {
    /// Register caller-owned service metadata before opening a cursor.
    #[must_use]
    pub fn incomplete(reason: impl AsRef<str>) -> Self {
        Self::incomplete_admitted(reason.as_ref(), &crate::WorkspaceCapability::resident())
            .expect("resident service metadata allocation failed")
    }

    /// Immutable pre-existing metadata needs no new allocation per invocation.
    #[must_use]
    pub const fn incomplete_static(reason: &'static str) -> Self {
        Self::Incomplete { reason: purrdf_lex::allocation::SharedText::from_static(reason) }
    }

    /// Author a newly discovered service reason under the invocation's original grant.
    /// Clones of this attestation retain the exact original text owner.
    /// # Errors
    /// Returns original admission, layout or allocator refusal before publication.
    pub fn incomplete_admitted(reason: &str, workspace: &crate::WorkspaceCapability) -> Result<Self, EvalError> {
        Ok(Self::Incomplete { reason: workspace.authored_text(reason)? })
    }
}

'''+s[at:]
old='''    fn service_level(&self) -> ServiceLevel {
        ServiceLevel::Undeclared
    }
}'''
new='''    fn service_level(&self) -> ServiceLevel {
        ServiceLevel::Undeclared
    }

    /// Read immutable metadata by sharing its original owner. A provider that
    /// discovers and authors a new reason during this call overrides this method
    /// and uses `ServiceLevel::incomplete_admitted` before creating its text.
    /// # Errors
    /// Returns the provider's original typed failure or native storage refusal.
    fn service_level_with_workspace(&self, _workspace: &crate::WorkspaceCapability) -> Result<ServiceLevel, EvalError> {
        Ok(self.service_level())
    }
}'''
assert old in s;s=s.replace(old,new,1)
s=s.replace('''    fn service_level(&self) -> ServiceLevel {
        self.cursor.service_level()
    }
}''','''    fn service_level(&self) -> ServiceLevel {
        self.cursor.service_level()
    }
    fn service_level_with_workspace(&self, workspace: &crate::WorkspaceCapability) -> Result<ServiceLevel, EvalError> {
        self.cursor.service_level_with_workspace(workspace)
    }
}''',1)
s=s.replace('declaration_contained_admitted(iri, "service level", || cursor.service_level(), workspace)', 'declaration_contained_admitted(iri, "service level", || cursor.service_level_with_workspace(workspace), workspace)?')
save(p,s)
p='crates/sparql-eval/src/witness.rs';s=read(p)
s=s.replace('pub incompleteness: WitnessSet<String>', 'pub incompleteness: WitnessSet<purrdf_lex::allocation::SharedText>')
s=s.replace('.insert_admitted(reason, workspace, |reason, memory| memory.string(reason))?', '.insert_admitted(reason, workspace, |reason, _| Ok(reason.clone()))?')
s=s.replace('BTreeSet::from(["shard 3 rebuilding".to_owned()])', 'BTreeSet::from([purrdf_lex::allocation::SharedText::from_static("shard 3 rebuilding")])')
save(p,s)
# Convert construction sites; patterns that borrow the shared reason stay intact.
for file in root.joinpath('crates').rglob('*.rs'):
    path=str(file.relative_to(root))
    source=images[path][1] if path in images else file.read_text()
    pat=re.compile(r'ServiceLevel::Incomplete\s*\{\s*reason:\s*([^{}]+?),?\s*\}',re.S)
    def convert(match):
        expression=match.group(1).strip().rstrip(',')
        # The retrieval bridge now already holds a shared original owner.
        if expression=='reason.clone()':return match.group()
        # Literal constants are already caller-owned; no new shared header needed.
        literal=re.fullmatch(r'("(?:[^"\\]|\\.)*")(?:\.to_owned\(\)|\.to_string\(\)|\.into\(\))?',expression)
        if literal:return 'ServiceLevel::incomplete_static('+literal.group(1)+')'
        return 'ServiceLevel::incomplete('+expression+')'
    updated=pat.sub(convert,source)
    if updated!=source:
        if path not in images: images[path]=[source,updated]
        else:images[path][1]=updated
# Static provider fixtures carry a host's frozen reason without per-call birth.
for path in ['crates/sparql-eval/tests/relation_witness.rs','crates/shapes/tests/relation_incompleteness.rs','crates/retrieval/tests/execute_dataset.rs','crates/retrieval/tests/on_demand_receipt.rs']:
    if path in images:
        images[path][1]=images[path][1].replace('ServiceLevel::incomplete(reason.to_owned())','ServiceLevel::incomplete_static(reason)')
# A native discovery fixture verifies both the new virtual door and original reason ownership.
p='crates/sparql-eval/src/workspace.rs';s=images[p][1] if p in images else read(p)
test='''
    #[test]
    fn service_metadata_retains_original_grant_through_shallow_attestation_clone() {
        use crate::property_fn::{PfCursor, PfRow, ServiceLevel};
        struct LateReason;
        impl PfCursor for LateReason {
            fn next(&mut self) -> Result<Option<PfRow>, EvalError> { Ok(None) }
            fn service_level_with_workspace(&self, workspace: &WorkspaceCapability) -> Result<ServiceLevel, EvalError> {
                ServiceLevel::incomplete_admitted("shard three is rebuilding", workspace)
            }
        }
        let (workspace, ledger, base) = account(65_536);
        let capability = workspace.capability();
        let service = crate::property_fn::service_level_contained_admitted(&LateReason, "http://example.org/pf", &capability).unwrap();
        let clone = service.clone();
        let original = ledger.current_bytes();
        drop(service);
        assert_eq!(ledger.current_bytes(), original);
        let ServiceLevel::Incomplete { reason } = &clone else { panic!("incomplete attestation"); };
        assert_eq!(reason.as_str(), "shard three is rebuilding");
        drop(clone);
        assert_eq!(ledger.current_bytes(), base);
    }
'''
# Match the ledger's actual existing method name before writing the fixture.
if 'ledger.current_bytes()' not in s:
    # Existing local fixtures use live_bytes on their operational ledger.
    test=test.replace('ledger.current_bytes()', 'ledger.live.load(Ordering::Relaxed)')
s=s[:s.rfind('\n}')]+test+s[s.rfind('\n}'):];save(p,s)
for path,(_,new) in images.items():
    new=re.sub(r'ServiceLevel::incomplete\(([A-Z_]+)\.to_owned\(\)\)', r'ServiceLevel::incomplete_static(\1)',new)
    if path=='crates/retrieval/tests/multimodal_read_bound.rs':
        new=new.replace('ServiceLevel::incomplete(reason.to_owned())','ServiceLevel::incomplete_static(reason)')
    else:
        new=new.replace('ServiceLevel::incomplete(reason.to_owned())','ServiceLevel::incomplete(reason)')
    images[path][1]=new
patch=''.join(''.join(difflib.unified_diff(old.splitlines(True),new.splitlines(True),fromfile='a/'+path,tofile='b/'+path)) for path,(old,new) in images.items())
(stage/'native-service-metadata-owner-draft.patch').write_text(patch)
for path,(_,new) in images.items(): (stage/(Path(path).stem+'-native-service-postimage.rs')).write_text(new)
