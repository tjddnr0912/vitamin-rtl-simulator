#!/bin/bash
# TAG=x [NOQ=1] [SV=1] [VL=1] [TOP=t] h2.sh file.sv [defines...]
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s580
D=$S/lens_diff2
f=$1; shift; b=${f%.sv}${TAG:-}
for v in pre post post2; do
  $S/$v/vita "$@" $f > $b.$v.txt 2>&1; echo "rc=$?" >> $b.$v.txt
  if [ "${NOQ:-0}" = 0 ]; then $S/$v/vita -DIV "$@" $f > $b.${v}q.txt 2>&1; echo "rc=$?" >> $b.${v}q.txt; fi
done
if [ "${NOQ:-0}" = 0 ]; then
  iverilog -g2012 -DIV "$@" -o $b.vvp $f > $b.iv.txt 2>&1 && vvp -n $b.vvp >> $b.iv.txt 2>&1; echo "rc=$?" >> $b.iv.txt
fi
if [ "${SV:-0}" = 1 ]; then
  $S/sv2v/sv2v-macOS/sv2v "$@" $f > $b.sv2v.v 2> $b.sv.txt && iverilog -g2012 -o $b.svvp $b.sv2v.v >> $b.sv.txt 2>&1 && vvp -n $b.svvp >> $b.sv.txt 2>&1; echo "rc=$?" >> $b.sv.txt
fi
if [ "${VL:-0}" = 1 ]; then
  rm -rf $b.obj; verilator --binary -Wno-fatal -Wno-lint -Wno-style --top-module ${TOP:-t} "$@" $f -Mdir $b.obj > $b.vlb.txt 2>&1 && $b.obj/V${TOP:-t} > $b.vl.txt 2>&1; echo "rc=$?" >> $b.vl.txt
fi
python3 $D/join2.py $b
