#!/bin/bash
# usage: run.sh <vita-binary> <tag> file.sv...   (file without _2s suffix; _2s twin used for verilator if present)
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s580
SV2V=$S/sv2v/sv2v-macOS/sv2v
VITA=$1; TAG=$2; shift 2
for f in "$@"; do
  b=${f%.sv}
  "$VITA" "$f" > $b.$TAG.txt 2>&1; echo "rc=$?" >> $b.$TAG.txt
  if [ "$TAG" = pre ] && [ "${NOORACLE:-0}" = 0 ]; then
    $SV2V "$f" > $b.sv2v.v 2> $b.sv2v.err; echo "sv2v_rc=$?" >> $b.sv2v.err
    iverilog -g2012 -o $b.sv2v.vvp $b.sv2v.v > $b.iv.txt 2>&1 && vvp -n $b.sv2v.vvp >> $b.iv.txt 2>&1; echo "rc=$?" >> $b.iv.txt
    vf=$f; [ -f ${b}_2s.sv ] && vf=${b}_2s.sv
    if [ "${NOVL:-0}" = 0 ]; then
      rm -rf obj_$b; verilator --binary --timing -Wno-fatal -Wno-lint -Wno-style --top-module t --Mdir obj_$b $vf > $b.vlc.txt 2>&1; echo "rc=$?" >> $b.vlc.txt
      if [ -x obj_$b/Vt ]; then ./obj_$b/Vt > $b.vl.txt 2>&1; else echo NOBIN > $b.vl.txt; fi
    fi
  fi
done
