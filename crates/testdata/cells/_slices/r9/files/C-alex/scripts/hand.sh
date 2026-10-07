#!/bin/bash
# hand.sh <tag> <top> <tbfile> "<-P/-G overrides as N=V ...>" <files...>
# runs iverilog (oracle first) then vita, compares full stdout minus tool banners
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/c40e1dc3-96e1-4997-a40d-90a56b79d219/scratchpad
C=$S/row9/C-alex; W=$C/scripts/wd; V=$S/accept/vita-final
tag=$1; top=$2; tb=$3; ov=$4; shift 4; O=$C/hand/out; mkdir -p $O
ip=""; vp=""; for kv in $ov; do ip="$ip -P$top.$kv"; vp="$vp -G $kv"; done
w0=$($W 300 6000000 $O/$tag.ivc.out iverilog -g2012 -s $top $ip -o $O/$tag.vvp $tb "$@")
w1=$($W 300 6000000 $O/$tag.iv.out vvp -n $O/$tag.vvp); rm -f $O/$tag.vvp
w2=$($W 300 6000000 $O/$tag.vita.out $V $vp $tb "$@")
grep -vE '^[^ ]+: \$finish called|^VCD info|^(warning|error|note)\[|^[^ ]+:[0-9]+:[0-9]+: (warning|error|note)|^simulation ended|^errors=' $O/$tag.iv.out > $O/$tag.iv.tr
grep -vE '^[^ ]+: \$finish called|^VCD info|^(warning|error|note)\[|^[^ ]+:[0-9]+:[0-9]+: (warning|error|note)|^simulation ended|^errors=' $O/$tag.vita.out > $O/$tag.vita.tr
if cmp -s $O/$tag.iv.tr $O/$tag.vita.tr; then res=MATCH; else res="DIVERGE first=$(diff $O/$tag.iv.tr $O/$tag.vita.tr | head -1)"; fi
echo "$tag | iv ${w1%% peak*} wall=$(echo $w1 | sed -E 's/.*wall_s=([0-9.]+).*/\1/') lines=$(wc -l < $O/$tag.iv.tr) | vita ${w2%% peak*} wall=$(echo $w2 | sed -E 's/.*wall_s=([0-9.]+).*/\1/') err=$(grep -cE '(error|fatal)\[' $O/$tag.vita.out) | $res | $(tail -1 $O/$tag.iv.tr | cut -c1-120)"
