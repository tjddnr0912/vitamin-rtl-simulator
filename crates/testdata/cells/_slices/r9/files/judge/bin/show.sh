#!/bin/bash
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/c40e1dc3-96e1-4997-a40d-90a56b79d219/scratchpad
d=$S/row9/judge/runs/$1; N=${2:-12}
for k in vita iv.c iv s2v.c s2v vl.c vl; do [ -f $d/$k.out ] || continue
  if [ $k = vl.c ]; then L=$(grep -E '%Error|%Warning|^WD|LOCK' $d/$k.out | grep -v -E 'Warning-(UNUSED|DECLFILENAME|WIDTH|UNDRIVEN|PINCONNECTEMPTY|MULTITOP|UNOPTFLAT|EOFNEWLINE)' | head -$N);
  elif [ $k = vl ]; then L=$(grep -v -E 'S i m u l a t i o n|^\s*$' $d/$k.out | head -$N);
  else L=$(grep -v -E 'W1017|^\s*$' $d/$k.out | head -$N); fi
  echo "--- $k"; echo "$L" | sed "s#$d/##g"; done
