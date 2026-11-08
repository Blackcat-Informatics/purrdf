Earlier SHACL laws must reject a multi-valued scalar argument before SPARQL
`COALESCE` or `IF` can mask it. Limit the new masking behavior to the explicit
November law, restoring the v4.0.0 behavior checked by the independent reference
interpreter. Grade all five current concat fixtures under November, including
the two empty-list boundaries.

Refresh exactly two OpenAPI version fields in the current schema golden from
4.0.0 to 4.0.1. Keep the historical golden and semantic comparison unchanged.

The reference interpreter remains unchanged and passes all 2,048 generated
worlds. Native verification passes 113 Shapes cases and two CLI cases; strict
affected library/test/binary Clippy and independent review pass. The prepared
product stage and fixture bytes are unchanged. Both focused OpenAPI golden and
historical semantic comparison cases pass with the version correction, which
also matches the captured current output byte for byte. Tracks #1235; the complete
mandatory main release preflight remains required before tags.
