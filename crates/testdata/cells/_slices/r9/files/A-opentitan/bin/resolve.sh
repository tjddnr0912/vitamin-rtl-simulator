#!/bin/sh
# usage: resolve.sh <top> <outdir>  -> outdir/vl_json.log, outdir/files.raw (verilator-resolved file list)
. /private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/c40e1dc3-96e1-4997-a40d-90a56b79d219/scratchpad/row9/A-opentitan/bin/env.sh
top=$1; out=$2; mkdir -p $out
rm -rf $out/vj
$R/bin/lock.sh $WD 1200 10000000 $out/vl_json.log /opt/homebrew/bin/verilator --json-only --json-only-meta-output $out/vj/meta.json --json-only-output $out/vj/tree.json -Wno-fatal -Wno-lint -Wno-style --top-module $top $DEFS $INCS $YDIRS +libext+.sv --Mdir $out/vj $EXTRA_FILES > $out/vl_json.wd 2>&1
cat $out/vl_json.wd
