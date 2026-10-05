#!/bin/bash
# TAG=x [SV=1] [SVD=-DNOC] [VL=1] h3.sh file.sv [defines...]
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s580
f=$1; shift; b=${f%.sv}${TAG:-}
for be in native interp vm; do $S/post3/vita --backend $be "$@" $f > $b.p3$be.txt 2>&1; echo "rc=$?" >> $b.p3$be.txt; done
$S/post2/vita "$@" $f > $b.p2.txt 2>&1; echo "rc=$?" >> $b.p2.txt
$S/pre/vita "$@" $f > $b.pre.txt 2>&1; echo "rc=$?" >> $b.pre.txt
iverilog -g2012 ${IVD:-} "$@" -o $b.vvp $f > $b.iv.txt 2>&1 && vvp -n $b.vvp >> $b.iv.txt 2>&1; echo "rc=$?" >> $b.iv.txt
T="p3native p3interp p3vm p2 pre iv"
if [ "${SV:-0}" = 1 ]; then $S/sv2v/sv2v-macOS/sv2v ${SVD:-} "$@" $f > $b.sv2v.v 2> $b.sv.txt && iverilog -g2012 -o $b.svvp $b.sv2v.v >> $b.sv.txt 2>&1 && vvp -n $b.svvp >> $b.sv.txt 2>&1; echo "rc=$?" >> $b.sv.txt; T="$T sv"; fi
if [ "${VL:-0}" = 1 ]; then rm -rf $b.obj; verilator --binary -Wno-fatal -Wno-lint -Wno-style --top-module t "$@" $f -Mdir $b.obj > $b.vlb.txt 2>&1 && $b.obj/Vt > $b.vl.txt 2>&1; echo "rc=$?" >> $b.vl.txt; T="$T vl"; fi
python3 $S/lens_snd3/join3.py $b $T
