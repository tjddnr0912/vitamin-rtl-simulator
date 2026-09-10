#!/bin/sh
# servant SoC (SERV bit-serial RISC-V) workload -- see RUN.md.
# Prefer `corpus-runner run --filter serv --compare`; this is the by-hand equivalent.
set -e
cd "$(dirname "$0")"
F=$(cat files.txt)
N=${N:-500000}
case "$1" in
  iverilog)  iverilog -g2012 -s tb -o serv.vvp $F && vvp serv.vvp +N=$N ;;
  vita)      ../../target/release/vita --top tb $F +N=$N ;;
  *) echo "usage: N=<cycles> $0 {iverilog|vita}"; exit 2 ;;
esac
