# Measurement lane notes (m2)

All evidence is in /home/paudley/Active/purrdf/.stage/bench-1281/ with the `m2_` prefix.

- Driver source: m2_driver.rs (placeholders @U@ @O@), installed by m2_mk.sh as the untracked
  example crates/shapes/examples/m2_drv.rs in both checkouts. Throwaway; nothing committed.
- Builds: m2_build.sh <checkout> <tag> [bench]; binaries m2_bin_main, m2_bin_old (release),
  m2_bin_mainsym, m2_bin_oldsym (bench profile = release with the symbol table kept; the
  bench profile writes to the release directory, so those two were copied by hand).
- Counts: m2_measure.sh (perf stat instructions:u, setarch -R, difference of process runs).
  Logs m2_07 (queries, 18,100 subjects), m2_08 (SHACL), m2_09 (181,000 subjects).
- Table generator: m2_table.py; profiles: m2_perf.sh + m2_perfdiff.py, outputs m2_16, m2_17.
- Result table: m2_table.md.
- The untracked example file is left in both checkouts (crates/shapes/examples/m2_drv.rs).
