#!/bin/sh
# lowRISC Ibex workload -- see RUN.md.
# Prefer `corpus-runner run --filter ibex`; this is the by-hand equivalent.
# verilator is this row's oracle: iverilog 13 cannot parse ibex_pkg.sv.
set -e
cd "$(dirname "$0")"
F=$(cat files.txt)
N=${N:-20000}
D="-DSYNTHESIS -DDV_FCOV_DISABLE"
I="-Isrc/vendor/lowrisc_ip/ip/prim/rtl -Isrc/vendor/lowrisc_ip/dv/sv/dv_utils -Isrc/rtl"
VL="--binary --timing -Wno-fatal -Wno-lint -Wno-style -j 0 -o v --top-module tb"
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
                 done; } | grep -E 'DIGEST|WATCHDOG' | sort | uniq -c ;;
  # The 4-state cross-check: sv2v (github.com/zachjs/sv2v, v0.0.13) converts the design to
  # Verilog that iverilog can run. iverilog rejects the attribute sv2v leaves after
  # `always_comb`, so it is stripped first.
  sv2v)        sv2v -E Always $D $I --top=tb -w ibex_sv2v.v $F
               perl -pi -e 's/\(\* full_case, parallel_case \*\)//g' ibex_sv2v.v
               iverilog -g2012 -s tb -o ibex_sv2v.vvp ibex_sv2v.v && vvp ibex_sv2v.vvp +N=$N ;;
  iverilog)    iverilog -g2012 -s tb $D $I -o ibex.vvp $F && vvp ibex.vvp +N=$N ;;
  *) echo "usage: N=<iterations> $0 {vita|verilator|verilator-x|sv2v|iverilog}"; exit 2 ;;
esac
