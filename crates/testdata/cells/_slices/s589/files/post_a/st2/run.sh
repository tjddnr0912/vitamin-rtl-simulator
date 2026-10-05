#!/bin/bash
# run.sh <sepdir> <tag>: two CUs (t.sv = package q + top; c.sv = child c) compiled separately into one library
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
B=$1; T=$2; cd /private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s589/post_a/st2
rm -rf wl_$T; mkdir wl_$T
{ $B/vcmp --work work=wl_$T c.sv && $B/vcmp --work work=wl_$T t.sv && $B/velab -L work=wl_$T --top top -o t_$T.velab && $B/vrun t_$T.velab; echo "rc=$?"; } > out.$T 2>&1
rm -rf wl_$T t_$T.velab
