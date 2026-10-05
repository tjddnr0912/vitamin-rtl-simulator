#!/bin/bash
# runp.sh <dir>: PRE + 3 oracles (run4.sh default) then CUT (tag cut) on every cell, 4-way; marker <dir>.rc
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad
G=$S/s593/g; d=$(cd $1 && pwd)
rm -f $d.rc
ls $d/*.sv | xargs -P 4 -I{} $G/run4.sh {} > $d.runlog 2>&1
ls $d/*.sv | xargs -P 4 -I{} $G/run4.sh {} $S/s593/post5/vita cut >> $d.runlog 2>&1
echo $? > $d.rc
