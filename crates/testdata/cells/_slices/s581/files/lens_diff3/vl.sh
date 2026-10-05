#!/bin/bash
L=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s581/lens_diff3
id=$1; f=$(ls $L/v/$id.sv $L/v2/$id.sv $L/v4/$id.sv $L/v4/$id.sv $L/v4/$id.sv $L/v4/$id.sv $L/v4/$id.sv 2>/dev/null | head -1)
if verilator --binary --timing -Wno-fatal -Wno-lint -Wno-style --top-module top -Mdir $L/vl/$id $f > $L/vl/$id.log 2>&1; then
  out=$($L/vl/$id/Vtop 2>&1 | grep -E '^(R|GI|G)=' | sort | tr '\n' ';')
else out="VLERR: $(grep -m1 -E '%Error' $L/vl/$id.log)"; fi
echo "$id | verilator: $out"
