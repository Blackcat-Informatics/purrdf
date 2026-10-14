# Issue #365: Post-v3.0: asynchronous Wasm lane and parse/serialize follow-ups from #358

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

Follow-ups from finalizing the asynchronous Wasm SERVICE/LOAD work for 3.0. None of these is a breaking change or a wrong answer; each was triaged as safe to ship in 3.0 and fixable in a 3.x release.

## Asynchronous lane and the JavaScript package

- **A hung handler can outlive its deadline in Node.** The runtime arms a job's deadline with `AbortSignal.timeout`, which Node does not count as keeping the event loop alive. A `resolveService`/`resolveLoad` handler that never settles and does no I/O, with `deadlineMs` set, lets the process exit ("Detected unsettled top-level await") instead of the job rejecting with its deadline. With anything else keeping the loop alive, the deadline trips as documented. Arm the deadline so it keeps the loop alive while a job waits on a host effect.
- **Poison errors carry no `error.code`.** Once the instance is poisoned, the error a caller receives ("the wasm instance trapped … cannot be used again") has no `code`, although every other error the package throws carries its stable code as `error.code`. Give it one.
- **The two lanes' stack depth and refusal wording differ slightly.** Both lanes refuse a too-deep request with `native-sparql-evaluation-stack-exhausted`, but each spends a little stack on its own frames, so the deepest request each evaluates can differ by a few levels, and the construct the message names is whichever frame found the stack low. Measure both lanes' budget from the shared `Operation::execute` entry (and name the construct position-independently) so the two lanes agree exactly; bench the synchronous path before moving it onto governed dispatch.
- **Deep nesting under the host-stack budget is reported as a 500.** Some deeply nested shapes (OPTIONAL, EXISTS, GRAPH, MINUS, LATERAL, SERVICE at 10⁴–10⁵ levels) pass admission and are refused during evaluation with `native-sparql-evaluation-stack-exhausted`, which the Cloudflare adapter answers with 500, while the docs describe nesting past the host-stack budget as refused before evaluation with `native-sparql-host-stack-exhausted` (400). Either admit those shapes against the budget before evaluation, or map the evaluation-stack refusal to 400 and say so.
- **One JavaScript test takes about 12 minutes.** `async-concurrency.test.mjs`'s "a request as deep as the synchronous lane evaluates … frames that never poll included" runs a deep tree with `yieldEveryPolls: 0`, about 673 000 polls, each a `setTimeout(0)` turn that Node clamps to at least 1 ms. Keep the proof and cut its cost (a smaller tree that still exceeds a frame that never polls, or a larger quantum).

## Parsing and serialization cost

- **`SELECT *` over many nested sub-SELECTs that each bind a new variable parses in quadratic time and memory.** Each `SELECT *` lists every variable visible below it, so 8 000 such levels take about 26 s and 547 MiB natively. The request is refused with a typed error on wasm, but the work before the refusal grows with the square of the depth. Represent a `SELECT *` projection without materializing each level's variable list, or admit the projection size before building it.
- **The GTS writer's identity keys cost time proportional to the term count times the nesting depth**, from its per-term expansion and linear path check; a triple-term chain thousands deep writes slowly. Memoize each term's identity key as the segment union now memoizes mapped terms.


## Comments (1)

### paudley — 2026-09-28T13:52:54Z

## A deeply nested triple term in a SERVICE answer crashes the Wasm instance when JS reads it

**What happens.** A `SERVICE` endpoint answers with a triple term nested about 20 000 levels deep, which is valid SPARQL Results JSON. The JSON reader decodes it and the evaluator joins it without trouble. When JS then reads the value (`rows.toArray()`, then walking `.object`), the instance traps with `Maximum call stack size exceeded` and is poisoned. Every later call fails with `unreachable`. On the async lane this happens even under `SILENT`, because the trap occurs after the query has answered. Natively the engine handles the same answer on a 1 MiB stack.

**Cause.** The Wasm `Term` stores a quoted triple as `TermInner::Quoted(Box<RdfTriple>)`, which is the public owned model. That model's `Clone`, `PartialEq`, `Hash` and `Drop` are derived, so each recurses once per level. The `subject`/`object` getters clone the remaining chain, and dropping the term recurses the same way. The engine's `TermValue` already does all of these over a work list (`TermBox`).

