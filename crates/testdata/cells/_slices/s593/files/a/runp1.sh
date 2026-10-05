#!/bin/bash
# runp1.sh <dir>: also post1 (W alone) on every cell
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad
d=$(cd $1 && pwd); rm -f $d.p1rc
ls $d/*.sv | xargs -P 4 -I{} $S/s593/g/run4.sh {} $S/s593/post1/vita post1 > $d.p1log 2>&1; echo $? > $d.p1rc
