#!/bin/bash
# wl.sh <name> <pkgfile> <modfile>: work-library staged (separate vcmp units) for PRE and POST -> <name>.prewl / .postwl
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad
cd $S/s591/r1/diff/cells
for t in pre post_a; do tag=${t%_a}wl; L=wl_$1_$t; rm -rf $L; mkdir $L
 { $S/s591/$t/sep/vcmp --work L=$L $2 && $S/s591/$t/sep/vcmp --work L=$L $3 && $S/s591/$t/sep/velab -L L=$L --top top -o $L/x.velab && $S/s591/$t/sep/vrun $L/x.velab; echo "rc=$?"; } > $1.$tag 2>&1
 rm -rf $L; done
