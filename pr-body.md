The full release preflight rejected the standalone-workspace test's
single-element iterator. Supply that manifest record through `std::iter::once`
so the required test-target lint accepts it.

Strict release Clippy including all testkit test targets and all eight
focused native path cases pass. Tracks #1235;
the main-only release preflight will run again before tags are created.
