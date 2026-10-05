#!/bin/bash
# usage: run.sh <cells...>   (writes <cell>.res next to each cell)
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad
PRE=$S/s591/pre/vita; POST=$S/s591/post_a/vita; SV2V=$S/tools/sv2v-macOS/sv2v
for f in "$@"; do
  b=$(basename $f .sv); d=$(dirname $f); w=$d/w_$b; rm -rf $w; mkdir -p $w; cp $f $w/t.sv
  {
  for tag in PRE POST; do
    bin=$PRE; [ $tag = POST ] && bin=$POST
    (cd $w && $bin t.sv > $tag.out 2>&1; echo $? > $tag.rc)
    echo "$tag rc=$(cat $w/$tag.rc): $(grep -v '^simulation ended\|^\s*$\|^ *|\|^ *= \|^ *-->' $w/$tag.out | head -3 | tr '\n' '|')"
  done
  (cd $w && iverilog -g2012 -o a.out t.sv > ivl.out 2>&1 && vvp -n a.out >> ivl.out 2>&1; echo $? > ivl.rc)
  echo "IVL rc=$(cat $w/ivl.rc): $(grep -v '^\s*$\|VCD info' $w/ivl.out | head -3 | tr '\n' '|')"
  (cd $w && $SV2V t.sv > s2v.v 2> s2v.out && iverilog -g2012 -o b.out s2v.v >> s2v.out 2>&1 && vvp -n b.out >> s2v.out 2>&1; echo $? > s2v.rc)
  echo "S2V rc=$(cat $w/s2v.rc): $(grep -v '^\s*$' $w/s2v.out | head -3 | tr '\n' '|')"
  } > $d/$b.res
done
