PREFIX ex: <http://example.org/>
WITH ex:g
DELETE { ?s ex:p ?o }
INSERT { ?s ex:q ?o }
WHERE  { ?s ex:p ?o }
