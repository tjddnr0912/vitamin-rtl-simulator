#!/bin/bash
# atk3.sh <dir> : u/<b>.sv (+ u/*.svh, u/extra/<b>_*.sv) -> p/ plain twin; run PRE, POSTB, POSTC, iverilog, verilator on both
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s587; D=$1
shopt -s nullglob
mkdir -p $D/p/extra
for f in $D/u/*.sv $D/u/*.svh $D/u/extra/*.sv; do r=${f#$D/u/}; perl -pe 's/\b(unique0|unique|priority) if\b/if/g' $f > $D/p/$r; done
for f in $D/u/*.sv; do b=$(basename $f .sv)
 for v in u p; do cd $D/$v; ex=$(for x in extra/${b}_*.sv; do printf "%s " "$x"; done)
  { $S/pre/sep/vita -I . $ex $b.sv; echo "rc=$?"; } > $b.PRE 2>&1
  { $S/post_b/vita -I . $ex $b.sv; echo "rc=$?"; } > $b.POSTB 2>&1
  { $S/post_c/vita -I . $ex $b.sv; echo "rc=$?"; } > $b.POST 2>&1
  { iverilog -g2012 -I. -o $b.vvp $ex $b.sv && vvp -n $b.vvp; echo "rc=$?"; } > $b.ivl 2>&1; rm -f $b.vvp
  rm -rf vl_$b; { verilator --binary --timing --assert -Wno-fatal -I. --Mdir vl_$b -o sim $ex $b.sv >/dev/null 2>vl_$b.build; brc=$?; if [ $brc -ne 0 ]; then grep -E '%Error' vl_$b.build | head -5; echo "build_rc=$brc"; else grep -E '%Warning' vl_$b.build | head -3; ./vl_$b/sim +verilator+error+limit+1000; echo "rc=$?"; fi; } > $b.vl 2>&1; rm -rf vl_$b
 done
done
echo done > $D/atk3.rc
