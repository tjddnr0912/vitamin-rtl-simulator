(§5.2 Q-MEMBIDXWR · rank ② · source row 24 plan Q3 (x.near.pwi_s1); row 24 park carry bc.var_bit_wr) (an indexed part-select or variable-index bit write of a struct member is refused):
* slot: after Q-MEMBSRC
* symptom: an indexed part-select of a packed-struct member as a statement target is E2002 at parse, a literal base too: `typedef struct packed {logic [3:0] a; logic [3:0] b;} s_t; s_t v; v = 8'hA5; v.a[2 -: 2] = 2'b10; v.b[1 -: 2] = 2'b10;` prints "expected a constant `[a:b]` range or bit-select on a packed-struct member WRITE (an indexed `[i+:w]`/`[i-:w]` is unsupported — iverilog refuses it too)" (`hdl-parser/src/struct_sel.rs:934`, the indexed write arm), also `v.a[1 +: 2] = 2'b10;`
* oracles: sv2v → iverilog and verilator `A pwi v=c6` (the `+:` cell: sv2v `c5`), iverilog `sorry: Assignments to part selects of a struct member are not yet supported.`
* pins: the same select as a read (`v.a[2 -: 2]`, `01` = sv2v) and the plain twin (`v[H -: 2] = 2'b10; v[1 -: 2] = 2'b10;` on `logic [7:0] v`, H = 6: `c6` = iverilog = sv2v) run
* witness: a named base (`v.a[H -: 2]`) is refused the same way
* symptom: a variable-index member BIT write is refused by its own arm: `typedef struct packed { logic [15:8] a; logic [3:0] b; } h_t; int i; h_t h; h = 12'h583; i = 10; h.a[i] = 1'b1;` is E2002 "expected a constant packed-struct member bit-select WRITE index, found '='" (`hdl-parser/src/struct_sel.rs:928`), where IEEE 1800-2017 §11.5.1 (ieee1800-2017.txt:12351) addresses a bit with an expression and sv2v → iverilog prints `A vbw h=5c3` (iverilog asserts, `elab_lval.cc:1406`)
* evidence: PRE 9d60771d, every backend
* fix: a constant base writes the member's bits as the literal `[a:b]` arm does, a variable base is a run-time offset into the container
* status: loud
