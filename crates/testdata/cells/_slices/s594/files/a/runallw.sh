#!/bin/bash
# runall.sh <dir> [binary tag]: run4 on every cell (4 parallel); marker <dir>.<tag>.rc
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad
d=$(cd $1 && pwd); V=${2:-$S/s592/pre/vita}; T=${3:-pre}
rm -f $d.$T.rc
ls $d/*.sv | xargs -P 4 -I{} $S/s594/a/run4w.sh {} $V $T > $d.$T.runlog 2>&1
echo $? > $d.$T.rc
