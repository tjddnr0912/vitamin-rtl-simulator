#!/bin/bash
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s582
G=$S/grounding
C=$S/post/census
mkdir -p $C
ls $G/gen/*_ci.sv $G/b2/*_ci.sv $G/b2/m6_*.sv $G/b2/m9_inside_ident.sv $G/b3/*_ci.sv $S/plan/probe/*.sv $S/tests/sv/*.sv $S/tests/px/*.sv > $C/cells.txt
while read f; do
  n=$(echo $f | sed "s#$S/##; s#/#__#g; s#\.sv\$##"); d=$C/$n; mkdir -p $d; cd $d
  $S/post/vita "$f" -o $d/w.vcd > post.out 2>&1; echo "rc=$?" >> post.out
  ( iverilog -g2012 -o iv.vvp "$f" && vvp -n iv.vvp ) > iv.out 2>&1 < /dev/null; echo "rc=$?" >> iv.out
done < $C/cells.txt
