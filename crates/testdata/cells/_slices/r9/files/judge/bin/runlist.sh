#!/bin/bash
J=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/c40e1dc3-96e1-4997-a40d-90a56b79d219/scratchpad/row9/judge
L=$1; rm -f $L.rc
while read id src top; do $J/bin/run4.sh $id $src $top; echo "$id done" >> $L.progress; done < $L
echo 0 > $L.rc
