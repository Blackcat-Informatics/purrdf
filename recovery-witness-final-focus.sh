#!/usr/bin/env bash
set -euo pipefail
export CI=1
stage=.stage/geo-deterministic-hierarchical-spatial
receipt="$stage/recovery-witness-final-atlas.log"
test ! -e "$receipt"
sha256sum crates/geo-kernel/src/topology/{relate,noding,segment,arrangement,mod}.rs crates/geo-kernel/src/atlas/{relate,arcs/arrangement}.rs crates/geo-kernel/src/context/retained.rs > "$stage/recovery-witness-final-source.sha256"
overall=0
status=0
cargo test --locked -p purrdf-geo-kernel --lib atlas::relate:: > "$receipt" 2>&1 || status=$?
printf 'terminal_status=%s\n' "$status" >> "$receipt"
if (( status )); then overall=1; fi
receipt="$stage/recovery-witness-final-topology.log"
test ! -e "$receipt"
status=0
cargo test --locked -p purrdf-geo-kernel --lib topology:: > "$receipt" 2>&1 || status=$?
printf 'terminal_status=%s\n' "$status" >> "$receipt"
if (( status )); then overall=1; fi
bash "$stage/recovery-governor-regenerate.sh" || overall=1
exit "$overall"
