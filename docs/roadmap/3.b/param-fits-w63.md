(§3.b param-fits-w63 · OPEN · in Constants / parameters) (a 63-bit unsigned parameter fits no value in the parser's check):
* symptom: `const_param_fits` (moved by PARSE-1 into `hdl-parser/src/const_fold.rs` from `params.rs:968`) tests `v < (1i64 << 63)` at w = 63 unsigned, which is `v < i64::MIN`, so no value fits
* witness: fits63b `localparam logic [62:0] P = 5000000000;` `typedef struct packed { logic [P - 4999999995 : 0] a; logic b; } t_t;` `$display("L w=%0d", $bits(t_t));`: PRE 56c997be E2002 at `t.sv:3:27`, rc 1; fits62b (`[61:0]`) `L w=7`
* oracles: iverilog `L w=7` on both; verilator refuses both (`%Error: t.sv:3:36: Value too wide for 32-bits expected in this context 63'h7ffffffe00000005`); by hand, 5000000000 fits 63 bits (W/batch2/docs/probe/SUMMARY_r1.txt)
* note: PARSE-1 premise E9, a type-range spelling (out O11 of §5.2 PARSE-1), kept byte-equal by PARSE-1
* status: OPEN
