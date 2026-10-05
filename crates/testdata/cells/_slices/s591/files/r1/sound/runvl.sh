#!/bin/bash
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
for f in "$@"; do
  b=$(basename $f .sv); d=$(dirname $f); w=$d/w_$b; mkdir -p $w/vl; cp $f $w/vl/t.sv
  (cd $w/vl && verilator --binary --timing -Wno-fatal --Mdir obj t.sv > vl.out 2>&1 && ./obj/Vt >> vl.out 2>&1; echo $? > vl.rc)
  echo "VL rc=$(cat $w/vl/vl.rc): $(grep -v '^\s*$\|^- \|^%Warning\|^ *:\|^ *|\|Verilator\|^make\|^g\?cc\|^clang\|^ar \|^ranlib\|^echo\|^rm \|^python\|^\./\|^- V' $w/vl/vl.out | grep -v 'obj/' | head -3 | tr '\n' '|')" > $d/$b.vl
done
