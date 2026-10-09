# Historical qualification records

These files record an earlier qualification. They are not the current driver.
The retained `qualify.sh` and command logs describe that historical execution;
do not execute them as current acceptance tooling. The current runtime-refusal
driver uses standard Cargo artifact selection through jq and the owning Rust
collector and validator. No Python selector is used by the current qualification.

The exact historical selector bytes are preserved as data:

- Original path: `freeze-controller.py` in this directory.
- Archive path: `freeze-controller.py.historical.txt` in this directory.
- Original and stored size: 1764 bytes; mode: 0644.
- Original and stored BLAKE3: `12a8a5475ae5647dbc54853dc8ac703ac39753b4c4c78909b442cc1ed99a045c`.

To reconstruct the historical record, copy the archive text to the original
relative path, retain mode 0644, and verify that BLAKE3 before interpreting the
historical commands. The script contents and original command logs are unchanged.
Do not restore it as current repository tooling.

The current attempt's session31125 terminated1 at the non-Rust ratchet because
the historical `.py` record was exposed as untracked source. That failure remains
recorded in `/opt/purrdf-308-runtime-refusal.hyKidvRk/ratchet.log` and its mirrored
logs/terminals. Archival representation does not relabel that failed attempt.
Shipping source, checks and invocation policy were not changed by this relocation.
Remaining gates and actual production qualification are still required.
