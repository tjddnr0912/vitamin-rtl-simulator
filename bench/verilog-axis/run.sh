#!/bin/sh
# verilog-axis 4x4 axis_switch + four axis_fifo workload -- see RUN.md.
# Prefer `corpus-runner run --filter verilog-axis --compare`; this is the
# by-hand equivalent. iverilog is this row's oracle.
set -e
cd "$(dirname "$0")"
F="src/rtl/axis_switch.v src/rtl/axis_register.v src/rtl/arbiter.v src/rtl/priority_encoder.v src/rtl/axis_fifo.v tb.v"
N=${N:-3000}
case "$1" in
  iverilog) iverilog -g2012 -o verilog-axis.vvp $F && vvp verilog-axis.vvp +N=$N ;;
  vita)     ../../target/release/vita $F +N=$N ;;
  *) echo "usage: N=<frames per source> $0 {iverilog|vita}"; exit 2 ;;
esac
