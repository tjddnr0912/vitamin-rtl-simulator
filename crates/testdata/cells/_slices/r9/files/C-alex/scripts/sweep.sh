#!/bin/bash
# sweep.sh <repo>: KICK=1 drive of every module x {def,v1,v2,v3}
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/c40e1dc3-96e1-4997-a40d-90a56b79d219/scratchpad
C=$S/row9/C-alex; r=$1; export KICK=1
for m in $(python3 -I -c "import json; print(' '.join(json.load(open('$C/$r.closure.json'))['closure']))"); do
  f=$C/$r/rtl/$(python3 -I -c "import json; print(json.load(open('$C/$r.closure.json'))['closure']['$m'][0])")
  $C/scripts/drive.sh $r $m kdef - 2000
  for v in v1 v2 v3; do
    pj=$(python3 -I $C/scripts/variants.py $f $m $v)
    [ -z "$pj" ] && continue
    echo "$pj" > $C/run/$r/$m/k$v.params
    $C/scripts/drive.sh $r $m k$v "$pj" 2000
  done
done
