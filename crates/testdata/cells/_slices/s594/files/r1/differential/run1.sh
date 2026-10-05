#!/bin/bash
# run1.sh <cell.sv> : PRE, POST one-shot, POST staged, iverilog, verilator, sv2v -> <cell>.{pre,post,stg,ivl,vl,s2v}
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad
SV2V=$S/s580/sv2v/sv2v-macOS/sv2v
f=$1; d=$(dirname $f); b=$(basename $f .sv); cd $d
{ $S/s594/pre/vita $b.sv; echo "rc=$?"; } > $b.pre 2>&1
{ $S/s594/post_a/vita $b.sv; echo "rc=$?"; } > $b.post 2>&1
{ P=$S/s594/post_a/sep; rm -f $b.vu $b.velab; $P/vcmp $b.sv -o $b.vu && $P/velab $b.vu -o $b.velab && $P/vrun $b.velab; echo "rc=$?"; rm -f $b.vu $b.velab; } > $b.stg 2>&1
{ iverilog -g2012 -o $b.vvp $b.sv && perl -e "alarm 15; exec @ARGV" vvp -n $b.vvp; echo "rc=$?"; } > $b.ivl 2>&1
rm -rf vl_$b; { verilator --binary --timing --assert -Wno-fatal --Mdir vl_$b -o sim $b.sv >/dev/null 2>vl_$b.build; brc=$?; if [ $brc -ne 0 ]; then grep -E '%Error' vl_$b.build | head -4; echo "build_rc=$brc"; else perl -e "alarm 15; exec @ARGV" ./vl_$b/sim; echo "rc=$?"; fi; } > $b.vl 2>&1
{ $SV2V $b.sv > $b.s2v.v 2>$b.s2v.err && iverilog -g2012 -o $b.s2v.vvp $b.s2v.v && perl -e "alarm 15; exec @ARGV" vvp -n $b.s2v.vvp; echo "rc=$?"; head -3 $b.s2v.err; } > $b.s2v 2>&1
rm -rf vl_$b vl_$b.build $b.vvp $b.s2v.vvp $b.s2v.err
