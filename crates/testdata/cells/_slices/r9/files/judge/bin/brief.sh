#!/bin/bash
J=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/c40e1dc3-96e1-4997-a40d-90a56b79d219/scratchpad/row9/judge
for id in "$@"; do d=$J/runs/$id; echo "## $id"
 v=$(grep -v -E 'W1017|^WD|^errors=|simulation ended|^\s*$' $d/vita.out | head -2 | sed "s#$d/##g" | cut -c1-330); vw=$(grep '^WD' $d/vita.out | awk '{print $2}')
 echo "  vita[$vw]: $v" | tr '\n' ' '; echo
 ic=$(grep -v -E '^WD|^\s*$' $d/iv.c.out 2>/dev/null | head -1 | cut -c1-200); io=$(grep -v -E '^WD|\$finish called|^\s*$' $d/iv.out 2>/dev/null | head -2 | tr '\n' ' ' | cut -c1-200)
 echo "  iv: ${ic:+COMPILE: $ic }${io}"
 sc=$(grep -v -E '^WD|^\s*$' $d/s2v.c.out 2>/dev/null | head -1 | cut -c1-200); so=$(grep -v -E '^WD|\$finish called|^\s*$' $d/s2v.out 2>/dev/null | head -2 | tr '\n' ' ' | cut -c1-200)
 echo "  s2v: ${sc:+COMPILE: $sc }${so}"
 lc=$(grep -E '%Error' $d/vl.c.out 2>/dev/null | head -1 | sed "s#$d/##g" | cut -c1-200); lo=$(grep -v -E '^WD|^- |^\s*$' $d/vl.out 2>/dev/null | head -2 | tr '\n' ' ' | cut -c1-200)
 echo "  vl: ${lc:+COMPILE: $lc }${lo}"
done
