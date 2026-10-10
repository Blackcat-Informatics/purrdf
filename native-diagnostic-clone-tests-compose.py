from pathlib import Path
import difflib
stage=Path('.stage/sparql-eval-complete-bounded-workspace'); source=Path('crates/rdf-core/src/diagnostic.rs'); old=source.read_text()
tests='''
    #[test]
    fn native_diagnostic_copy_preserves_typed_fields_and_exact_original_layouts() {
        use purrdf_lex::allocation::{Admission,Memory,StorageError};
        struct Grant { live: usize, limit: usize }
        impl Admission for Grant {
            fn resize(&mut self, bytes: usize) -> Result<(),StorageError> {
                if bytes > self.limit { return Err(StorageError::AdmissionFailed); }
                self.live = bytes; Ok(())
            }
        }
        let primary = DiagnosticPresentation::new("primary", "bad {value}",
            vec![DiagnosticParameter::new("value",DiagnosticValue::Text("verbatim λ".into()))]).unwrap();
        let detail = DiagnosticPresentation::new("detail", "at {offset}",
            vec![DiagnosticParameter::new("offset",DiagnosticValue::Unsigned((1 << 53) + 1))]).unwrap();
        let mut source = RdfDiagnostic::error("typed-copy", "")
            .with_presentation(primary.with_detail(detail).unwrap());
        source.location = Some(Box::new(RdfLocation {
            path: Some("document.ttl".into()), logical: Some("source stage".into()),
            subject: Some("example subject".into()), line: Some(17), column: Some(9),
            gts_term_id: Some(41), gts_quad_index: Some(42), gts_reifier_id: Some(43),
            gts_frame_index: Some(44), gts_segment_index: Some(45),
        }));
        let mut refused = Grant { live: 0, limit: 0 };
        {
            let mut memory = Memory::new(&mut refused);
            assert!(matches!(source.clone_with_memory(&mut memory),Err(StorageError::AdmissionFailed)));
            assert_eq!(memory.live_bytes(),0,"first copy refuses before payload allocation");
        }
        assert_eq!(refused.live,0);
        let mut admitted = Grant { live: 0, limit: usize::MAX };
        {
            let mut memory = Memory::new(&mut admitted);
            let copy = source.clone_with_memory(&mut memory).unwrap();
            assert_eq!(copy,source,"all typed fields and exact English survive");
            let bytes = copy.retained_bytes().unwrap();
            assert_eq!(memory.live_bytes(),bytes,"the original grant covers concrete copied capacities and boxes");
            drop(copy);
            memory.release_bytes(bytes).unwrap();
            assert_eq!(memory.live_bytes(),0);
        }
        assert_eq!(admitted.live,0);
    }
'''
at=old.index('    #[test]',old.index('mod tests {')); new=old[:at]+tests+'\n'+old[at:]
patch=''.join(difflib.unified_diff(old.splitlines(True),new.splitlines(True),fromfile='a/'+str(source),tofile='b/'+str(source)))
(stage/'native-diagnostic-clone-original-grant-tests.patch').write_text(patch)
(stage/'native-diagnostic-clone-tests-postimage.rs').write_text(new)
