# Initial source-view compilation failures

These are failure summaries from the actual compiler output, not passing qualification.

The first focused build failed with unreachable_pub on the new source module and missing_debug_implementations on eight public view types. The module is now exported through the crate root and views have explicit Debug implementations; the subsequent four-case run passed. The expanded six-case run with schema views and metadata also passed.

Adding actual role/nominal/legacy/stop controls introduced an unnecessary qualification after rdf was imported. The compiler correctly rejected the fully qualified rdf::DIR_LANG_STRING path under unused_qualifications. It was changed to the imported rdf spelling. The ten-case rerun is pending; there was no suppression or compiler-policy change.
