#!/usr/bin/env python3
# gen_p8.py: W scalar (no T-typed array) x a left operand whose self width const_self_width cannot give
# (explicit-array element sibling, parameter-count replication, ...) -> masked_pre on a now-negative value
import os
D='p8'; os.makedirs(D, exist_ok=True)
DECL="""  localparam T X = -4;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  localparam int N = 2;
  localparam bit C = 1;
"""
EX = {
 'Wu1': "(C ? X : A[1]) ==? 8'b1111_1?00",
 'Wu2': "(X | A[1]) ==? 8'b1111_11?0",
 'Wu3': "{N{X}} ==? 16'b1111_1100_1111_1?00",
 'Wu4': "(X + {N{1'b0}}) ==? 8'b1111_1?00",
 'Wu5': "(X & A[0]) ==? 4'sb1?00",
 'Wu6': "X ==? 4'sb1?00",
}
def body(cons, e):
    if cons == 'lp':  return f"  localparam L = ({e});\n  initial #1 $display(\"L=%b\", L);\n"
    if cons == 'gc':  return f"  case (1'b1)\n    ({e}): begin : gc initial #1 $display(\"GC=item\"); end\n    default: begin : gc initial #1 $display(\"GC=def\"); end\n  endcase\n"
    if cons == 'rb':  return f"  logic [({e})+3:0] v;\n  initial #1 $display(\"vb=%0d\", $bits(v));\n"
    if cons == 'gi':  return f"  if ({e}) begin : gi initial #1 $display(\"GI=then\"); end else begin : gi initial #1 $display(\"GI=else\"); end\n"
n=0
for k,e in EX.items():
    for c in ('lp','gc','rb','gi'):
        for ctl in (False, True):
            ov = "" if ctl else " #(.T(logic signed [7:0]))"
            src = ("`timescale 1ns/1ns\nmodule m #(parameter type T = logic [7:0]);\n" + DECL + body(c, e) +
                   "endmodule\nmodule t;\n  m" + ov + " u();\n  initial #5 $finish;\nendmodule\n")
            open(f'{D}/{k}_{c}{"_ctl" if ctl else ""}.sv','w').write(src); n+=1
print(n)
