(§5.2 Q-MEMBIDXNOT · rank — · source Q-XFALLBACK review dr2 N-6) (a `~` index on a non-zero-base struct member reads x):
* slot: in Q-MEMBSRC: the same producer
* slot: parked with it
* a packed-struct member select whose index is a `~` reads x where the member's base is not 0, at run time and in a constant context: over `typedef struct packed { logic [15:8] a; logic [7:0] z; } h_t; h_t s = 16'h0302;` `s.a[~4'd6]` (bit 9) reads `x`, iverilog 13.0 and verilator 5.052 `1`, while `s.z[~4'd14]` (base 0), the plain twin `r[~4'd6]` over `logic [15:8] r` and `s.a[4'd9]` read 1 = both oracles (W/qxfb/runC/probe_n6 n6d.sv)
* the select arm: the parser's member remap `k - dbase` (`hdl-parser/src/scope.rs:170` `remap_pv_idx`) puts the index in a 32-bit subtraction, so `~` is taken at 32 bits (0xFFFFFFF9 − 8) instead of the index's self-determined 4 — measured by the base-0 control, the subtraction itself not traced
* also a `localparam` container (`P.a[~N +: 2]` over `P = 40'h0`, N = 4'd7: PRE `r=xx`, sv2v and verilator `00`; Q-XFALLBACK review dr2 N-6 ctl_tq08_noxP, ctl_tq21_noxN `P.a[~N]` with N = 4'd1 PRE `r=x`, verilator `0`), which makes tq08 / tq09 / tq21 print one oracle's line by coincidence
* Q-MEMBSRC's own witness `logic [P.a[~N]:0] d` (PRE 1, IEEE 2) is this read
* pre-existing
