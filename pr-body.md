An answer callback that panics now unwinds through the existing evaluator boundary guard. It restores local caps, node cursors and evidence frames before the context can be reused. Normal completion and returned-error precedence continue through the existing finish implementation. The guard adds no allocation or second Drop implementation, and the regression reuses the existing provider.

The callback/error and healthy-reuse regression passes, as do existing scope/provider controls, native/scalar/SIMD stack tests, strict Clippy and helper census. The current hosted run is fully successful, including library/Python, portable execution, optimized package and all seven assembly configurations. Its complete 112-site projection was generated and checked by the original producer and is identical to the maintained document.

The final local `make check` also passed, including its nested release-WASM build. PR feedback and final integration assessment remain pending. Retained fixed-work evidence applies to its recorded inputs and does not establish a DISTINCT instruction win.

Closes #545.
