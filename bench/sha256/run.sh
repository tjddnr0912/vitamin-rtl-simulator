#!/bin/sh
# bench/sha256 — secworks/sha256 @ 837c5cc396f001d18f2c765721c585716eb439ae (BSD-2-Clause)
# top = tb ; workload dialled with +N=<blocks> (default 2000)
# Expected DIGEST (N=2000): e75e29e81cff3c66de9e0f419baa516ea08e6414fa1f9f62a757538288351724
set -e
D=$(cd "$(dirname "$0")" && pwd)
FILES="$D/tb.v $D/src/src/rtl/sha256_core.v $D/src/src/rtl/sha256_w_mem.v $D/src/src/rtl/sha256_k_constants.v"
N=${N:-2000}
case "$1" in
  iverilog) iverilog -g2012 -o "$D/x.vvp" $FILES && vvp "$D/x.vvp" +N=$N ;;
  verilator) verilator --binary --timing -Wno-fatal -o vsim $FILES && "$D/obj_dir/vsim" +N=$N ;;
  *) "$D/../../target/release/vita" $FILES +N=$N ;;
esac