**Fix (written, not merged).** The patch below makes `TermInner::Quoted` hold a `TermValue::Triple`. SELECT cells go from `TermValue` to a `Term` without passing through `RdfTerm`. The getters return a component, cloned iteratively. `Term::quoted` builds the value directly for `quotedTriple`/`asTerm`. Non-triple cells still go through `convert::term_value_into_rdf_term`, so that SIMD-audit site stays live. A nested non-IRI predicate is still refused, and nested blank nodes take the default scope, as they did through the owned model. The change stays inside `purrdf-wasm` and changes no public API.

**Verified so far:** `cargo test -p purrdf-wasm --lib` passes, 219 tests, including two new ones:
- a 100 000-level cell is marshalled, walked, cloned, compared and dropped on a 128 KiB thread;
- a nested literal predicate is refused, and its IRI neighbour is a term equal to its owned-model round trip.

**Not yet run:** clippy, `make wasm-pkg-test`, and the Node reproduction on a rebuilt package. Owned model (`RdfTerm`/`RdfTriple` derived recursion) is out of scope for this patch; changing it is a public-API decision.

Node reproduction (against a built `crates/rdf-wasm/js` package):

```js
import { Dataset, QueryEngine, ready } from "./index.mjs";
await ready();
const EX = "http://example.org/", depth = 20000;
const iri = `{"type":"uri","value":"${EX}a"}`;
const open = `{"type":"triple","value":{"subject":${iri},"predicate":${iri},"object":`;
const body = `{"head":{"vars":["o","x"]},"results":{"bindings":[{"o":{"type":"uri","value":"${EX}o1"},"x":${open.repeat(depth)}${iri}${"}}".repeat(depth)}}]}}`;
const engine = new QueryEngine();
const r = await engine.queryAsync(Dataset.parse(`<${EX}a> <${EX}p> <${EX}o1> .\n`, "nquads"),
  `SELECT ?x WHERE { ?s <${EX}p> ?o . SERVICE <${EX}sparql> { ?o <${EX}q> ?x } }`,
  { resolveService: async () => body });
let t = r.rows.toArray()[0].x; while (t.termType === "Quad") t = t.object; // traps before the patch
```

Patch (against f7133ea0; `git apply`):

```diff
diff --git crates/rdf-wasm/src/convert.rs crates/rdf-wasm/src/convert.rs
index 89dc9917..1d8ba831 100644
--- crates/rdf-wasm/src/convert.rs
+++ crates/rdf-wasm/src/convert.rs
@@ -58,7 +58,7 @@ pub(crate) fn rdf_term_to_term_value(term: &RdfTerm) -> TermValue {
 }
 
 /// The owned [`RdfTerm`] of a term value that is not a triple term.
-fn leaf_rdf_term(value: TermValue) -> RdfTerm {
+pub(crate) fn leaf_rdf_term(value: TermValue) -> RdfTerm {
     match value {
         TermValue::Iri(iri) => RdfTerm::Iri(iri),
         TermValue::Blank { label, .. } => RdfTerm::BlankNode(label),
diff --git crates/rdf-wasm/src/factory.rs crates/rdf-wasm/src/factory.rs
index 8a851c6e..da62884d 100644
--- crates/rdf-wasm/src/factory.rs
+++ crates/rdf-wasm/src/factory.rs
@@ -11,7 +11,7 @@
 
 use core::cell::Cell;
 
-use purrdf::{RdfLiteral, RdfTriple};
+use purrdf::RdfLiteral;
 use wasm_bindgen::prelude::*;
 
 use crate::term::{Quad, Term, TermInner, parse_direction};
@@ -213,12 +213,7 @@ impl DataFactory {
                 ));
             }
         };
-        let triple = RdfTriple::new(
-            subject.to_rdf_term().map_err(|e| JsError::new(&e))?,
-            predicate_iri,
-            object.to_rdf_term().map_err(|e| JsError::new(&e))?,
-        );
-        Ok(Term::from_inner(TermInner::Quoted(Box::new(triple))))
+        Term::quoted(subject, predicate_iri, object).map_err(|e| JsError::new(&e))
     }
 
     /// `fromTerm(original)` → a copy of `original` (RDF/JS structural clone).
diff --git crates/rdf-wasm/src/query.rs crates/rdf-wasm/src/query.rs
index 77faeab8..880bc0fd 100644
--- crates/rdf-wasm/src/query.rs
+++ crates/rdf-wasm/src/query.rs
@@ -85,7 +85,6 @@ use wasm_bindgen::prelude::*;
 
 use crate::async_query::AsyncOperationKind;
 use crate::codec::resolve_format;
-use crate::convert::term_value_into_rdf_term;
 use crate::dataset::{Dataset, serialize_frozen_with_options};
 use crate::jsonld::{CompiledJsonLdContext, context_options, decode_options};
 use crate::operation::{
@@ -2060,8 +2059,7 @@ fn select_row(
 }
 
 fn term_from_value(value: purrdf::TermValue) -> Result<Term, String> {
-    let term = term_value_into_rdf_term(value)?;
-    Ok(Term::from_canonical_rdf_term(term))
+    Term::from_value(value)
 }
 
 /// Decode `provenance_prefix`/`provenance_iri` (both `None`, or both `Some`) into an
diff --git crates/rdf-wasm/src/stream.rs crates/rdf-wasm/src/stream.rs
index 5e0ce48a..7990a82c 100644
--- crates/rdf-wasm/src/stream.rs
+++ crates/rdf-wasm/src/stream.rs
@@ -286,7 +286,7 @@ mod tests {
             "https://e/p",
             RdfTerm::iri("https://e/o"),
         );
-        let quoted = Term::from_inner(TermInner::Quoted(Box::new(inner)));
+        let quoted = Term::quoted_triple(inner);
         let q = Quad::from_parts(
             named("https://e/stmt"),
             named("https://e/asserts"),
diff --git crates/rdf-wasm/src/term.rs crates/rdf-wasm/src/term.rs
index cb8a9f05..e9f7237e 100644
--- crates/rdf-wasm/src/term.rs
+++ crates/rdf-wasm/src/term.rs
@@ -14,7 +14,13 @@
 //! carry the RDF-1.2 base `direction` — neither of which any incumbent RDF/JS library
 //! models.
 
-use purrdf::{RdfLiteral, RdfTerm, RdfTextDirection, RdfTriple};
+#[cfg(test)]
+use purrdf::RdfTriple;
+use purrdf::{BlankScope, RdfLiteral, RdfTerm, RdfTextDirection, TermBox, TermValue};
+
+use crate::convert::{
+    leaf_rdf_term, rdf_term_to_term_value, term_value_into_rdf_term, term_value_to_rdf_term,
+};
 use wasm_bindgen::prelude::*;
 
 /// `xsd:string` — the datatype of a plain literal (RDF 1.1 §3.3).
@@ -37,8 +43,12 @@ pub(crate) enum TermInner {
     Blank(String),
     /// `termType: "Literal"`.
     Literal(RdfLiteral),
-    /// `termType: "Quad"` used as a term — an RDF-1.2 quoted triple.
-    Quoted(Box<RdfTriple>),
+    /// `termType: "Quad"` used as a term — an RDF-1.2 quoted triple, held as the engine's
+    /// [`TermValue::Triple`] (always that variant, every predicate an IRI, every blank node
+    /// in the default scope). Its clone, equality, `Debug` and drop walk a work list, so a
+    /// triple term however deeply nested — a remote `SERVICE` answer's, say — crosses the
+    /// wasm boundary without a stack frame per level.
+    Quoted(TermValue),
     /// `termType: "Variable"` — a SPARQL variable name (without the `?`).
     Variable(String),
     /// `termType: "DefaultGraph"`.
@@ -82,7 +92,7 @@ impl Term {
             RdfTerm::Iri(iri) => Self::from_inner(TermInner::Named(iri.clone())),
             RdfTerm::BlankNode(label) => Self::from_inner(TermInner::Blank(label.clone())),
             RdfTerm::Literal(lit) => Self::literal(lit.clone()),
-            RdfTerm::Triple(triple) => Self::from_inner(TermInner::Quoted(triple.clone())),
+            RdfTerm::Triple(_) => Self::from_inner(TermInner::Quoted(rdf_term_to_term_value(t))),
         }
     }
 
@@ -93,7 +103,67 @@ impl Term {
             RdfTerm::Iri(iri) => Self::from_inner(TermInner::Named(iri)),
             RdfTerm::BlankNode(label) => Self::from_inner(TermInner::Blank(label)),
             RdfTerm::Literal(lit) => Self::from_inner(TermInner::Literal(lit)),
-            RdfTerm::Triple(triple) => Self::from_inner(TermInner::Quoted(triple)),
+            RdfTerm::Triple(_) => Self::from_inner(TermInner::Quoted(rdf_term_to_term_value(&t))),
+        }
+    }
+
+    /// A quoted-triple [`Term`] over `(subject, predicate, object)`.
+    pub(crate) fn quoted(
+        subject: &Self,
+        predicate: impl Into<String>,
+        object: &Self,
+    ) -> Result<Self, String> {
+        Ok(Self::from_inner(TermInner::Quoted(TermValue::Triple {
+            s: TermBox::new(subject.to_value()?),
+            p: TermBox::new(TermValue::Iri(predicate.into())),
+            o: TermBox::new(object.to_value()?),
+        })))
+    }
+
+    /// A quoted-triple [`Term`] over an owned engine triple.
+    #[cfg(test)]
+    pub(crate) fn quoted_triple(triple: RdfTriple) -> Self {
+        Self::from_rdf_term(&RdfTerm::Triple(Box::new(triple)))
+    }
+
+    /// Build a [`Term`] from an engine value, as a SELECT cell arrives. A triple term stays
+    /// a value (see [`TermInner::Quoted`]); every nested predicate must be an IRI, and every
+    /// nested blank node takes the default scope, as an owned term's blank node does.
+    pub(crate) fn from_value(mut value: TermValue) -> Result<Self, String> {
+        if !matches!(value, TermValue::Triple { .. }) {
+            return term_value_into_rdf_term(value).map(Self::from_canonical_rdf_term);
+        }
+        let mut work: Vec<&mut TermValue> = vec![&mut value];
+        while let Some(node) = work.pop() {
+            match node {
+                TermValue::Triple { s, p, o } => {
+                    if !matches!(**p, TermValue::Iri(_)) {
+                        return Err("a triple-term predicate must be an IRI".to_owned());
+                    }
+                    work.push(&mut **s);
+                    work.push(&mut **o);
+                }
+                TermValue::Blank { scope, .. } => *scope = BlankScope::DEFAULT,
+                TermValue::Iri(_) | TermValue::Literal { .. } => {}
+            }
+        }
+        Ok(Self::from_inner(TermInner::Quoted(value)))
+    }
+
+    /// The [`Term`] of one component of a held triple term.
+    fn from_component(component: &TermValue) -> Self {
+        match component {
+            TermValue::Triple { .. } => Self::from_inner(TermInner::Quoted(component.clone())),
+            leaf => Self::from_canonical_rdf_term(leaf_rdf_term(leaf.clone())),
+        }
+    }
+
+    /// Lower this term to the engine's dataset-independent [`TermValue`]. Errors as
+    /// [`Self::to_rdf_term`] does.
+    pub(crate) fn to_value(&self) -> Result<TermValue, String> {
+        match &self.inner {
+            TermInner::Quoted(value) => Ok(value.clone()),
+            _ => self.to_rdf_term().map(|term| rdf_term_to_term_value(&term)),
         }
     }
 
@@ -106,7 +176,7 @@ impl Term {
             TermInner::Named(iri) => Ok(RdfTerm::Iri(iri.clone())),
             TermInner::Blank(label) => Ok(RdfTerm::BlankNode(label.clone())),
             TermInner::Literal(lit) => Ok(RdfTerm::Literal(lit.clone())),
-            TermInner::Quoted(triple) => Ok(RdfTerm::Triple(triple.clone())),
+            TermInner::Quoted(value) => term_value_to_rdf_term(value),
             TermInner::Variable(_) => Err("a Variable is not a valid RDF term".to_owned()),
             TermInner::DefaultGraph => {
                 Err("the DefaultGraph is not a valid subject/predicate/object".to_owned())
@@ -186,7 +256,7 @@ impl Term {
     #[wasm_bindgen(getter)]
     pub fn subject(&self) -> Option<Self> {
         match &self.inner {
-            TermInner::Quoted(t) => Some(Self::from_rdf_term(&t.subject)),
+            TermInner::Quoted(TermValue::Triple { s, .. }) => Some(Self::from_component(s)),
             _ => None,
         }
     }
@@ -195,7 +265,7 @@ impl Term {
     #[wasm_bindgen(getter)]
     pub fn predicate(&self) -> Option<Self> {
         match &self.inner {
-            TermInner::Quoted(t) => Some(Self::from_inner(TermInner::Named(t.predicate.clone()))),
+            TermInner::Quoted(TermValue::Triple { p, .. }) => Some(Self::from_component(p)),
             _ => None,
         }
     }
@@ -204,7 +274,7 @@ impl Term {
     #[wasm_bindgen(getter)]
     pub fn object(&self) -> Option<Self> {
         match &self.inner {
-            TermInner::Quoted(t) => Some(Self::from_rdf_term(&t.object)),
+            TermInner::Quoted(TermValue::Triple { o, .. }) => Some(Self::from_component(o)),
             _ => None,
         }
     }
@@ -289,15 +359,10 @@ impl Quad {
     /// the RDF-1.2 wedge: pass the result as a subject/object to embed it.
     #[wasm_bindgen(js_name = asTerm)]
     pub fn as_term(&self) -> Result<Term, JsError> {
-        let triple = RdfTriple::new(
-            self.subject.to_rdf_term().map_err(|e| JsError::new(&e))?,
-            match &self.predicate.inner {
-                TermInner::Named(iri) => iri.clone(),
-                _ => return Err(JsError::new("a quad predicate must be a NamedNode")),
-            },
-            self.object.to_rdf_term().map_err(|e| JsError::new(&e))?,
-        );
-        Ok(Term::from_inner(TermInner::Quoted(Box::new(triple))))
+        let TermInner::Named(predicate) = &self.predicate.inner else {
+            return Err(JsError::new("a quad predicate must be a NamedNode"));
+        };
+        Term::quoted(&self.subject, predicate.clone(), &self.object).map_err(|e| JsError::new(&e))
     }
 
     /// Structural RDF/JS quad equality.
@@ -456,6 +521,78 @@ mod tests {
         assert_eq!(t.datatype().unwrap().value(), RDF_LANG_STRING);
     }
 
+    /// A SELECT cell holding a triple term a hundred thousand levels deep — what a hostile
+    /// `SERVICE` endpoint can answer — becomes a JS term, is walked through its `object`
+    /// getters, cloned, compared and dropped on a thread whose whole stack is 128 KiB.
+    #[test]
+    fn a_hundred_thousand_level_cell_crosses_the_boundary_on_a_128_kib_thread() {
+        const LEVELS: usize = 100_000;
+        std::thread::Builder::new()
+            .stack_size(128 * 1024)
+            .spawn(|| {
+                let term = Term::from_value(purrdf_core::test_rng::triple_chain(LEVELS))
+                    .expect("a chain of IRI predicates is a term");
+                let copy = term.clone();
+                assert!(term.equals(&copy));
+                let mut levels = 0;
+                let mut at = term.object().expect("a quoted triple has an object");
+                levels += 1;
+                while at.term_type() == "Quad" {
+                    // Each step drops the previous level's whole remaining chain.
+                    at = at.object().expect("a quoted triple has an object");
+                    levels += 1;
+                    if levels == 3 {
+                        break;
+                    }
+                }
+                assert_eq!(levels, 3);
+                assert_eq!(
+                    term.predicate().expect("predicate").value(),
+                    "http://example.org/p"
+                );
+                drop((term, copy, at));
+            })
+            .expect("spawn")
+            .join()
+            .expect("no stack overflow");
+    }
+
+    /// A cell whose nested triple term has a literal predicate is refused; its neighbour
+    /// with an IRI predicate at the same depth, and its blank nodes, are a term.
+    #[test]
+    fn a_cell_with_a_non_iri_nested_predicate_is_refused_and_its_neighbour_is_a_term() {
+        let nested = |predicate: TermValue| TermValue::Triple {
+            s: TermBox::new(TermValue::Blank {
+                label: "b".to_owned(),
+                scope: BlankScope::DEFAULT,
+            }),
+            p: TermBox::new(TermValue::iri("http://example.org/p")),
+            o: TermBox::new(TermValue::Triple {
+                s: TermBox::new(TermValue::iri("http://example.org/s")),
+                p: TermBox::new(predicate),
+                o: TermBox::new(TermValue::iri("http://example.org/o")),
+            }),
+        };
+        let literal = TermValue::Literal {
+            lexical_form: "p".to_owned(),
+            datatype: XSD_STRING.to_owned(),
+            language: None,
+            direction: None,
+        };
+        assert!(Term::from_value(nested(literal)).is_err());
+        let term = Term::from_value(nested(TermValue::iri("http://example.org/q")))
+            .expect("IRI predicates at every level");
+        assert_eq!(term.subject().expect("subject").term_type(), "BlankNode");
+        let inner = term.object().expect("object");
+        assert_eq!(
+            inner.predicate().expect("predicate").value(),
+            "http://example.org/q"
+        );
+        // The same term through the owned model is equal to it.
+        let owned = Term::from_rdf_term(&term.to_rdf_term().expect("an RDF term"));
+        assert!(term.equals(&owned));
+    }
+
     #[test]
     fn quoted_triple_term_exposes_components() {
         let triple = RdfTriple::new(
@@ -463,7 +600,7 @@ mod tests {
             "https://e/p",
             RdfTerm::iri("https://e/o"),
         );
-        let t = Term::from_inner(TermInner::Quoted(Box::new(triple)));
+        let t = Term::quoted_triple(triple);
         assert_eq!(t.term_type(), "Quad");
         assert_eq!(t.value(), "");
         assert_eq!(t.subject().unwrap().value(), "https://e/s");
```


