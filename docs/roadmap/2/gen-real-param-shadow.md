(§2 gen-real-param-shadow · BLOCKED · in Scoping / imports / block-locals) (a generate `real` parameter does not shadow an outer integral one):
* A generate `real` parameter does not shadow an outer integral one
* witness (BIND-1 plan F-b): p16_meta_walk_real_twin `localparam logic [3:0] N = 4'd7;` and in `g` `localparam real N = 20; logic [N-1:0] w;` printing `$bits(N), $bits(w), N`: PRE `A 4 4 20.000000`, the value from the inner twin and the type from the outer key (META-WALK, §5.2 BIND-1)
* p16: iverilog `A 1 20 20.000000`, verilator `%Error: t.sv:5:13: Value too wide for 32-bits expected in this context 19.0` (W/bind1/ground/probe/DIGEST.txt); pinned as PRE prints (`bind_pins.rs`)
* witness (batch 4 lens-diff S3, S4): b04_real_shadow_int_range `localparam int R = 4;` and in `g` `localparam real R = 2.6; logic [R:0] x;`: PRE = POST `A bx=5`; b33_real_shadow_wide_bits, a generate `real R = 0.5` over an outer 100-bit `R`: `A 100 0.500000`
* b04: iverilog and sv2v → iverilog `A bx=4`, verilator `%Error: t.sv:5:12: Value too wide for 32-bits expected in this context 2.6000000000000001`; b33: iverilog `A 1 0.500000`, verilator `A 64 0.500000`, sv2v `error: Invalid data type for $bits().`
* status: BLOCKED (declaring-scope fold, §4.5.562)
