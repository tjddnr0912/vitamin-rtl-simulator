#!/bin/bash
# r.sh <probe.sv> [defines] : vita PRE/POST/POST2(native,interp,vm), iverilog direct (-DNO_INSIDE), sv2v->iverilog, verilator (VL=1)
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s580
f=$1; shift; b=${f%.sv}${TAG:-}
for v in pre post post2; do $S/$v/vita "$@" $f > $b.$v.txt 2>&1; echo "rc=$?" >> $b.$v.txt; done
for be in interp vm; do $S/post2/vita --backend $be "$@" $f > $b.p2$be.txt 2>&1; echo "rc=$?" >> $b.p2$be.txt; done
iverilog -g2012 -DNO_INSIDE -DNOCLASS ${IVD:-} "$@" -o $b.ivd.out $f > $b.ivd.txt 2>&1 && vvp -n $b.ivd.out >> $b.ivd.txt 2>&1; echo "rc=$?" >> $b.ivd.txt
$S/sv2v/sv2v-macOS/sv2v -DNOCLASS ${SVD:-} "$@" $f > $b.sv2v.v 2> $b.sv2v.err && iverilog -g2012 -o $b.iv.out $b.sv2v.v > $b.iv.txt 2>&1 && vvp -n $b.iv.out >> $b.iv.txt 2>&1; echo "rc=$? $(head -c 300 $b.sv2v.err)" >> $b.iv.txt
if [ -n "$VL" ]; then rm -rf $b.obj; verilator --binary --timing -Wno-fatal -Wno-lint -Wno-style -DTWO_STATE -DNOCLASS "$@" -Mdir $b.obj --top-module t $f > $b.vlb.txt 2>&1 && $b.obj/Vt > $b.vl.txt 2>&1; echo "rc=$?" >> $b.vl.txt; [ -s $b.vl.txt ] || tail -3 $b.vlb.txt > $b.vl.txt; fi
for k in pre post post2 p2interp p2vm ivd iv vl; do [ -f $b.$k.txt ] || continue; echo "--- $k"; grep -v '^\s*$' $b.$k.txt | grep -vE 'VCD|^- |finish called|Verilog \$finish|^\s*\^|-->|^\s*\|' | cut -c1-200 | head -${LINES:-12}; done
