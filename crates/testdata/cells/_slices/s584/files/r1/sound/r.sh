#!/bin/bash
# usage: r.sh cell.sv [oracles]   -> runs PRE/A/AB x native/interp/vm (+ iverilog/verilator if 2nd arg = o)
D=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s584/r1/sound
B=$D/..
f=$1; n=$(basename $f .sv); W=$D/w/$n; mkdir -p $W
if [ "$2" = o ]; then
  ( cd $W && iverilog -g2012 -o a.vvp $f > iv_c.txt 2>&1; echo "iv compile rc=$?"; [ -f a.vvp ] && perl -e 'alarm 20; exec @ARGV' vvp -n a.vvp 2>&1 | grep -v '^VCD info'; echo "iv rc=$?" ) > $W/iv.out 2>&1
  echo "## iverilog"; cat $W/iv_c.txt $W/iv.out | head -40
  ( cd $W && rm -rf obj && perl -e 'alarm 120; exec @ARGV' verilator --binary --timing --assert -Wno-fatal -Wno-lint -Wno-style --top-module top -Mdir obj $f > vl_c.txt 2>&1; echo "vl compile rc=$?"; [ -x obj/Vtop ] && perl -e 'alarm 20; exec @ARGV' obj/Vtop 2>&1 | grep -v '^- '; ) > $W/vl.out 2>&1
  echo "## verilator"; grep -i 'error' $W/vl_c.txt | head -5; cat $W/vl.out | head -40
fi
for bin in pre a ab; do
  prev=""
  for be in native interp vm; do
    out=$(cd $W && perl -e 'alarm 20; exec @ARGV' $B/vita_$bin --backend $be -o $W/v.vcd $f 2>&1 | grep -v 'VITA-W1017'; echo "rc=${PIPESTATUS[0]}")
    if [ "$out" != "$prev" ]; then echo "## vita_$bin $be"; echo "$out"; fi
    prev="$out"
  done
done
