PREFIX ex: <http://example.org/>
DELETE { ?s ex:p ?o }
USING ex:g1
USING NAMED ex:g2
WHERE { GRAPH ex:g2 { ?s ex:p ?o } }
