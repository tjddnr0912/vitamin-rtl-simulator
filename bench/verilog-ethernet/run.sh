#!/bin/sh
# verilog-ethernet eth_mac_1g GMII-loopback workload -- see RUN.md.
# Prefer `corpus-runner run --filter verilog-ethernet --compare`; this is the
# by-hand equivalent.
set -e
cd "$(dirname "$0")"
F="tb.v src/rtl/eth_mac_1g.v src/rtl/axis_gmii_rx.v src/rtl/axis_gmii_tx.v src/rtl/lfsr.v"
N=${N:-1000}
case "$1" in
  iverilog)  iverilog -g2012 -o verilog-ethernet.vvp $F && vvp verilog-ethernet.vvp +N=$N ;;
  vita)      ../../target/release/vita $F +N=$N ;;
  verilator) verilator --binary --timing -Wno-fatal -o v --top-module tb $F && ./obj_dir/v +N=$N ;;
  *) echo "usage: N=<frames> $0 {iverilog|vita|verilator}"; exit 2 ;;
esac
