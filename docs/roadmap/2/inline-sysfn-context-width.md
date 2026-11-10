(§2 inline-sysfn-context-width · OPEN · in Inline / frame binds) (a `$bits` term in a 64-bit function body loses the context width):
* symptom: in a function body inlined at its call, a `$bits` leaf (a system call with no runtime id) turns the 64-bit context off, so `g * x` folds at 32 bits (`elaborate/src/inline_body_ctx.rs:154` `None => true`)
* witness: k1r_lit64 `function [63:0] fb(input [31:0] x); fb = g * x + $bits(x); endfunction` beside `fc`, the same with `$countones(x)`, `g = 32'hFFFFFFFF`, `$display("K1lit64 bits=%h cnt=%h", fb(32'hFFFFFFFF), fc(32'hFFFFFFFF));`: PRE 56c997be `K1lit64 bits=0000000000000021 cnt=fffffffe00000021` rc 0
* oracles: iverilog and verilator `K1lit64 bits=fffffffe00000021 cnt=fffffffe00000021` (W/batch2/docs/probe/SUMMARY_r1.txt)
* note: found by SYS-1a's build (W/sys1/build/NOTES.txt), kept byte-equal there (structure only); `$countones` maps to an id and keeps the context
* status: OPEN
