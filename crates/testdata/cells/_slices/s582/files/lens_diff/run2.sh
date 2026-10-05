#!/bin/bash
# usage: run.sh design.(sv|v) [vita-args]   TOOLS="pre post iv sv vl"
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s582
SV2V=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s580/sv2v/sv2v-macOS/sv2v
f=$(cd "$(dirname "$1")"; pwd)/$(basename "$1"); shift; ext=${f##*.}; n=$(basename "$f" .$ext); d=$(dirname "$f")/out_$n; mkdir -p $d; cd $d
if [ "$ext" = v ]; then IVG=-g2005; VLL="--default-language 1364-2005"; DEF="pre post iv vl"; else IVG=-g2012; VLL=""; DEF="pre post iv sv vl"; fi
TOOLS=${TOOLS:-$DEF}
echo "### $n.$ext"
filt() { grep -v '^$' | sed "s#$(dirname $f)/##g; s/^/  /" | cut -c1-300 | head -${1:-25}; }
for t in $TOOLS; do case $t in
pre|post|post2) echo "--- $t $*"; $S/$t/vita "$@" "$f" -o $d/$t.vcd > $t.out 2>&1; echo "rc=$?" >> $t.out; filt < $t.out ;;
iv) echo "--- iverilog $IVG"; ( iverilog $IVG -o iv.vvp "$f" && vvp -n iv.vvp ) > iv.out 2>&1; echo "rc=$?" >> iv.out; grep -v '^VCD' iv.out | filt ;;
sv) echo "--- sv2v->iverilog"; ( $SV2V "$f" > sv2v.v && iverilog -g2012 -o sv.vvp sv2v.v && vvp -n sv.vvp ) > sv.out 2>&1; echo "rc=$?" >> sv.out; grep -v '^VCD\|\$finish called' sv.out | filt ;;
vl) echo "--- verilator $VLL"; rm -rf obj_dir; ( verilator --binary --timing $VLL -Wno-fatal -Wno-lint -Wno-style --top-module top "$f" -Mdir obj_dir > vl_build.log 2>&1 || { grep -i 'error' vl_build.log | head -6; exit 9; }; ./obj_dir/Vtop ) > vl.out 2>&1; echo "rc=$?" >> vl.out; grep -v '^- \|Verilog \$finish' vl.out | filt ;;
esac; done
