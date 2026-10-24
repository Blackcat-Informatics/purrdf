The sink at line 1783 prints two page counts from the private native fixture.
Both counts derive from `directory_image(1025)`, which interns only the fixed
`urn:directory:{index}` sequence and constructs deterministic quad rows. The
entire module is under `#[cfg(test)]`; this fixture accepts no external data or
credentials. The adjacent byte-equality assertion is a separate determinism
control. Alert 411 is classified as `used in tests` for this specific synthetic
input and diagnostic. The codec and its certification behavior remain covered
by the existing native controls.
