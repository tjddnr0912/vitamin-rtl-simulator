#!/bin/sh
# usage: run_ip.sh <name> <top> [tools]   tools: any of deps vl iv sv vita (default all)
. /private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/c40e1dc3-96e1-4997-a40d-90a56b79d219/scratchpad/row9/A-opentitan/bin/env.sh
name=$1; top=$2; tools=${3:-"deps vl iv sv vita"}
d=$R/ips/$name; mkdir -p $d; cd $d
W="perl $S/s2-review/sound/p3/wd.pl"
for t in $tools; do case $t in
deps) python3 -I $R/bin/deps.py $OT $top $EXTRA > files.txt 2> deps.err; echo "deps: $(wc -l < files.txt) files; $(cat deps.err)";;
vl) $R/bin/lock.sh $W 1200 10000000 vl.log /opt/homebrew/bin/verilator --lint-only -Wno-fatal --top-module $top $DEFS $INCS $(cat files.txt) > vl.wd 2>&1; echo "vl: $(cat vl.wd) err=$(grep -c '^%Error' vl.log) warn=$(grep -c '^%Warning' vl.log)";;
iv) $W 300 6000000 iv.log /opt/homebrew/bin/iverilog -g2012 -s $top $DEFS $INCS -o iv.vvp $(cat files.txt) > iv.wd; echo "iv: $(cat iv.wd) lines=$(grep -c . iv.log)";;
sv) $W 300 6000000 sv2v.log $SV2V $DEFS $INCS $(cat files.txt) -w sv2v_out.v > sv.wd; echo "sv2v: $(cat sv.wd)"; if [ -s sv2v_out.v ]; then $W 300 6000000 sviv.log /opt/homebrew/bin/iverilog -g2012 -s $top -o sviv.vvp sv2v_out.v > sviv.wd; echo "sv2v->iv: $(cat sviv.wd) lines=$(grep -c . sviv.log)"; fi;;
vita) $W 300 6000000 vita.log $VITA --top $top --timeout 1000 $DEFS $INCS $(cat files.txt) > vita.wd; echo "vita: $(cat vita.wd) $(grep -E '^errors=' vita.log)";;
vitap) $R/bin/ovl.sh files.txt > files.p.txt; $W 300 6000000 vitap.log $VITA --top $top --timeout 1000 $DEFS -I$R/patched/hw/ip/prim/rtl -I$R/patched/hw/dv/sv/dv_utils $(cat files.p.txt) > vitap.wd; echo "vitaP: $(cat vitap.wd) $(grep -E '^errors=' vitap.log)";;
esac; done
