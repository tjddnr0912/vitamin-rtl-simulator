(§3.b sym-member-user-cast · OPEN · in Parser accept) (a cast in a symbolic struct member's range is refused):
* symptom: a member range over a header parameter that spells a cast is E2002 (`hdl-parser/src/sym_struct.rs:154` `names_an_overridable` has no arm for a user cast)
* witness: d2_usercast `parameter W = 4;` `typedef struct packed { logic [integer'(W) - 1 : 0] a; logic [1:0] b; } s_t;` `s_t v;` `v = 6'b100111;` `$display("A bv=%0d ba=%0d va=%b", $bits(v), $bits(v.a), v.a);`
  * PRE `t.sv:3:27: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or a constant-literal range in v1 (a `[E:0]` range over a header parameter is laid out per instance), found identifier 's_t'`
  * iverilog, sv2v → iverilog and verilator `A bv=6 ba=4 va=1001`
* witness: l7_usersize_member the same with `logic [64'(W) - 1 : 0] a;`: the same PRE error, the same three oracle lines
* oracles: 2 oracles
* status: OPEN
* source: W/qsymmix/plan/cells/d2_usercast, l7_usersize_member
