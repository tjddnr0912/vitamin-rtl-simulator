#!/bin/bash
# t_run.sh <cell.sv> <vita> : vita on fu, verilator on fu, iverilog + verilator on the plain twin (_fp.sv)
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
f=$1; V=$2; d=$(dirname $f); b=$(basename $f .sv); cd $d
sed -E 's/(unique|priority) if/if/' $b.sv > ${b}_fp.sv
{ $V $b.sv; echo "rc=$?"; } > $b.vita 2>&1
{ $V ${b}_fp.sv; echo "rc=$?"; } > ${b}_fp.vita 2>&1
{ iverilog -g2012 -o $b.vvp ${b}_fp.sv && vvp -n $b.vvp; echo "rc=$?"; } > ${b}_fp.ivl 2>&1
for x in $b ${b}_fp; do rm -rf vl_$x; { verilator --binary --timing --assert -Wno-fatal --Mdir vl_$x -o sim $x.sv >/dev/null 2>vl_$x.build; brc=$?; if [ $brc -ne 0 ]; then grep -E '%Error' vl_$x.build | head -5; echo "build_rc=$brc"; else grep -E '%Warning|-Info' vl_$x.build | head -5; ./vl_$x/sim +verilator+error+limit+1000; echo "rc=$?"; fi; } > $x.vl 2>&1; rm -rf vl_$x; done
rm -f $b.vvp
