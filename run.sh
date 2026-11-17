#!/usr/bin/env bash
# usage: run.sh <step> <purpose> <n> <cmd...>  (cwd: the worktree)
# Records wrapper identity before/after; runs the one command under the build lock.
W=/home/paudley/stage/scripts/cargo
D=/home/paudley/Active/purrdf/.worktrees/1351-switch-canonical-output-to-the-new/.stage/switch-canonical-output-to-the-new
step=$1; purpose=$2; n=$3; shift 3
log=$D/s${step}_${purpose}_${n}.log
{
  echo "dir=$PWD commit=$(git rev-parse HEAD)"
  echo "cmd=$*"
  echo "wrapper-before: $(stat -c '%y' $W) $(sha256sum $W | cut -d' ' -f1)"
} > "$log"
flock /home/paudley/Active/purrdf/.stage/build.lock "$@" >> "$log" 2>&1
rc=$?
{
  echo "wrapper-after: $(stat -c '%y' $W) $(sha256sum $W | cut -d' ' -f1)"
  echo "EXIT=$rc"
} >> "$log"
echo "EXIT=$rc log=$log"
exit $rc
