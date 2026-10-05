#!/bin/bash
# stage.sh <cell.sv>: split package(s) and module into two CUs, vcmp each, velab -L, vrun
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
S=/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad
P=${2:-$S/s589/pre/sep}
f=$1; b=$(basename $f .sv); d=st_$b; rm -rf $d; mkdir -p $d/wl
awk '/^module /{m=1} {if(m) print > "'$d'/m.sv"; else print > "'$d'/p.sv"}' $f
cd $d
{ $P/vcmp --work work=wl p.sv; $P/vcmp --work work=wl m.sv; $P/velab -L work=wl --top top -o t.velab && $P/vrun t.velab; echo "rc=$?"; } 2>&1 | grep -v W1017 | grep -v '^errors=0' > ../$b.staged
cd ..; rm -rf $d
