from pathlib import Path
stage=Path(__file__).parent;root=stage.parent.parent;out=stage/'remote-native-owner-postimages'
def edit(path,fn):
    p=out/path;p.write_text(fn(p.read_text()))
def remote(s):
    idx=s.index('#[cfg(test)]\nmod tests')
    s=s[:idx]+(stage/'remote-fixture-adapters.rs').read_text()+'\n'+s[idx:]
    s=s.replace('for row in &seq.rows {','for row in seq.rows.iter() {')
    s=s.replace('AdmittedRemoteError::message(purrdf_core::SilencedKind::Decode, error.diagnostic(), workspace)','AdmittedRemoteError::message(purrdf_core::SilencedKind::Decode, &error.diagnostic().message, workspace)')
    s=s.replace('return Err(EvalError::StackExhausted { construct }),','return Err(EvalError::StackExhausted { construct: *construct }),')
    s=s.replace('return Err(EvalError::HostStackExhausted { construct }),','return Err(EvalError::HostStackExhausted { construct: *construct }),')
    return s
edit('crates/sparql-eval/src/remote.rs',remote)
def serializer(s):
    s=s.replace('return Err(CarrierError::Parse(error.into()))','return Err(CarrierError::Parse(crate::ParseError::Syntax {reason:memory.format(&error)?,at:0}))')
    return s
edit('crates/sparql-algebra/src/serialize.rs',serializer)
def workspace(s):
    fixtures=(stage/'remote-native-owner-fixtures.rs').read_text()
    # libtest has no counting global allocator. The exact original grant checks
    # here are paired with the production integration test below, which has one.
    fixtures=fixtures.replace('        let probe=purrdf_alloc_probe::thread_window();\n','').replace('        let measured=probe.finish();\n        assert_eq!(measured.allocations,0);\n','')
    fixtures=fixtures.replace('let dataset=Arc::new(purrdf_core::RdfDataset::default());','let dataset=purrdf_core::RdfDatasetBuilder::new().freeze().unwrap();')
    idx=s.rfind('\n}')
    return s[:idx]+'\n'+fixtures+s[idx:]
edit('crates/sparql-eval/src/workspace.rs',workspace)
