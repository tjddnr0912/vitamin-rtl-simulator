#!/bin/bash
# run7.sh <dir> <cell> base           -> <cell>.{pre,ivl,vl,s2v}
# run7.sh <dir> <cell> <tag> <bindir> -> <cell>.<tag> (one-shot <bindir>/vita) and <cell>.<tag>stg (staged <bindir>/sep)
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad
SV2V=$S/s580/sv2v/sv2v-macOS/sv2v
cd $1; b=$2
if [ -f $b.files ]; then F=$(cat $b.files); else F=$b.sv; fi
if [ "$3" = base ]; then
 { $S/s591/pre/vita $F; echo "rc=$?"; } > $b.pre 2>&1
 { iverilog -g2012 -o $b.vvp $F && vvp -n $b.vvp; echo "rc=$?"; } > $b.ivl 2>&1
 rm -rf vl_$b; { verilator --binary --timing --assert -Wno-fatal --top-module top --Mdir vl_$b -o sim $F >/dev/null 2>vl_$b.build; brc=$?; if [ $brc -ne 0 ]; then grep -E '%Error' vl_$b.build | head -4; echo "build_rc=$brc"; else grep -E '%Warning' vl_$b.build | head -2; ./vl_$b/sim; echo "rc=$?"; fi; } > $b.vl 2>&1
 { $SV2V $F > $b.s2v.v 2>$b.s2v.err && iverilog -g2012 -o $b.s2v.vvp $b.s2v.v && vvp -n $b.s2v.vvp; echo "rc=$?"; head -2 $b.s2v.err; } > $b.s2v 2>&1
 rm -rf vl_$b vl_$b.build $b.vvp $b.s2v.vvp $b.s2v.err $b.s2v.v
else
 T=$3; B=$4
 { $B/vita $F; echo "rc=$?"; } > $b.$T 2>&1
 { $B/sep/vcmp $F -o $b.$T.vu && $B/sep/velab $b.$T.vu -o $b.$T.velab && $B/sep/vrun $b.$T.velab; echo "rc=$?"; } > $b.${T}stg 2>&1
 rm -f $b.$T.vu $b.$T.velab
fi
