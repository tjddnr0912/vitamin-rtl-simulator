#!/bin/sh
# secworks/aes @ 80dc4718e1dcbbdb4b0dd1bdb393d8f7b98981dc (BSD-2-Clause) -- see RUN.md
# N=200 -> iverilog median 6.18 s / vita median 3.53 s. DIGEST=cfaa46dd896b2275ade662d344f5e251
#
# NOTE: vita EXITS 1 here while printing the CORRECT digest (9x VITA-E4002 on a
# real out-of-range read in aes_key_mem.v; iverilog/verilator accept it silently).
# Gate on the DIGEST line, NOT on the exit code.
#
# NOTE: this file is #!/bin/sh on purpose -- zsh does not word-split $F.
D=$(cd "$(dirname "$0")" && pwd)
F="$D/src/src/rtl/aes_core.v $D/src/src/rtl/aes_encipher_block.v $D/src/src/rtl/aes_decipher_block.v $D/src/src/rtl/aes_key_mem.v $D/src/src/rtl/aes_sbox.v $D/src/src/rtl/aes_inv_sbox.v $D/tb.v"
N=${N:-200}
case "$1" in
  iverilog)  iverilog -g2012 -o "$D/x.vvp" $F && vvp "$D/x.vvp" +N=$N ;;
  vita)      "$D/../../target/release/vita" $F +N=$N ;;
  verilator) verilator --binary --timing -Wno-fatal -o vsim --top-module tb $F && ./obj_dir/vsim +N=$N ;;
  *) echo "usage: N=<count> $0 {iverilog|vita|verilator}"; exit 2 ;;
esac
