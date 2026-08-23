#!/bin/sh
# biriscv workload bench -- pinned reproducer. Usage: ./run.sh [N_cycles]
# MUST be run with CWD = bench/biriscv/ (prog.hex is opened relative to CWD).
# Both tools are pinned to the SAME elaborate root (tb_top): the upstream
# wildcard also drags in biriscv_trace_sim, which is an uninstantiated second
# root. See RUN.md.
set -e
N=${1:-50000}
VITA=${VITA:-../../target/release/vita}
set -- $(cat files.txt)

iverilog -g2012 -s tb_top -I src/src/core -D TRACE=0 -o x.vvp "$@"
vvp x.vvp +N=$N

"$VITA" --top tb_top -I src/src/core -D TRACE=0 "$@" +N=$N
