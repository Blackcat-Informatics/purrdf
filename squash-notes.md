fix(release): install C bundle libraries into portable paths

The C build succeeded, but pinned cargo-c 0.10.23 chooses lib/<Debian multiarch> by default on Ubuntu. The closed portable bundle expects lib/ and lib/pkgconfig/ and correctly refused the unexpected installation. Explicit --libdir lib and --pkgconfigdir lib/pkgconfig make the existing command install exactly the required recipient layout. Four added argv lines in scripts/build-capi-bundle.py; no other source changes.

Refs #375. Reviewed head f9446efd76d8c13a2599f36b1a2e6ba858e565c8; base 040acf33bed71a147f3bdc1224d45028bbcc5564. Pinned upstream cargo-c source confirms the platform default. All four existing capture self-tests pass, preserving current private output, header and library identity checks and stale-output refusals. Ruff check/format and diff checks pass; normal hook and signed commit pass. Independent source review GO confirms recipient and validator paths match.

Library code, ABI, crate payloads and grants are unchanged. Python/npm 3.0.0 have published and passed independent public acceptance at 040acf33; those tags stay fixed. Rust publishing will carry this packaging-only correction, with unchanged library inputs. Earlier Cargo authentication/publication/GitHub Release steps were skipped. No new full runtime qualification is claimed; its successful prior run is preserved.

Standing .goals satisfied. Deficiency ledger empty. Added-line deferral scan clean. No base conflicts or changes. The packaging failure is resolved in the command; actual subsequent build/publication remains to be independently verified.
