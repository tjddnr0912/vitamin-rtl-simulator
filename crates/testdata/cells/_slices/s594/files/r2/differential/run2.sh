#!/bin/bash
# run2.sh <cell.sv> [full]: post_b one-shot (.postb) + post_b staged (.stgb); with "full" also PRE, post_a, oracles
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad
f=$1; d=$(dirname $f); b=$(basename $f .sv); cd $d
{ $S/s594/post_b/vita $b.sv; echo "rc=$?"; } > $b.postb 2>&1
{ P=$S/s594/post_b/sep; rm -f $b.vub $b.velabb; $P/vcmp $b.sv -o $b.vub && $P/velab $b.vub -o $b.velabb && $P/vrun $b.velabb; echo "rc=$?"; rm -f $b.vub $b.velabb; } > $b.stgb 2>&1
[ "$2" = full ] && $S/s594/r1/differential/run1.sh $d/$b.sv
exit 0
