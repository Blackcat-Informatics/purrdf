// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//! Structural and allocation receipts; no clocks or timing measurements.
use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_core::{RdfDatasetBuilder,TermValue};
use purrdf_sparql_algebra::{Expression,GraphPattern,Query,SparqlParser};
use purrdf_sparql_eval::{NativeSparqlEngine,PreparedQuery,QueryOptions,QueryGovernors,ExtensionEnv,MemoryRelation,PropertyFunctionRegistry};
#[global_allocator]
static GLOBAL:CountingAllocator=CountingAllocator;
#[unsafe(no_mangle)]
#[inline(never)]
pub fn qualification_parse(text:&str)->Query { SparqlParser::new().parse_query(text).unwrap() }
fn main() {
    println!("application policy size={} align={}", size_of::<purrdf_sparql_algebra::algebra::ApplicationPolicy>(), align_of::<purrdf_sparql_algebra::algebra::ApplicationPolicy>());
    println!("layout graph={} expr={} query={} prepared={} node_ref={}",size_of::<GraphPattern>(),size_of::<Expression>(),size_of::<Query>(),size_of::<PreparedQuery>(),size_of::<purrdf_sparql_algebra::walk::NodeRef>());
    let queries=[
        "SELECT ?s ?o WHERE { ?s <http://example.org/p> ?o }",
        "SELECT ?s ?v WHERE { ?s <http://example.org/p> ?o . ?s <http://example.org/q> ?v FILTER(?o != ?v) }",
        "SELECT ?s ?o ?v WHERE { ?s <http://example.org/p> ?o OPTIONAL { ?s <http://example.org/q> ?v } }",
        "SELECT DISTINCT ?s WHERE { { ?s <http://example.org/p> ?o } UNION { ?s <http://example.org/q> ?v } }",
        "SELECT ?s (COUNT(?o) AS ?n) WHERE { ?s <http://example.org/p> ?o } GROUP BY ?s",
        "SELECT ?s WHERE { ?s <http://example.org/p> ?o MINUS { ?s <http://example.org/q> ?o } FILTER EXISTS { ?s <http://example.org/p> ?v } }",
        "SELECT ?s ?o ?v WHERE { ?s <http://example.org/p> ?o LATERAL { ?s <http://example.org/q> ?v } }",
        "SELECT ?s ?o WHERE { GRAPH <http://example.org/g> { ?s <http://example.org/p> ?o } }",
        "SELECT ?g ?s ?o WHERE { GRAPH ?g { ?s <http://example.org/p> ?o } }",
        "SELECT ?s ?o WHERE { ?s <http://example.org/p> ?o BIND(STR(?o) AS ?label) } ORDER BY ?s LIMIT 8",
        "SELECT (COUNT(DISTINCT *) AS ?n) WHERE { ?s <http://example.org/p> ?o }",
    ];
    let mut builder=RdfDatasetBuilder::new();
    for index in 0..16 {
        let s=builder.intern_iri(&format!("http://example.org/s{index}"));
        let p=builder.intern_iri("http://example.org/p");let q=builder.intern_iri("http://example.org/q");
        let o=builder.intern_iri(&format!("http://example.org/o{index}"));let v=builder.intern_iri(&format!("http://example.org/v{index}"));
        builder.push_quad(s,p,o,None);builder.push_quad(s,q,v,None);
        let g=builder.intern_iri("http://example.org/g");builder.push_quad(s,p,o,Some(g));
    }
    let dataset=builder.freeze().unwrap();
    for (index,text) in queries.iter().enumerate() {
        let window=CurrentThreadWindow::open();let query=qualification_parse(text);let parsed=window.close();
        let retained=query.retained_size_bytes();
        let window=CurrentThreadWindow::open();let cloned=query.clone();let cloning=window.close();
        let window=CurrentThreadWindow::open();drop(cloned);let dropping=window.close();
        let engine=NativeSparqlEngine::new();
        let window=CurrentThreadWindow::open();let prepared=engine.prepare_query(text,None).unwrap();let preparing=window.close();
        let algebra=query.clone();
        let window=CurrentThreadWindow::open();let admitted=engine.prepare_algebra(algebra,QueryOptions::EMPTY).unwrap();let admitting=window.close();
        assert_eq!(admitted.query(),prepared.query());
        let _warm=engine.query_prepared_view(&*dataset,&prepared,&[],QueryOptions::EMPTY).unwrap();
        let window=CurrentThreadWindow::open();let result=engine.query_prepared_view(&*dataset,&prepared,&[],QueryOptions::EMPTY).unwrap();let executing=window.close();
        let governed=engine.query_prepared_governed_view(&*dataset,&prepared,&[],QueryOptions::EMPTY,&QueryGovernors::METERED).unwrap();
        println!("case{index} retained={retained} plan={} rows={} parse={parsed:?} clone={cloning:?} drop={dropping:?} prepare={preparing:?} admit={admitting:?} reuse={executing:?} consumed={:?}",prepared.retained_size_bytes(),result.solutions().map_or(0,|(_,rows)|rows.len()),governed.evidence().consumed);
        let bindings=[("s".to_owned(),TermValue::iri("http://example.org/s0"))];
        let _warm=engine.query_prepared_view(&*dataset,&prepared,&bindings,QueryOptions::EMPTY).unwrap();
        let window=CurrentThreadWindow::open();
        let bound=engine.query_prepared_view(&*dataset,&prepared,&bindings,QueryOptions::EMPTY).unwrap();
        let substituted=window.close();
        let governed=engine.query_prepared_governed_view(&*dataset,&prepared,&bindings,QueryOptions::EMPTY,&QueryGovernors::METERED).unwrap();
        println!("prebound{index} rows={} reuse={substituted:?} consumed={:?}",bound.solutions().map_or(0,|(_,rows)|rows.len()),governed.evidence().consumed);
    }
    let mut relations=PropertyFunctionRegistry::new();
    relations.register("http://example.org/relation",std::sync::Arc::new(MemoryRelation::new(1,1,(0..16).map(|index|vec![TermValue::iri(format!("http://example.org/s{index}")),TermValue::integer(index)]).collect()).unwrap()));
    let env=ExtensionEnv::over_relations(relations).unwrap();
    let options=QueryOptions::EMPTY.with_env(&env);
    for (index,text) in [
        "SELECT ?s ?value WHERE { ?s <http://example.org/relation> ?value }",
        "SELECT ?s ?value WHERE { VALUES ?s { <http://example.org/s0> <http://example.org/s1> } LATERAL { ?s <http://example.org/relation> ?value } }",
        "SELECT ?s (COUNT(DISTINCT *) AS ?n) WHERE { ?s <http://example.org/relation> ?value } GROUP BY ?s",
    ].into_iter().enumerate() {
        let engine=NativeSparqlEngine::new();
        let window=CurrentThreadWindow::open();
        let prepared=engine.prepare_query_with_options(text,None,options).unwrap();
        let preparing=window.close();
        let _warm=engine.query_prepared_view(&*dataset,&prepared,&[],options).unwrap();
        let window=CurrentThreadWindow::open();
        let result=engine.query_prepared_view(&*dataset,&prepared,&[],options).unwrap();
        let executing=window.close();
        let governed=engine.query_prepared_governed_view(&*dataset,&prepared,&[],options,&QueryGovernors::METERED).unwrap();
        println!("relation{index} plan={} rows={} prepare={preparing:?} reuse={executing:?} consumed={:?}",prepared.retained_size_bytes(),result.solutions().map_or(0,|(_,rows)|rows.len()),governed.evidence().consumed);
    }
}
