#!/bin/sh
# verilog-i2c master + slave workload -- see RUN.md.
# Prefer `corpus-runner run --filter verilog-i2c --compare`; this is the
# by-hand equivalent. iverilog is this row's oracle. TRACE=1 adds +TRACE.
set -e
cd "$(dirname "$0")"
F="src/rtl/i2c_master.v src/rtl/i2c_slave.v tb.v"
N=${N:-900000}
T=""; [ -n "${TRACE:-}" ] && T="+TRACE"
case "$1" in
  iverilog) iverilog -g2012 -o verilog-i2c.vvp $F && vvp verilog-i2c.vvp +N=$N $T ;;
  vita)     ../../target/release/vita $F +N=$N $T ;;
  *) echo "usage: [TRACE=1] N=<cycles> $0 {iverilog|vita}"; exit 2 ;;
esac
