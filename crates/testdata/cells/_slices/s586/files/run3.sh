#!/bin/bash
# usage: run3.sh <celldir> [extra iverilog args]   runs vita PRE, iverilog, verilator
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s586
VITA=${VITA:-$S/pre/vita}
d=$1; shift
cd $d || exit 9
files=$(ls *.sv | tr '\n' ' ')
$VITA $VITA_ARGS $files > vita.out 2> vita.err; echo $? > vita.rc
rm -f a.vvp
iverilog -g2012 "$@" -o a.vvp $files > iv.cout 2> iv.cerr; echo $? > iv.crc
if [ -f a.vvp ]; then vvp -n a.vvp > iv.out 2> iv.err; echo $? > iv.rc; fi
rm -rf obj_dir
verilator --binary --timing --assert -Wno-fatal $VL_ARGS $files > vl.cout 2> vl.cerr; echo $? > vl.crc
if [ -x obj_dir/V$(echo ${VL_TOP:-t}) ]; then ./obj_dir/V${VL_TOP:-t} +verilator+error+limit+1000 > vl.out 2> vl.err; echo $? > vl.rc; fi
