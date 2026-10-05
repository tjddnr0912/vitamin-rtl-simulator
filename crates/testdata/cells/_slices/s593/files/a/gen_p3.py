#!/usr/bin/env python3
# gen_p3.py: audit cells — S(a) loud->value shapes (S), W x lhs shapes (W), SysCall width (F), AE constant functions (A)
# x consumers lp gi gc rb rep psw ad. Writes p3/<grp><n>_<cons>[_ctl].sv
import os
D='p3'; os.makedirs(D, exist_ok=True)
DECL = {
 'SN': "  localparam logic signed [7:0] SN = -8'sd60;\n",
 'SP': "  localparam logic signed [7:0] SP = 8'sd84;\n",
 'U':  "  localparam logic [7:0] U = 8'h0C;\n",
 'B':  "  localparam logic [63:0] B = 64'h8000_0000_0000_000C;\n",
 'B2': "  localparam logic [63:0] B2 = 64'h1_0000_000C;\n",
 'I':  "  localparam int I = 12;\n",
 'fx': "  function automatic logic [3:0] fx(input int a); logic [3:0] t; return t; endfunction\n",
 'fxs':"  function automatic logic signed [3:0] fxs(input int a); logic signed [3:0] t; return t - 4'sd4; endfunction\n",
 'fr': "  function automatic logic [3:0] fr(input int a); if (a > 5) fr = 4'd1; endfunction\n",
 'PA': "  function automatic logic signed [3:0] fxs(input int a); logic signed [3:0] t; return t - 4'sd4; endfunction\n  localparam logic signed [3:0] PA = fxs(2);\n",
 'X':  "  localparam T X = -4;\n",
 'A':  "  localparam T A [0:1] = '{-4, 2};\n",
 'C':  "  localparam bit C = 1;\n",
 'g':  "  if (1) begin : g localparam T Y = -4; end\n",
 'f':  "  function automatic logic [7:0] f(input logic [7:0] v); return v; endfunction\n",
}
EXPRS = {
 'S1': ("SN ==? 4'b?100", 'S', ['SN']), 'S2': ("SP ==? 4'sb?100", 'S', ['SP']), 'S3': ("U inside {4'b1?00}", 'S', ['U']),
 'S4': ("U ==? (4'b1?00)", 'S', ['U']), 'S5': ("U ==? 'b1?00", 'S', ['U']), 'S6': ("B ==? 4'b1?00", 'S', ['B']),
 'S7': ("(4'd15 + 4'd1) ==? 5'b1?000", 'S', []), 'S8': ("8'sb1111_1100 ==? 4'sb1?00", 'S', []),
 'S9': ("I inside {4'b1?00, 4'b0011}", 'S', ['I']),
 'W1': ("X ==? 8'b1111_1?00", 'W', ['X']), 'W2': ("X ==? 4'sb1?00", 'W', ['X']), 'W3': ("A[0] ==? 8'b1111_1?00", 'W', ['A']),
 'W4': ("g.Y ==? 8'b1111_1?00", 'W', ['g']), 'W5': ("$unsigned(X) ==? 8'b1111_1?00", 'W', ['X']),
 'W6': ("(C ? X : A[1]) ==? 8'b1111_1?00", 'W', ['X','A','C']), 'W7': ("X inside {8'b1111_1?00}", 'W', ['X']),
 'W8': ("{X, 4'b0} ==? 12'b1111_1?00_0000", 'W', ['X']), 'W9': ("f(X) ==? 8'b1111_1?00", 'W', ['X','f']),
 'W10': ("(X >>> 1) ==? 8'b1111_11?0", 'W', ['X']),
 'F1': ("$unsigned(B2) ==? 4'b1?00", 'S', ['B2']), 'F2': ("$signed(B2) ==? 4'sb1?00", 'S', ['B2']),
 'F3': ("$unsigned(B2) == 4'd12", 'S', ['B2']),
 'A1': ("fx(2) inside {4'b0?00}", 'A', ['fx']), 'A2': ("fxs(2) ==? 4'sb1?00", 'A', ['fxs']),
 'A3': ("fx(2) ==? (4'b0?00)", 'A', ['fx']), 'A4': ("fx(2) ==? 4'b0?00", 'A', ['fx']),
 'A5': ("PA ==? 4'sb1?00", 'A', ['PA']), 'A6': ("fr(2) inside {4'b0?00}", 'A', ['fr']),
}
def body(cons, e):
    if cons == 'lp':  return f"  localparam L = ({e});\n  initial $display(\"L=%b\", L);\n"
    if cons == 'gi':  return f"  if ({e}) begin : gi initial $display(\"GI=then\"); end else begin : gi initial $display(\"GI=else\"); end\n"
    if cons == 'gc':  return f"  case (1'b1)\n    ({e}): begin : gc initial $display(\"GC=item\"); end\n    default: begin : gc initial $display(\"GC=def\"); end\n  endcase\n"
    if cons == 'rb':  return f"  logic [({e})+3:0] v;\n  initial $display(\"vb=%0d\", $bits(v));\n"
    if cons == 'rep': return f"  wire [7:0] r = {{(({e})+1){{4'b1010}}}};\n  initial #1 $display(\"r=%b\", r);\n"
    if cons == 'psw': return f"  wire [7:0] s = 8'hA5;\n  wire [7:0] ps = s[0 +: ({e})+3];\n  initial #1 $display(\"ps=%b\", ps);\n"
    if cons == 'ad':  return f"  logic [3:0] arr [0:({e})+2];\n  initial $display(\"asz=%0d\", $size(arr));\n"
n=0
for k,(e,g,ds) in EXPRS.items():
    decl=''.join(DECL[d] for d in ds)
    for cons in ('lp','gi','gc','rb','rep','psw','ad'):
        if g in ('S','A'):
            src = "`timescale 1ns/1ns\nmodule t;\n" + decl + body(cons, e) + "  initial #5 $finish;\nendmodule\n"
            open(f'{D}/{k}_{cons}.sv','w').write(src); n+=1
        else:
            for ctl in (False, True):
                if ctl and cons not in ('lp','gc'): continue
                ov = "" if ctl else " #(.T(logic signed [7:0]))"
                src = ("`timescale 1ns/1ns\nmodule m #(parameter type T = logic [7:0]);\n" + decl + body(cons, e) +
                       "endmodule\nmodule t;\n  m" + ov + " u();\n  initial #5 $finish;\nendmodule\n")
                open(f'{D}/{k}_{cons}{"_ctl" if ctl else ""}.sv','w').write(src); n+=1
print(n)
