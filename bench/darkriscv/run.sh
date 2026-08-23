#!/bin/sh
# darkriscv bench — run from bench/darkriscv/src/sim (upstream relative
# `include "../rtl/config.vh"` and $readmemh("../src/darksocv.mem")).
# Core harness (vita-clean):
#   iverilog -g2012 -DSIMULATION=1 -D__WAITSTATE__=7 -I ../rtl -o core.vvp \
#            ../../tb2.v ../rtl/darkriscv.v ../rtl/darkram.v && vvp core.vvp +N=300000
#   vita --top tb2 -DSIMULATION=1 -D__WAITSTATE__=7 -I ../rtl \
#        ../../tb2.v ../rtl/darkriscv.v ../rtl/darkram.v +N=300000
# Full SoC harness (vita REFUSES — see notes):
#   ... ../../tb.v ../rtl/darksocv.v ../rtl/darkbridge.v ../rtl/darkuart.v \
#       ../rtl/darkriscv.v ../rtl/darkpll.v ../rtl/darkram.v ../rtl/darkio.v \
#       ../rtl/darkcache.v ../rtl/darkmac.v
