#!/bin/bash
# vl.sh cell.sv... : verilator --binary --timing --assert, run with +verilator+error+limit+1000
M=$(dirname "$0")
for f in "$@"; do
  b=$(basename "$f" .sv); d=$(cd "$(dirname "$f")" && pwd)
  rm -rf "$M/vlobj_$b"
  (cd "$d" && verilator --binary --timing --assert -Wno-fatal -Wno-lint -Wno-style --Mdir "$M/vlobj_$b" --top-module top "$b.sv" > "$M/vlc_$b.log" 2>&1); rc=$?
  echo "##### verilator $b.sv compile rc=$rc"
  if [ $rc -eq 0 ]; then (cd "$d" && "$M/vlobj_$b/Vtop" +verilator+error+limit+1000 2>&1 | grep -v '^- V e r\|^- Verilator:\|ignored due to +verilator+error+limit'); echo "run rc=${PIPESTATUS[0]}"; else grep -m5 '%Error' "$M/vlc_$b.log"; fi
done
