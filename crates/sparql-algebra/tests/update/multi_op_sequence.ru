PREFIX ex: <http://example.org/>
INSERT DATA { ex:s ex:p ex:o } ;
DELETE WHERE { ?s ex:gone ?o } ;
INSERT { ?s ex:derived ?o } WHERE { ?s ex:p ?o }
