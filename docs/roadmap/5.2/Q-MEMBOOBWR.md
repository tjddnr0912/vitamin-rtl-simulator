(§5.2 Q-MEMBOOBWR · rank ② · source row 24 plan Q4, design review R8 (g1.bound_diag_oobw_lit, pp.mrelw_wr)) (a struct member range write with a bit outside the member is refused):
* slot: after Q-MEMBSRC
* symptom: a literal packed-struct member range write with a bit outside the member is E2002 at parse, `expected a constant in-bounds packed-struct member range WRITE in the member's declared direction` (`hdl-parser/src/struct_sel.rs:895`), where IEEE 1800-2017 §11.5.1 (ieee1800-2017.txt:12386–12387) writes the in-range bits: `typedef struct packed { logic [7:0] a; logic [3:0] s; } h_t; h = '0; h.a[9:6] = 4'hF;` iverilog = sv2v → iverilog = verilator `A h=c00`
* oracles: with a member above `a` (`{logic [3:0] t; logic [7:0] a; logic [3:0] s}`), `h.a[8:5] = 4'hf` IEEE `0e00`, iverilog = sv2v → iverilog `1e00` (bit 8 written into `t`)
* pins: the plain twin `logic [7:0] v; v[9:6] = 4'hF;` writes the in-range bits (`c0`, all three), so only the member path refuses
* symptom: a reversed literal member range write is refused the same way inside a dead arm: `typedef struct packed { logic [7:0] a; logic [3:0] s; } h_t; localparam int P = 6; h = 12'h2d5; if (P == 0) h.a[0:5] = 6'h3f;` is E2002 with the same text, where iverilog = sv2v → iverilog run `A dr h=2d5` and vita runs the plain twin `if (P == 0) h[4:9] = 6'h3f;` (`logic [11:0] h`) as `A dr h=2d5`
* evidence: PRE 9d60771d, every backend
* note: the named spelling `h.a[H:6]` is row 24's E2002
* status: loud
* source: row 24 park carry plan.q2_dead_rev_lit
