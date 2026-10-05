#!/bin/bash
# run2.sh <dir> : run PRE (pre/sep/vita) and POST (post_b/vita) on every .sv in dir -> <b>.PRE/.POST + obsPRE_/obsPOST_
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s587
cd $1; for f in *.sv; do b=${f%.sv}
 rm -rf obsPRE_$b obsPOST_$b
 { $S/pre/sep/vita --obs-dir obsPRE_$b $f; echo "rc=$?"; } > $b.PRE 2>&1
 { $S/post_b/vita --obs-dir obsPOST_$b $f; echo "rc=$?"; } > $b.POST 2>&1
done
