#!/bin/bash
# usage: TOOLS="vita iv sv vl" run.sh cell.sv [vita-extra-args]
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s582
SV2V=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s580/sv2v/sv2v-macOS/sv2v
f=$(cd "$(dirname "$1")"; pwd)/$(basename "$1"); shift; n=$(basename "$f" .sv); d=$(dirname "$f")/out_$n; mkdir -p $d; cd $d
TOOLS=${TOOLS:-"vita iv sv vl"}
echo "### $n"
for t in $TOOLS; do case $t in
vita) echo "--- vita(PRE) $*"; $S/pre/vita "$@" "$f" -o $d/w.vcd > vita.out 2>&1; echo "rc=$?" >> vita.out; grep -v '^$' vita.out | sed "s#$(dirname $f)/##; s/^/  /" | cut -c1-260 ;;
iv) echo "--- iverilog"; ( iverilog -g2012 -o iv.vvp "$f" && vvp -n iv.vvp ) > iv.out 2>&1; echo "rc=$?" >> iv.out; grep -v '^VCD\|^$' iv.out | sed "s#$(dirname $f)/##; s/^/  /" | head -30 ;;
sv) echo "--- sv2v->iverilog"; ( $SV2V "$f" > sv2v.v && iverilog -g2012 -o sv.vvp sv2v.v && vvp -n sv.vvp ) > sv.out 2>&1; echo "rc=$?" >> sv.out; grep -v '^VCD\|^$\|\$finish called' sv.out | sed "s#$(dirname $f)/##; s/^/  /" | head -40 ;;
vl) echo "--- verilator"; rm -rf obj_dir; ( verilator --binary --timing -Wno-fatal -Wno-lint -Wno-style --top-module top "$f" -Mdir obj_dir > vl_build.log 2>&1 || { grep -i 'error' vl_build.log | head -8; exit 9; }; ./obj_dir/Vtop ) > vl.out 2>&1; echo "rc=$?" >> vl.out; grep -v '^$\|^- \|Verilog \$finish' vl.out | sed "s#$(dirname $f)/##; s/^/  /" | head -40 ;;
esac; done
