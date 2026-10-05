#!/bin/bash
# usage: TOOLS="post pre sv vl" run.sh <post-binary> cell.sv ...
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s582
SV2V=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s580/sv2v/sv2v-macOS/sv2v
POST=$1; shift
TOOLS=${TOOLS:-"post pre sv vl"}
for f0 in "$@"; do
f=$(cd "$(dirname "$f0")"; pwd)/$(basename "$f0"); n=$(basename "$f" .sv); d=$(dirname "$f")/out/$n; mkdir -p $d; cd $d
for t in $TOOLS; do case $t in
post) $POST "$f" -o $d/w.vcd > post.out 2>&1; echo "rc=$?" >> post.out ;;
pre) $S/pre/vita "$f" -o $d/w.vcd > pre.out 2>&1; echo "rc=$?" >> pre.out ;;
sv) ( $SV2V "$f" > sv2v.v && iverilog -g2012 -o sv.vvp sv2v.v && vvp -n sv.vvp ) > sv.out 2>&1; echo "rc=$?" >> sv.out ;;
vl) rm -rf obj_dir; ( verilator --binary --timing -Wno-fatal -Wno-lint -Wno-style --top-module t "$f" -Mdir obj_dir > vl_build.log 2>&1 || { grep -i 'error' vl_build.log | head -8; exit 9; }; ./obj_dir/Vt ) > vl.out 2>&1; echo "rc=$?" >> vl.out ;;
esac; done
done
