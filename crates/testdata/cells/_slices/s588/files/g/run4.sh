#!/bin/bash
# run4.sh <cell.sv> [vita]  -> <b>.PRE <b>.ivl <b>.vl <b>.s2v next to the cell
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad
f=$1; V=${2:-$S/s588/pre/vita}; SFX=${3:-PRE}; d=$(dirname $f); b=$(basename $f .sv); cd $d
{ $V $b.sv; echo "rc=$?"; } > $b.$SFX 2>&1
[ "$SFX" != PRE ] && exit 0
{ iverilog -g2012 -o $b.vvp $b.sv && perl -e "alarm 30; exec @ARGV" vvp -n $b.vvp; echo "rc=$?"; } > $b.ivl 2>&1
{ $S/s588/g/tools/sv2v $b.sv > $b.s2v.v 2>$b.s2v.err && iverilog -g2012 -o $b.s2vvp $b.s2v.v && perl -e "alarm 30; exec @ARGV" vvp -n $b.s2vvp; echo "rc=$?"; cat $b.s2v.err; } > $b.s2v 2>&1
rm -rf vl_$b; { verilator --binary --timing -Wno-fatal -Wno-lint --Mdir vl_$b -o sim $b.sv >/dev/null 2>vl_$b.build; brc=$?; if [ $brc -ne 0 ]; then grep -E '%Error' vl_$b.build | head -5; echo "build_rc=$brc"; else perl -e "alarm 30; exec @ARGV" ./vl_$b/sim; echo "rc=$?"; fi; } > $b.vl 2>&1
rm -rf vl_$b vl_$b.build $b.vvp $b.s2vvp
