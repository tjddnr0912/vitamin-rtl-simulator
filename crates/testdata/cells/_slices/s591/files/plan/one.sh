#!/bin/bash
# one.sh <cell.sv>: PRE+oracles then proto2
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad
$S/s591/g/run4.sh "$1" $S/s591/pre/vita pre
$S/s591/g/run4.sh "$1" $S/s591/proto2/vita post2
