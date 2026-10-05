#!/bin/bash
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s580
NEW=$1
for f in S*.sv; do b=${f%.sv}
  for t in pre post; do $S/$t/vita $f > $b.$t.txt 2>&1; echo rc=$? >> $b.$t.txt; done
  $NEW $f > $b.new.txt 2>&1; echo rc=$? >> $b.new.txt
  if grep -q inside $f; then $S/sv2v/sv2v-macOS/sv2v $f > $b.v 2>$b.sv2v.err && iverilog -g2012 -o $b.vvp $b.v > $b.iv.txt 2>&1 && vvp -n $b.vvp >> $b.iv.txt 2>&1 || cat $b.sv2v.err > $b.iv.txt
  else iverilog -g2012 -o $b.vvp $f > $b.iv.txt 2>&1 && vvp -n $b.vvp >> $b.iv.txt 2>&1; fi
  rm -rf obj_$b; verilator --binary --timing -Wno-fatal -Wno-lint -Wno-style --top-module t --Mdir obj_$b $f > $b.vlc.txt 2>&1 && ./obj_$b/Vt > $b.vl.txt 2>&1 || { grep -m1 '%Error' $b.vlc.txt > $b.vl.txt; }
done
