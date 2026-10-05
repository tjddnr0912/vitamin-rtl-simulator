#!/bin/bash
# ora.sh <cell.sv>: iverilog 13 -g2012, verilator 5.052 --binary --timing --assert, sv2v 0.0.13 -> iverilog; outputs <cell>.ivl .vl .s2v
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad
SV2V=$S/s580/sv2v/sv2v-macOS/sv2v
f=$1; d=$(dirname $f); b=$(basename $f .sv); cd $d
{ iverilog -g2012 -o $b.vvp $b.sv && perl -e 'alarm 20; exec @ARGV' vvp -n $b.vvp; echo "rc=$?"; } > $b.ivl 2>&1
rm -rf vl_$b; { verilator --binary --timing --assert -Wno-fatal --Mdir vl_$b -o sim $b.sv >/dev/null 2>vl_$b.build; brc=$?; if [ $brc -ne 0 ]; then grep -E '%Error' vl_$b.build | head -5; echo "build_rc=$brc"; else grep -E '%Warning' vl_$b.build | head -3; perl -e 'alarm 20; exec @ARGV' ./vl_$b/sim +verilator+error+limit+1000; echo "rc=$?"; fi; } > $b.vl 2>&1
{ $SV2V $b.sv > $b.s2v.v 2>$b.s2v.err && iverilog -g2012 -o $b.s2v.vvp $b.s2v.v && perl -e 'alarm 20; exec @ARGV' vvp -n $b.s2v.vvp; echo "rc=$?"; head -3 $b.s2v.err; } > $b.s2v 2>&1
rm -rf vl_$b vl_$b.build $b.vvp $b.s2v.vvp $b.s2v.err $b.s2v.v
