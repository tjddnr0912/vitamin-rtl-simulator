#!/bin/bash
# usage: runall.sh <dir> [vita] [tag]  -> runs run4.sh on every .sv, 4-way parallel; writes <dir>.rc when done
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
G=$(dirname $0); d=$1; V=${2:-/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s592/pre/vita}; T=${3:-pre}
rm -f $d.rc
ls $d/*.sv | xargs -P 4 -I{} $G/run4.sh {} $V $T > $d.runlog 2>&1
echo $? > $d.rc
