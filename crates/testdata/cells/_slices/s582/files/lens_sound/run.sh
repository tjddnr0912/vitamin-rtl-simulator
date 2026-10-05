#!/bin/bash
# usage: TOOLS="pre post sv vl iv" run.sh cell.sv
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s582
SV2V=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s580/sv2v/sv2v-macOS/sv2v
f=$(cd "$(dirname "$1")"; pwd)/$(basename "$1"); n=$(basename "$f"); n=${n%.*}; d=$(dirname "$f")/out_$n; mkdir -p $d; cd $d
TOOLS=${TOOLS:-"pre post sv vl"}
echo "### $n"
for t in $TOOLS; do case $t in
pre|post|post2) echo "--- vita($t)"; $S/$t/vita "$f" -o $d/$t.vcd > $t.out 2>&1; echo "rc=$?" >> $t.out; grep -v '^$' $t.out | sed "s#$(dirname $f)/##; s/^/  /" | cut -c1-300 | head -30 ;;
iv) echo "--- iverilog(default gen)"; ( iverilog -o iv.vvp "$f" && vvp -n iv.vvp ) > iv.out 2>&1; echo "rc=$?" >> iv.out; grep -v '^VCD\|^$' iv.out | sed "s#$(dirname $f)/##; s/^/  /" | head -20 ;;
iv12) echo "--- iverilog -g2012"; ( iverilog -g2012 -o iv12.vvp "$f" && vvp -n iv12.vvp ) > iv12.out 2>&1; echo "rc=$?" >> iv12.out; grep -v '^VCD\|^$' iv12.out | sed "s#$(dirname $f)/##; s/^/  /" | head -20 ;;
sv) echo "--- sv2v->iverilog"; ( $SV2V "$f" > sv2v.v && iverilog -g2012 -o sv.vvp sv2v.v && vvp -n sv.vvp ) > sv.out 2>&1; echo "rc=$?" >> sv.out; grep -v '^VCD\|^$\|\$finish called' sv.out | sed "s#$(dirname $f)/##; s/^/  /" | head -30 ;;
vl) echo "--- verilator"; rm -rf obj_dir; ( verilator --binary --timing -Wno-fatal -Wno-lint -Wno-style --top-module top "$f" -Mdir obj_dir > vl_build.log 2>&1 || { grep -i 'error' vl_build.log | head -6; exit 9; }; ./obj_dir/Vtop ) > vl.out 2>&1; echo "rc=$?" >> vl.out; grep -v '^$\|^- \|Verilog \$finish' vl.out | sed "s#$(dirname $f)/##; s/^/  /" | head -30 ;;
esac; done
