#!/bin/bash
# runall2.sh <bin> <tag>: vita-only run over every s594 set + probes + round-2 + lens cells
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad
G=$S/s594/g; A=$S/s594/a; I=$S/s594/i
python3 $G/runv.py $1 $2 $G/c1 $G/c2 $G/c3 $G/c4 $G/old/* $A/cells $A/cells2 $A/cells3 $A/cells4 $A/cells5 $A/ivd $I/probe $I/dump $I/pin $I/r2 $I/r2/lens/* > $I/all2.$2.log 2>&1
echo $? > $I/all2.$2.rc
