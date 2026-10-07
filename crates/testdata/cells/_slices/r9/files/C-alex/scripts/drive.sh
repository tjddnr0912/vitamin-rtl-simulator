#!/bin/bash
# usage: drive.sh <repo> <module> <tag> <params-json|-> <ncyc>
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/c40e1dc3-96e1-4997-a40d-90a56b79d219/scratchpad
C=$S/row9/C-alex; WD="perl $S/s2-review/sound/p3/wd.pl"; V=$S/accept/vita-final
r=$1; m=$2; tag=$3; pj=$4; n=$5; O=$C/run/$r/$m; mkdir -p $O
fs=$(python3 -I -c "import json; print(' '.join(json.load(open('$C/$r.closure.json'))['closure']['$m']))")
files=""; for f in $fs; do files="$files $C/$r/rtl/$f"; done
g=$(python3 -I $C/scripts/gen_tb.py $C/$r/rtl $m "$pj" $O $tag $n $files)
if [[ "$g" != OK* ]]; then echo "$m/$tag | GEN: $g" | head -5; exit 0; fi
w2=$($WD 300 6000000 $O/$tag.ivc.out iverilog -g2012 -s tb -o $O/$tag.vvp $O/tb_$tag.v $files)
w3="-"; [ -f $O/$tag.vvp ] && w3=$($WD 300 6000000 $O/$tag.iv.out vvp -n $O/$tag.vvp) && rm -f $O/$tag.vvp
w1=$($WD 300 6000000 $O/$tag.vita.out $V $O/tb_$tag.v $files)
grep -E '^[0-9]+( |$)' $O/$tag.iv.out > $O/$tag.iv.tr; grep -E '^[0-9]+( |$)' $O/$tag.vita.out > $O/$tag.vita.tr
ni=$(wc -l < $O/$tag.iv.tr); nv=$(wc -l < $O/$tag.vita.tr)
if cmp -s $O/$tag.iv.tr $O/$tag.vita.tr; then res=MATCH; else res="DIVERGE first=$(diff $O/$tag.iv.tr $O/$tag.vita.tr | head -1)"; fi
ne=$(grep -cE '(error|fatal)\[VITA' $O/$tag.vita.out); nw=$(grep -cE 'warning\[VITA' $O/$tag.vita.out)
echo "$m/$tag | vita: ${w1%% peak*} vw=$(echo $w1 | sed -E "s/.*wall_s=([0-9.]+).*/\1/") iw=$(echo $w3 | sed -E "s/.*wall_s=([0-9.]+).*/\1/") err=$ne warn=$nw lines=$nv | iv: ${w3%% peak*} lines=$ni | $res"
