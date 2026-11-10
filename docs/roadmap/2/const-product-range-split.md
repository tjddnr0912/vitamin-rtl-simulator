(§2 const-product-range-split · in Oracle splits (recorded, not chased)) (a range bound over a product that overflows 64 bits):
* is_range `logic [((3037000500 * 3037000500) % 1000) - 190 : 0] x;` `$display("L w=%0d", $bits(x));`: PRE 56c997be `L w=1` rc 0, iverilog `L w=191`, verilator `L w=3`; with 32-bit operands (is_range_ok, `30370005`) all three `L w=598` (W/batch2/docs/probe/SUMMARY_r1.txt)
* note: PARSE-1 premise E6: the three tools fold the overflowing product apart, and vita's 1 bit agrees with neither; kept byte-equal by PARSE-1
