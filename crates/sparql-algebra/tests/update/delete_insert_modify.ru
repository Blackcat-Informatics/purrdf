PREFIX ex: <http://example.org/>
DELETE { ?s ex:old ?o }
INSERT { ?s ex:new ?o }
WHERE  { ?s ex:old ?o }
