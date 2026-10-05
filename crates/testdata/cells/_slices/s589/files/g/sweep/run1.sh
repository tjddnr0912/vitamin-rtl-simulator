#!/bin/bash
# run1.sh <cell.sv> <vita> <tag>: 20 s self-alarm watchdog
f=$1; V=$2; TAG=$3; d=$(dirname $f); b=$(basename $f .sv); cd $d
{ perl -e 'alarm 20; exec @ARGV' $V $b.sv; echo "rc=$?"; } > $b.$TAG 2>&1
