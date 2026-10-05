#!/bin/bash
# stage1.sh <cell.sv> [sepdir] [tag]: one-CU staged run (vcmp -> velab -> vrun); output <cell>.<tag>
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad
P=${2:-$S/s591/pre/sep}; TAG=${3:-stg}
f=$1; d=$(dirname $f); b=$(basename $f .sv); cd $d
{ $P/vcmp $b.sv -o $b.$TAG.vu && $P/velab $b.$TAG.vu -o $b.$TAG.velab && $P/vrun $b.$TAG.velab; echo "rc=$?"; } > $b.$TAG 2>&1
rm -f $b.$TAG.vu $b.$TAG.velab
