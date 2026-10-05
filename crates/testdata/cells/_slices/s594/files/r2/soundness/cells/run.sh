#!/bin/bash
# run.sh cell.sv -> one block: PRE / POST / iverilog / verilator / sv2v raw lines
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad
SV2V=$S/s580/sv2v/sv2v-macOS/sv2v; AL='perl -e "alarm 15; exec @ARGV"'
f=$1; b=$(basename $f .sv); cd $(dirname $f)
r() { grep -vE '^\s*$|^VCD|finish called|\$finish|^- |Verilog \$finish' | grep -vE '^\*\*|^Command|^rc=0$' | head -${2:-6} | sed "s/^/  $1: /"; }
{ $S/s594/pre/vita $b.sv; echo "rc=$?"; } 2>&1 | r PRE
{ $S/s594/post_a/vita $b.sv; echo "rc=$?"; } 2>&1 | r POST
{ iverilog -g2012 -o $b.vvp $b.sv && perl -e 'alarm 15; exec @ARGV' vvp -n $b.vvp; echo "rc=$?"; } 2>&1 | r IVL
rm -rf vl_$b; { verilator --binary --timing -Wno-fatal --Mdir vl_$b -o sim $b.sv >/dev/null 2>vl_$b.build && perl -e 'alarm 15; exec @ARGV' ./vl_$b/sim || { grep -E '%Error' vl_$b.build | head -3; }; } 2>&1 | r VL 4
{ $SV2V $b.sv > $b.s2v.v 2>$b.s2v.err && iverilog -g2012 -o $b.s2v.vvp $b.s2v.v && perl -e 'alarm 15; exec @ARGV' vvp -n $b.s2v.vvp; echo "rc=$?"; head -2 $b.s2v.err; } 2>&1 | r S2V
rm -rf vl_$b vl_$b.build $b.vvp $b.s2v.vvp $b.s2v.err $b.s2v.v
