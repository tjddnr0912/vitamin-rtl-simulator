#!/bin/bash
# run.sh <cell.sv>  -> .pre .p4 .p4nb (ibody off) .ivl .vl .s2v
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad
SV2V=$S/s580/sv2v/sv2v-macOS/sv2v
f=$1; d=$(dirname $f); b=$(basename $f .sv); cd $d
{ $S/s589/pre/vita $b.sv; echo "rc=$?"; } > $b.pre 2>&1
{ $S/s589/g/post4/vita $b.sv; echo "rc=$?"; } > $b.p4 2>&1
{ VITA_PROTO_OFF=ibody $S/s589/g/post4/vita $b.sv; echo "rc=$?"; } > $b.p4nb 2>&1
[ -n "$NOORA" ] && exit 0
{ iverilog -g2012 -o $b.vvp $b.sv && vvp -n $b.vvp; echo "rc=$?"; } > $b.ivl 2>&1
rm -rf vl_$b; { verilator --binary --timing --assert -Wno-fatal --Mdir vl_$b -o sim $b.sv >/dev/null 2>vl_$b.build; brc=$?; if [ $brc -ne 0 ]; then grep -E '%Error' vl_$b.build | head -4; echo "build_rc=$brc"; else ./vl_$b/sim; echo "rc=$?"; fi; } > $b.vl 2>&1
{ $SV2V $b.sv > $b.s2v.v 2>$b.s2v.err && iverilog -g2012 -o $b.s2v.vvp $b.s2v.v && vvp -n $b.s2v.vvp; echo "rc=$?"; head -3 $b.s2v.err; } > $b.s2v 2>&1
rm -rf vl_$b vl_$b.build $b.vvp $b.s2v.vvp $b.s2v.err $b.s2v.v
