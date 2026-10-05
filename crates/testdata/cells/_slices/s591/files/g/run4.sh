#!/bin/bash
# usage: run4.sh <cell.sv> [vita-binary] -> <cell>.pre .ivl .vl .s2v (+ obs_<cell>/run.json routes in .route)
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad
SV2V=$S/s580/sv2v/sv2v-macOS/sv2v
f=$1; V=${2:-$S/s591/pre/vita}; TAG=${3:-pre}; d=$(dirname $f); b=$(basename $f .sv); cd $d
rm -rf obs_$b; { $V --obs-dir obs_$b $b.sv; echo "rc=$?"; } > $b.$TAG 2>&1
if [ -f obs_$b/run.json ]; then python3 - obs_$b/run.json > $b.$TAG.route <<'PY'
import json,sys
j=json.load(open(sys.argv[1]))
for s in (j.get("subroutines") or {}).get("items",[]):
    print(s.get("module"), s.get("name"), s.get("route"), s.get("sites"))
PY
fi
rm -rf obs_$b
[ "$TAG" != pre ] && exit 0
{ iverilog -g2012 -o $b.vvp $b.sv && vvp -n $b.vvp; echo "rc=$?"; } > $b.ivl 2>&1
rm -rf vl_$b; { verilator --binary --timing --assert -Wno-fatal --Mdir vl_$b -o sim $b.sv >/dev/null 2>vl_$b.build; brc=$?; if [ $brc -ne 0 ]; then grep -E '%Error' vl_$b.build | head -5; echo "build_rc=$brc"; else grep -E '%Warning' vl_$b.build | head -3; ./vl_$b/sim +verilator+error+limit+1000; echo "rc=$?"; fi; } > $b.vl 2>&1
{ $SV2V $b.sv > $b.s2v.v 2>$b.s2v.err && iverilog -g2012 -o $b.s2v.vvp $b.s2v.v && vvp -n $b.s2v.vvp; echo "rc=$?"; head -3 $b.s2v.err; } > $b.s2v 2>&1
rm -rf vl_$b vl_$b.build $b.vvp $b.s2v.vvp $b.s2v.err
