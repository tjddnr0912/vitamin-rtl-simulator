#!/bin/sh
# OpenTitan primitives workload -- see RUN.md.
# Prefer `corpus-runner run --filter opentitan-prims`; this is the by-hand equivalent.
# verilator is this row's oracle: iverilog 13 cannot parse prim_cipher_pkg.sv.
set -e
cd "$(dirname "$0")"
F=$(cat files.txt)
N=${N:-32}
D="-DSYNTHESIS"
I="-Isrc/hw/ip/prim/rtl"
VL="--binary --timing --timescale 1ns/1ns -Wno-fatal -Wno-lint -Wno-style -Wno-MULTIDRIVEN -j 0 -o v --top-module tb"
case "$1" in
  vita)        ../../target/release/vita --top tb $D $I $F +N=$N ;;
  verilator)   verilator $VL $D $I $F && ./obj_dir/v +N=$N ;;
  # The oracle condition (RUN.md): the digest may not depend on x. Prints each distinct
  # result with its run count, so a single line means all 66 runs agreed.
  verilator-x) verilator $VL --x-assign unique --x-initial unique --Mdir obj_dir_x $D $I $F
               { for r in 0 1; do ./obj_dir_x/v +N=$N +verilator+rand+reset+$r; done
                 s=1
                 while [ $s -le 64 ]; do
                   ./obj_dir_x/v +N=$N +verilator+rand+reset+2 +verilator+seed+$s
                   s=$((s + 1))
                 done; } | grep -E 'DIGEST' | sort | uniq -c ;;
  # The 4-state cross-check: sv2v (github.com/zachjs/sv2v, v0.0.13) converts the design to
  # Verilog that iverilog runs. Not `-E Always`: that turns `always_comb` into `always @*`,
  # which does not run at time 0, so a block whose inputs never change stays x (RUN.md).
  sv2v)        sv2v $D $I --top=tb -w prims_sv2v.v $F
               iverilog -g2012 -s tb -o prims_sv2v.vvp prims_sv2v.v && vvp prims_sv2v.vvp +N=$N ;;
  iverilog)    iverilog -g2012 -s tb $D $I -o prims.vvp $F && vvp prims.vvp +N=$N ;;
  *) echo "usage: N=<vectors> $0 {vita|verilator|verilator-x|sv2v|iverilog}"; exit 2 ;;
esac
