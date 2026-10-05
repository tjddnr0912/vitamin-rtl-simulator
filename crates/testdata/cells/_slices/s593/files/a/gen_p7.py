#!/usr/bin/env python3
# gen_p7.py: the 160 loud->value families (lens / c4 / c8-c12 shapes) varied into the 🆕 AC sinks (gc rb rep psw ad)
# plus lp/gi, and 🆕 AE variants (a call reading a never-assigned 4-state variable inside the left operand / via a parameter)
import os
D='p7'; os.makedirs(D, exist_ok=True)
def body(cons, e, ind='  '):
    b={'lp':  f"localparam L = ({e});\ninitial #1 $display(\"L=%b\", L);\n",
       'gi':  f"if ({e}) begin : gi initial #1 $display(\"GI=then\"); end else begin : gi initial #1 $display(\"GI=else\"); end\n",
       'gc':  f"case (1'b1)\n  ({e}): begin : gc initial #1 $display(\"GC=item\"); end\n  default: begin : gc initial #1 $display(\"GC=def\"); end\nendcase\n",
       'rb':  f"logic [({e})+3:0] v;\ninitial #1 $display(\"vb=%0d\", $bits(v));\n",
       'rep': f"wire [7:0] r = {{(({e})+1){{4'b1010}}}};\ninitial #1 $display(\"r=%b\", r);\n",
       'psw': f"wire [7:0] s = 8'hA5;\nwire [7:0] ps = s[0 +: ({e})+3];\ninitial #1 $display(\"ps=%b\", ps);\n",
       'ad':  f"logic [3:0] arr [0:({e})+2];\ninitial #1 $display(\"asz=%0d\", $size(arr));\n"}[cons]
    return ''.join(ind+l+'\n' for l in b.splitlines())
FX="  function automatic logic [3:0] fx(input int a); logic [3:0] t; return t; endfunction\n"
FXS="  function automatic logic signed [3:0] fxs(input int a); logic signed [3:0] t; return t; endfunction\n"
FXS64="  function automatic logic signed [63:0] fxs64(input int a); logic signed [63:0] t; return t - 64'sd4; endfunction\n"
def top(decl, cons, e):
    return "`timescale 1ns/1ns\nmodule t;\n" + decl + body(cons, e) + "  initial #5 $finish;\nendmodule\n"
FAM = {
 # name: function(cons) -> source
 'B02': lambda c: top("  typedef enum logic signed [3:0] {ES = -4'sd4, ET = 4'sd3} est4;\n", c, "ES ==? 8'sb1111_1?00"),
 'DP3': lambda c: ("`timescale 1ns/1ns\nmodule sub;\n  parameter logic signed [63:0] P = 0;\n" + body(c, "P ==? 4'sb1?00") +
                   "endmodule\nmodule t;\n  sub u();\n  defparam u.P = -4;\n  initial #5 $finish;\nendmodule\n"),
 'B07': lambda c: ("`timescale 1ns/1ns\npackage pa;\n  localparam logic signed [63:0] PS = -64'sd4;\nendpackage\npackage pb;\n  import pa::*;\n"
                   "  localparam RB = (PS ==? 4'sb1?00);\nendpackage\n" + top("", c, "pb::RB")[len("`timescale 1ns/1ns\n"):]),
 'B10': lambda c: ("`timescale 1ns/1ns\nmodule t;\n  if (1) begin : gb\n    localparam logic signed [63:0] GS = -64'sd4;\n" +
                   body(c, "GS ==? 4'sb1?00", '    ') + "  end\n  initial #5 $finish;\nendmodule\n"),
 'PT5': lambda c: ("`timescale 1ns/1ns\nmodule sub #(parameter type T = logic [3:0], parameter T PV = '0);\n" + body(c, "PV ==? 4'sb1?00") +
                   "endmodule\nmodule t;\n  sub #(.T(logic signed [63:0]), .PV(-64'sd4)) u();\n  initial #5 $finish;\nendmodule\n"),
 'TRN': lambda c: top("  localparam logic signed [7:0] SA = -8'sd4;\n  localparam bit C = 1;\n", c, "(C ? SA : 8'sd0) ==? 4'sb1?00"),
 'ADD': lambda c: top("  localparam logic signed [7:0] SA = -8'sd4;\n", c, "(SA + 8'sd0) ==? 4'sb1?00"),
 'SHR': lambda c: top("  localparam logic signed [3:0] SA = -4'sd2;\n", c, "(SA >>> 1) ==? 4'sb111?"),
 # 🆕 AE variants
 'xDP': lambda c: ("`timescale 1ns/1ns\nmodule sub;\n  parameter logic signed [63:0] P = 0;\n" + body(c, "P ==? 4'sb1?00") +
                   "endmodule\nmodule t;\n" + FXS64 + "  sub u();\n  defparam u.P = fxs64(2);\n  initial #5 $finish;\nendmodule\n"),
 'xTRN': lambda c: top(FXS + "  localparam bit C = 1;\n", c, "(C ? fxs(2) - 4'sd4 : 4'sd0) ==? 4'sb1?00"),
 'xADD': lambda c: top(FXS, c, "(fxs(2) - 4'sd4) ==? 4'sb1?00"),
 'xCAT': lambda c: top(FX, c, "{fx(2), 4'b0000} inside {8'b0?00_0000}"),
 'xSYS': lambda c: top(FX, c, "$clog2(fx(2) + 4'd1) inside {32'b0000_0000_0000_0000_0000_0000_0000_000?}"),
 'xPT':  lambda c: ("`timescale 1ns/1ns\nmodule sub #(parameter type T = logic [3:0], parameter T PV = '0);\n" + body(c, "PV ==? 4'sb1?00") +
                   "endmodule\nmodule t;\n" + FXS64 + "  sub #(.T(logic signed [63:0]), .PV(fxs64(2))) u();\n  initial #5 $finish;\nendmodule\n"),
}
n=0
for k,f in FAM.items():
    for c in ('lp','gi','gc','rb','rep','psw','ad'):
        open(f'{D}/{k}_{c}.sv','w').write(f(c)); n+=1
print(n)
