#!/bin/bash
# h.sh <cell.sv>...: PRE, POST one-shot, PRE/POST staged, iverilog, verilator, sv2v -> <cell>.{pre,post,pres,posts,ivl,vl,s2v}; prints summary
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad
PRE=$S/s589/pre/vita; POST=$S/s589/post_a/vita; PS=$S/s589/pre/sep; QS=$S/s589/post_a/sep
SV2V=$S/tools/sv2v-macOS/sv2v
one() { { perl -e 'alarm 20; exec @ARGV' "$1" "$2"; echo "rc=$?"; } > "$3" 2>&1; }
stg() { local P=$1 f=$2 o=$3; { perl -e 'alarm 20; exec @ARGV' $P/vcmp "$f" -o "$o.vu" && perl -e 'alarm 20; exec @ARGV' $P/velab "$o.vu" -o "$o.velab" && perl -e 'alarm 20; exec @ARGV' $P/vrun "$o.velab"; echo "rc=$?"; } > "$o" 2>&1; rm -f "$o.vu" "$o.velab"; }
for f in "$@"; do
  d=$(dirname $f); b=$(basename $f .sv); cd $d
  one $PRE $b.sv $b.pre; one $POST $b.sv $b.post; stg $PS $b.sv $b.pres; stg $QS $b.sv $b.posts
  { iverilog -g2012 -o $b.vvp $b.sv && perl -e 'alarm 20; exec @ARGV' vvp -n $b.vvp; echo "rc=$?"; } > $b.ivl 2>&1
  rm -rf vl_$b; { verilator --binary --timing --assert -Wno-fatal --Mdir vl_$b -o sim $b.sv >/dev/null 2>vl_$b.build; brc=$?; if [ $brc -ne 0 ]; then grep -E '%Error' vl_$b.build | head -3; echo "build_rc=$brc"; else perl -e 'alarm 20; exec @ARGV' ./vl_$b/sim; echo "rc=$?"; fi; } > $b.vl 2>&1
  { $SV2V $b.sv > $b.s2v.v 2>$b.s2v.err && iverilog -g2012 -o $b.s2v.vvp $b.s2v.v && perl -e 'alarm 20; exec @ARGV' vvp -n $b.s2v.vvp; echo "rc=$?"; head -3 $b.s2v.err; } > $b.s2v 2>&1
  rm -rf vl_$b vl_$b.build $b.vvp $b.s2v.vvp $b.s2v.err $b.s2v.v
done
