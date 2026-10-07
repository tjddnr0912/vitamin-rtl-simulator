#!/bin/sh
# verilog-uart loopback workload -- see RUN.md.
# Prefer `corpus-runner run --filter verilog-uart --compare`; this is the
# by-hand equivalent. iverilog is this row's oracle. TRACE=1 adds +TRACE.
set -e
cd "$(dirname "$0")"
F="src/rtl/uart.v src/rtl/uart_rx.v src/rtl/uart_tx.v tb.v"
N=${N:-20000}
T=""; [ -n "${TRACE:-}" ] && T="+TRACE"
case "$1" in
  iverilog) iverilog -g2012 -o verilog-uart.vvp $F && vvp verilog-uart.vvp +N=$N $T ;;
  vita)     ../../target/release/vita $F +N=$N $T ;;
  *) echo "usage: [TRACE=1] N=<bytes> $0 {iverilog|vita}"; exit 2 ;;
esac
