#!/bin/bash
# usage: TAG=x h.sh file.sv [defines...]  -> vita PRE/POST (inside variant), PRE/POST -DIV (==? variant), iverilog -DIV
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s580
D=$S/lens_diff
f=$1; shift; b=${f%.sv}${TAG:-}
for v in pre post; do
  $S/$v/vita "$@" $f > $b.$v.txt 2>&1; echo "rc=$?" >> $b.$v.txt
  if [ "${NOQ:-0}" = 0 ]; then $S/$v/vita -DIV "$@" $f > $b.${v}q.txt 2>&1; echo "rc=$?" >> $b.${v}q.txt; fi
done
if [ "${NOQ:-0}" = 0 ]; then
iverilog -g2012 -DIV "$@" -o $b.vvp $f > $b.iv.txt 2>&1 && vvp -n $b.vvp >> $b.iv.txt 2>&1; echo "rc=$?" >> $b.iv.txt
fi
python3 $D/join.py $b
