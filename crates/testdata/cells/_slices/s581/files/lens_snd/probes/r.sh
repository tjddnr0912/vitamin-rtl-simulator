#!/bin/bash
# r.sh cell.sv ... : PRE / POST-T / POST / iverilog / sv2v->iverilog / verilator, raw lines (no 'simulation ended')
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
SCR=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s581
SV2V=$SCR/../s580/sv2v/sv2v-macOS/sv2v
for f in "$@"; do
  d=$(cd $(dirname $f); pwd); b=$(basename $f .sv); cd $d
  echo "##### $b"
  for t in pre postT post; do
    o=$($SCR/$t/vita $b.sv 2>&1); rc=$?
    echo "[$t rc=$rc] $(echo "$o" | grep -v '^simulation ended' | sed 's/^/  /' | tr '\n' '|' | cut -c1-900)"
  done
  o=$(iverilog -g2012 -o $b.vvp $b.sv 2>&1 && vvp -n $b.vvp 2>&1); rc=$?
  echo "[iv rc=$rc] $(echo "$o" | grep -v 'finish called\|^$' | tr '\n' '|' | cut -c1-700)"
  o=$($SV2V $b.sv > $b.s.v 2>$b.s.err && iverilog -g2012 -o $b.s.vvp $b.s.v 2>&1 && vvp -n $b.s.vvp 2>&1; cat $b.s.err); rc=$?
  echo "[sv rc=$rc] $(echo "$o" | grep -v 'finish called\|^$' | tr '\n' '|' | cut -c1-700)"
  if [ -z "$NOVL" ]; then
    rm -rf obj_$b; verilator --binary --timing -Wno-fatal -Wno-lint -Wno-style --top-module top $b.sv -Mdir obj_$b > $b.vlb.txt 2>&1; r=$?
    if [ $r = 0 ]; then o=$(obj_$b/Vtop 2>&1); rc=$?; else o=$(grep -E '%Error' $b.vlb.txt | head -3); rc=$r; fi
    echo "[vl rc=$rc] $(echo "$o" | grep -v 'finish called\|^- \|^$' | tr '\n' '|' | cut -c1-700)"
  fi
done
