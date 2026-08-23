#!/bin/sh
# verilog-axi bench: 2x2 axi_crossbar + 2 axi_ram, two synthetic AXI masters.
# usage: ./run.sh [N]   (N = transactions per master, default 5000)
set -e
D=$(cd "$(dirname "$0")" && pwd)
R="$D/src/rtl"
N=${1:-5000}
FILES="$R/axi_crossbar.v $R/axi_crossbar_rd.v $R/axi_crossbar_wr.v \
$R/axi_crossbar_addr.v $R/axi_register_rd.v $R/axi_register_wr.v \
$R/arbiter.v $R/priority_encoder.v $R/axi_ram.v $D/tb.v"
echo "== iverilog =="
iverilog -g2012 -o "$D/x.vvp" $FILES
time vvp "$D/x.vvp" +N=$N | grep -E '^(OPS|D0|CYCD|DIGEST|WATCHDOG)'
echo "== vita =="
time vita $FILES +N=$N || echo "vita exit=$?"
