#!/bin/bash
# usage: elab.sh <repo>   (runs from C-alex)
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/c40e1dc3-96e1-4997-a40d-90a56b79d219/scratchpad
C=$S/row9/C-alex; WD="perl $S/s2-review/sound/p3/wd.pl"; V=$S/accept/vita-final
r=$1; O=$C/elab/$r; mkdir -p $O
python3 -I -c "
import json; j=json.load(open('$C/$r.closure.json'))
for m,fs in j['closure'].items(): print(m, ' '.join(fs))
" | while read m fs; do
  files=""; for f in $fs; do files="$files $C/$r/rtl/$f"; done
  w1=$($WD 300 6000000 $O/$m.vita.out $V --timeout 1000 $files)
  w2=$($WD 300 6000000 $O/$m.iv.out iverilog -g2012 -s $m -o $O/$m.vvp $files)
  w3="-"; if [ -f $O/$m.vvp ]; then w3=$($WD 300 6000000 $O/$m.vvp.out vvp -n $O/$m.vvp); rm -f $O/$m.vvp; fi
  ne=$(grep -cE '^(\S+: )?(error|fatal)\[VITA' $O/$m.vita.out); nw=$(grep -cE 'warning\[VITA' $O/$m.vita.out)
  echo "$m | vita: $w1 err=$ne warn=$nw | iv: $w2 | vvp: $w3"
done
