#!/bin/bash
# run_r2.sh <dir> <cell> [oracles=1]: writes <cell>.{pre,posta,postb,postbstg} (+ ivl vl s2v when oracles=1); files = <cell>.files else <cell>.sv; -I. everywhere
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad
SV2V=$S/s580/sv2v/sv2v-macOS/sv2v
cd $1; b=$2; O=${3:-1}
if [ -f $b.files ]; then F=$(cat $b.files); else F=$b.sv; fi
if [ "$O" = 1 ]; then
 { $S/s591/pre/vita -I . $F; echo "rc=$?"; } > $b.pre 2>&1
 { $S/s591/post_a/vita -I . $F; echo "rc=$?"; } > $b.posta 2>&1
fi
{ $S/s591/post_b/vita -I . $F; echo "rc=$?"; } > $b.postb 2>&1
P=$S/s591/post_b/sep
{ $P/vcmp -I . $F -o $b.vu && $P/velab $b.vu -o $b.velab && $P/vrun $b.velab; echo "rc=$?"; } > $b.postbstg 2>&1
rm -f $b.vu $b.velab
[ "$O" = 1 ] || exit 0
{ iverilog -g2012 -I. -o $b.vvp $F && vvp -n $b.vvp; echo "rc=$?"; } > $b.ivl 2>&1
rm -rf vl_$b; { verilator --binary --timing --assert -Wno-fatal -I. --top-module top --Mdir vl_$b -o sim $F >/dev/null 2>vl_$b.build; brc=$?; if [ $brc -ne 0 ]; then grep -E '%Error' vl_$b.build | head -4; echo "build_rc=$brc"; else ./vl_$b/sim; echo "rc=$?"; fi; } > $b.vl 2>&1
{ $SV2V -I . $F > $b.s2v.v 2>$b.s2v.err && iverilog -g2012 -o $b.s2v.vvp $b.s2v.v && vvp -n $b.s2v.vvp; echo "rc=$?"; head -2 $b.s2v.err; } > $b.s2v 2>&1
rm -rf vl_$b vl_$b.build $b.vvp $b.s2v.vvp $b.s2v.err $b.s2v.v
