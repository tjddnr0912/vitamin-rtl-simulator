#!/bin/bash
# runb.sh cell... ; cell = X.sv (extra files in X.extra) or X (multi-file: X.files lists files in order)
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad
SV2V=$S/tools/sv2v-macOS/sv2v; F='^simulation ended\|^\s*$\|W1017\|^ *|\|^ *= \|^ *-->\|errors=\|VCD info\|CallStack\|error, called'
for c in "$@"; do
  d=$(pwd); b=${c%.sv}; w=$d/w_$b; rm -rf $w; mkdir -p $w
  if [ -f $b.files ]; then files=$(cat $b.files); else files=$c; fi
  cp $files $w/; [ -f $b.extra ] && cp $(cat $b.extra) $w/
  {
  for tag in pre post_a post_b; do V=$S/s591/$tag/vita
    (cd $w && $V $files > $tag.out 2>&1; echo "$tag rc=$?: $(grep -v "$F" $tag.out | head -2 | tr '\n' '|')")
  done
  (cd $w && $S/s591/post_b/vita vcmp $files -o st.vu > st.out 2>&1; r1=$?; r2=-; r3=-; if [ $r1 = 0 ]; then $S/s591/post_b/vita velab st.vu -o st.velab >> st.out 2>&1; r2=$?; [ $r2 = 0 ] && { $S/s591/post_b/vita vrun st.velab >> st.out 2>&1; r3=$?; }; fi; echo "post_b-staged $r1/$r2/$r3: $(grep -v "$F" st.out | head -2 | tr '\n' '|')")
  (cd $w && iverilog -g2012 -o a.out $files > ivl.out 2>&1 && vvp -n a.out >> ivl.out 2>&1; echo "IVL rc=$?: $(grep -v "$F\|finish called" ivl.out | head -2 | tr '\n' '|')")
  (cd $w && $SV2V $files > s2v.v 2> s2v.out && iverilog -g2012 -o b.out s2v.v >> s2v.out 2>&1 && vvp -n b.out >> s2v.out 2>&1; echo "S2V rc=$?: $(grep -v "$F\|finish called" s2v.out | head -2 | tr '\n' '|')")
  } > $d/$b.res
done
