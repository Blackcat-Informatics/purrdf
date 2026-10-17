Published dc569c874 through normal hooks and push. The only source delta removes
the complete report from two assertion failure messages; both conditions and
all production code are unchanged. Affected strict test compilation, both native
cases and both exact scalar-WASM cases pass (no filtered cases).

CodeQL's two cleartext-output annotations are addressed in source, not dismissed
or suppressed. Refreshed hosted CodeQL must still establish their resolution.
Unchanged CLI, current-support, production strict/helper and CodeRabbit source
evidence retain their scope; none is relabelled as a fresh whole-suite execution.
No full local gate or unrelated suite was repeated.
