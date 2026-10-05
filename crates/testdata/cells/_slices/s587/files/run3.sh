#!/bin/bash
# usage: run3.sh <cell.sv> [vita-binary]   -> writes <cell>.vita/.ivl/.vl next to the cell
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s587
f=$1; V=${2:-$S/pre/vita}; d=$(dirname $f); b=$(basename $f .sv); cd $d
rm -rf obs_$b; { $V --obs-dir obs_$b $b.sv; echo "rc=$?"; } > $b.vita 2>&1
{ iverilog -g2012 -o $b.vvp $b.sv && vvp -n $b.vvp; echo "rc=$?"; } > $b.ivl 2>&1
rm -rf vl_$b; { verilator --binary --timing --assert -Wno-fatal --Mdir vl_$b -o sim $b.sv >/dev/null 2>vl_$b.build; brc=$?; if [ $brc -ne 0 ]; then grep -E '%Error' vl_$b.build | head -5; echo "build_rc=$brc"; else grep -E '%Warning' vl_$b.build | head -3; ./vl_$b/sim +verilator+error+limit+1000; echo "rc=$?"; fi; } > $b.vl 2>&1
rm -rf vl_$b $b.vvp
