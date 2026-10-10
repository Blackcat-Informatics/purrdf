from pathlib import Path
import difflib
root=Path(__file__).resolve().parents[2];stage=Path(__file__).resolve().parent
p='crates/sparql-results/src/json_read.rs'
# Apply after the producer packet, so the writer can integrate independently.
s=(stage/'json_read-native-results-postimage.rs').read_text()
tests=r'''
    #[test]
    fn native_select_keeps_exact_original_payload_grant_and_refuses_before_birth() {
        #[derive(Default)]
        struct Grant { live: usize, peak: usize, limit: usize }
        impl Admission for Grant {
            fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
                if bytes > self.limit { return Err(StorageError::AdmissionFailed); }
                self.live = bytes;
                self.peak = self.peak.max(bytes);
                Ok(())
            }
        }
        // Results before head, escaped structural/variable keys, repeated
        // columns and nested RDF 1.2 terms exercise original live producers.
        let bytes = br#"{"results":{"bindings":[{"\u0078":{"type":"triple","value":{"subject":{"type":"uri","value":"http://example.org/s"},"predicate":{"type":"uri","value":"http://example.org/p"},"object":{"type":"literal","value":"a\nb","xml:lang":"en","its:dir":"ltr","dir":"r\u0074l"}}}}]},"head":{"vars":["\u0078","x"]}}"#;
        let mut refused = Grant::default();
        assert!(matches!(from_json_with_memory(bytes, None, &mut Memory::new(&mut refused)), Err(ReadError::Storage(StorageError::AdmissionFailed))));
        assert_eq!(refused.live, 0);
        assert_eq!(refused.peak, 0);

        for ceiling in [None, Some(2)] {
            let mut grant = Grant { limit: 1 << 20, ..Grant::default() };
            let mut memory = Memory::new(&mut grant);
            let parsed = from_json_with_memory(bytes, ceiling, &mut memory).expect("native SELECT");
            assert!(!parsed.truncated);
            assert_eq!(parsed.solutions, from_json(bytes).expect("resident parity"));
            assert_eq!(parsed.solutions.rows[0][0], parsed.solutions.rows[0][1]);
            let mut expected = parsed.solutions.variables.capacity() * size_of::<String>()
                + parsed.solutions.variables.iter().map(String::capacity).sum::<usize>()
                + parsed.solutions.rows.capacity() * size_of::<BindingRow>();
            for row in &parsed.solutions.rows {
                expected += row.capacity() * size_of::<Option<TermValue>>();
                for term in row.iter().flatten() {
                    let _ = term.visit_terms(|term| {
                        expected += match term {
                            TermValue::Iri(text) | TermValue::Blank { label: text, .. } => text.capacity(),
                            TermValue::Literal { lexical_form, datatype, language, .. } => lexical_form.capacity() + datatype.capacity() + language.as_ref().map_or(0, String::capacity),
                            TermValue::Triple { .. } => 3 * size_of::<TermValue>(),
                        };
                        std::ops::ControlFlow::<()>::Continue(())
                    });
                }
            }
            assert_eq!(memory.live_bytes(), expected, "only surviving output owns the original grant");
            let mut solutions = parsed.solutions;
            while let Some(mut row) = solutions.rows.pop() {
                while let Some(term) = row.pop() { if let Some(term) = term { release_term(term, &mut memory).unwrap(); } }
                memory.release_vec(row).unwrap();
            }
            memory.release_vec(solutions.rows).unwrap();
            while let Some(variable) = solutions.variables.pop() { memory.release_string(variable).unwrap(); }
            memory.release_vec(solutions.variables).unwrap();
            assert_eq!(memory.live_bytes(), 0);
            drop(memory);
            assert_eq!(grant.live, 0);
            assert!(grant.peak >= expected);
        }
    }

    #[test]
    fn native_select_retains_syntax_priority_and_bounded_suffix_law() {
        let malformed = br#"{"head":{"vars":[false]},"results":{"bindings":[]},"extra":"\udc00"}"#;
        let resident = from_json(malformed).unwrap_err();
        let native = from_json_with_memory(malformed, None, &mut Memory::new(&mut Resident)).unwrap_err();
        assert_eq!(native, ReadError::Lexical(resident));
        let bytes = br#"{"head":{"vars":["x"]},"results":{"bindings":[{"x":{"type":"uri","value":"http://example.org/x"}},false]}}"#;
        let native = from_json_with_memory(bytes, Some(1), &mut Memory::new(&mut Resident)).unwrap();
        assert_eq!(native, from_json_bounded(bytes, 1).unwrap());
        assert!(native.truncated);
        assert_eq!(native.solutions.rows.len(), 1);
    }
'''
new=s[:s.rfind('\n}')]+tests+s[s.rfind('\n}'):]
(stage/'native-json-results-original-grant-tests.patch').write_text(''.join(difflib.unified_diff(s.splitlines(True),new.splitlines(True),fromfile='a/'+p,tofile='b/'+p)))
