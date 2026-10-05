#!/bin/bash
# run.sh <probe.sv> : vita PRE/POST, iverilog direct (-DNO_INSIDE), sv2v->iverilog, verilator (-DTWO_STATE)
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s580
f=$1; b=$(basename $f .sv); d=$(dirname $f); shift
for v in pre post; do $S/$v/vita "$@" $f > $d/$b.$v.txt 2>&1; echo "rc=$?" >> $d/$b.$v.txt; done
iverilog -g2012 -DNO_INSIDE -o $d/$b.ivd.out $f > $d/$b.ivd.txt 2>&1 && vvp -n $d/$b.ivd.out >> $d/$b.ivd.txt 2>&1
$S/sv2v/sv2v-macOS/sv2v $f > $d/$b.sv2v.v 2> $d/$b.sv2v.err && iverilog -g2012 -o $d/$b.iv.out $d/$b.sv2v.v > $d/$b.iv.txt 2>&1 && vvp -n $d/$b.iv.out >> $d/$b.iv.txt 2>&1 || cat $d/$b.sv2v.err >> $d/$b.iv.txt
if [ -z "$NOVL" ]; then
verilator --binary --timing -Wno-fatal -Wno-lint -Wno-style -DTWO_STATE -Mdir $d/obj_$b --top-module t $f > $d/$b.vlb.txt 2>&1 && $d/obj_$b/Vt > $d/$b.vl.txt 2>&1 || tail -5 $d/$b.vlb.txt > $d/$b.vl.txt
fi
for k in pre post ivd iv vl; do echo "--- $k"; grep -v '^\s*$' $d/$b.$k.txt 2>/dev/null | grep -v 'VCD\|^- \|finish called\|Verilog \$finish' | head -${LINES:-14}; done
