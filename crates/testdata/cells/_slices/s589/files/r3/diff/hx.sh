#!/bin/bash
# hx.sh <tag> <bindir> <cell.sv>...: one-shot (.<tag>) + staged one-CU (.<tag>s)
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
T=$1; D=$2; shift 2
for f in "$@"; do d=$(dirname $f); b=$(basename $f .sv); cd $d
 { perl -e 'alarm 20; exec @ARGV' $D/vita $b.sv; echo "rc=$?"; } > $b.$T 2>&1
 { perl -e 'alarm 20; exec @ARGV' $D/sep/vcmp $b.sv -o $b.$T.vu && perl -e 'alarm 20; exec @ARGV' $D/sep/velab $b.$T.vu -o $b.$T.velab && perl -e 'alarm 20; exec @ARGV' $D/sep/vrun $b.$T.velab; echo "rc=$?"; } > $b.${T}s 2>&1; rm -f $b.$T.vu $b.$T.velab
done
