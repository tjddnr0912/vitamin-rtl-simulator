(§2 bind-actual-lexical-layout · OPEN · in Scoping / imports / block-locals) (a `bind` actual's struct member takes the bind scope's layout, not the target's):
* symptom: a member select in a `bind` instance's port actual is desugared with the struct layout of the scope that holds the `bind` statement and read on the target scope's variable, so it reads the wrong bits where the two scopes declare the name with different layouts
* witness (row 24's EH1-cut premise judge 1, builder line 10, W/r24e/premise/cells/k4j_bind_la): `typedef struct packed { logic [1:0] h; logic [1:0] b; } pk_t;` and `typedef struct packed { logic [1:0] b; logic [1:0] h; } pk2_t;`, `module m (output logic [3:0] q); pk_t s; assign s = 4'b1001; assign q = s; endmodule`, in `t` `pk2_t s;` (`s = 4'b0110`) and `bind m chk2 c (.a({s.h[1:0], s.b[1:0]}));` with `chk2` printing `C a=%b`: PRE cdf0c457 rc 0 `A q=1001` `C a=0110`, and the same on the flat and whole-member spellings (flat.sv, whole.sv)
  * oracles: verilator 5.054 `A q=1001` `C a=1001`; iverilog 13.0 `t.sv:12: syntax error`, `t.sv:12: error: Invalid module item.`; sv2v `t.sv:12:3: Parse error: missing expected `endmodule``
  * IEEE 1800-2017 §23.11 (ieee1800-2017.txt:34054-34055): "All actual ports and parameters in the bind_instantiation refer to objects from the viewpoint of the bind_target_instance."
* note: pre-existing, not row 24's: the judge measured it on all three spellings (K4, k4j)
* oracles: verilator and hand-IEEE (iverilog and sv2v refuse the `bind` line, t.sv:12)
* status: OPEN
