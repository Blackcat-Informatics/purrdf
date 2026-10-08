# Why not Rust: This qualification invokes genuine Python RDFLib custom functions to record its observable callback order.
import json
import hashlib
from pathlib import Path
import rdflib
from rdflib.plugins.sparql.operators import register_custom_function

assert rdflib.__version__ == "7.6.0"
print(json.dumps({"oracle_version": rdflib.__version__, "oracle_module": rdflib.__file__, "source_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), "lock_sha256": hashlib.sha256(Path("uv.lock").read_bytes()).hexdigest()}))
calls = []

def counter(value):
    calls.append(int(value))
    return rdflib.Literal(len(calls))

register_custom_function(rdflib.URIRef("http://example.org/counter"), counter)
cases = [
    ("false-first", "SELECT ?x", "VALUES ?x {1} FILTER(false) FILTER(ex:counter(1)=1)"),
    ("error-first", "SELECT ?x", "VALUES ?x {1} FILTER(1/0) FILTER(ex:counter(2)=1)"),
    ("three-in-order", "SELECT ?x", "VALUES ?x {1} FILTER(ex:counter(1)=1) FILTER(ex:counter(2)=2) FILTER(ex:counter(3)=3)"),
    ("three-reversed", "SELECT ?x", "VALUES ?x {1} FILTER(ex:counter(3)=3) FILTER(ex:counter(2)=2) FILTER(ex:counter(1)=1)"),
    ("ask-first-witness", "ASK", "VALUES ?x {1 2 3} FILTER(ex:counter(4)>=1) FILTER(ex:counter(5)>=1)"),
]
for name, form, body in cases:
    calls.clear()
    result = rdflib.Graph().query("PREFIX ex:<http://example.org/> " + form + " WHERE { " + body + " }")
    answer = bool(result) if result.type == "ASK" else [[str(term) if term is not None else None for term in row] for row in result]
    print(json.dumps({"case": name, "type": result.type, "answer": answer, "arguments": calls}))
