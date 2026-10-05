#!/usr/bin/env python3
# Batch C: callees that write nothing (chain with empty branches), ctor via member handle, task callees, t0 contexts.
import os
D = os.path.dirname(os.path.abspath(__file__))
CH = "unique if (x) begin end else if (z) begin end"
HH = "if (x) begin end else unique if (z) begin end"
TB = open(os.path.join(D, "t1s_m_fg.sv")).read().split("endmodule\n", 1)[1]
DUTH = "module dut(input logic a, input logic b, output logic [1:0] y);\n"
GN = """  function void g(input logic x, input logic z);
    CHAIN
  endfunction
"""
GNA = GN.replace("function void g", "function automatic void g")
TN = """  task t(input logic x, input logic z);
    CHAIN
  endtask
"""
TNA = TN.replace("task t", "task automatic t")
FG = """  function logic [1:0] f(input logic x, input logic z);
    CALL;
    return {x, z};
  endfunction
"""
FGA = FG.replace("function logic", "function automatic logic")
c = {}
c["u1s_m_fg_nowrite"] = DUTH + GN + FG.replace("CALL", "g(x, z)") + "  assign y = f(a, b);\nendmodule\n" + TB
c["u1a_m_fg_nowrite_auto"] = DUTH + GNA + FGA.replace("CALL", "g(x, z)") + "  assign y = f(a, b);\nendmodule\n" + TB
c["u2s_m_ft_nowrite"] = DUTH + TN + FG.replace("CALL", "t(x, z)") + "  assign y = f(a, b);\nendmodule\n" + TB
c["u2a_m_ft_nowrite_auto"] = DUTH + TNA + FGA.replace("CALL", "t(x, z)") + "  assign y = f(a, b);\nendmodule\n" + TB
c["u3_pkg_fg_nowrite_import"] = "package pk;\n" + GN + FG.replace("CALL", "g(x, z)") + "endpackage\n" + DUTH + "  import pk::*;\n  assign y = f(a, b);\nendmodule\n" + TB
c["u3s_pkg_fg_nowrite_scoped"] = "package pk;\n" + GN + FG.replace("CALL", "g(x, z)") + "endpackage\n" + DUTH + "  assign y = pk::f(a, b);\nendmodule\n" + TB
c["u4_unit_fg_nowrite"] = GN + FG.replace("CALL", "g(x, z)") + DUTH + "  assign y = f(a, b);\nendmodule\n" + TB
c["u5_ifc_fg_nowrite_local"] = "interface I;\n  logic a, b; logic [1:0] y;\n" + GN + FG.replace("CALL", "g(x, z)") + "  assign y = f(a, b);\nendinterface\n" + open(os.path.join(D, "t6l_ifc_fg_local.sv")).read().split("endinterface\n", 1)[1]
c["u6_gen_fg_nowrite"] = DUTH + "  if (1) begin : gb\n" + GN + FG.replace("CALL", "g(x, z)") + "    assign y = f(a, b);\n  end\nendmodule\n" + TB
CLS = "class C;\nMETHODS" + FG.replace("CALL", "CALLC") + "endclass\n"
DUT_OBJ = DUTH + "  C obj; initial obj = new;\n  assign y = obj.f(a, b);\nendmodule\n"
c["u7_c_fg_this_nowrite"] = CLS.replace("METHODS", GN).replace("CALLC", "this.g(x, z)") + DUT_OBJ + TB
c["u8_c_ft_this_nowrite"] = CLS.replace("METHODS", TN).replace("CALLC", "this.t(x, z)") + DUT_OBJ + TB
c["u9_c_ft_bare_nowrite"] = CLS.replace("METHODS", TN).replace("CALLC", "t(x, z)") + DUT_OBJ + TB
# ctor through a member handle
CTOR = """class D;
  logic [1:0] r;
  function new(input logic x, input logic z);
    r = 0;
    unique if (x) r = 1; else if (z) r = 2;
  endfunction
endclass
"""
CTORH = CTOR.replace("unique if (x) r = 1; else if (z) r = 2;", "if (x) r = 1; else unique if (z) r = 2;")
for nm, asg in (("u10_c_fnew_member", "d = new(x, z)"), ("u11_c_fnew_this", "this.d = new(x, z)")):
    body = """class C;
  D d;
  function logic [1:0] f(input logic x, input logic z);
    ASG;
    return d.r;
  endfunction
endclass
""".replace("ASG", asg) + DUT_OBJ + TB
    c[nm] = "CTORX" + body
# t0 contexts for armed procedural positions (chain on a, b written by a later t0 initial)
c["s1i_initial_first"] = """module top;
  logic a, b;
  initial begin : chk
    logic x, z; x = a; z = b;
    CHAIN
  end
  initial begin a = 0; b = 1; #1 b = 0; #1 $finish; end
  initial #100 $finish;
endmodule
"""
c["s1s_selftimed_first"] = """module top;
  logic a, b;
  always begin : chk
    logic x, z; x = a; z = b;
    CHAIN
    @(a or b);
  end
  initial begin a = 0; b = 1; #1 b = 0; #1 $finish; end
  initial #100 $finish;
endmodule
"""
CHILD = """module dut(input logic a, input logic b, output logic [1:0] y);
  assign y = 0;
  initial begin : chk
    logic x, z; x = a; z = b;
    CHAIN
  end
endmodule
"""
c["s2_child_initial_tbL"] = CHILD + TB
c["s2_child_initial_tbF"] = TB + CHILD
c["s3_child_selftimed_tbL"] = CHILD.replace("initial begin : chk", "always begin : chk").replace("    CHAIN\n  end", "    CHAIN\n    @(a or b);\n  end") + TB
c["s4_ctor_initial"] = "CTORX" + """module top;
  logic a, b; D d;
  initial d = new(a, b);
  initial begin a = 0; b = 1; #1 b = 0; #1 $finish; end
  initial #100 $finish;
endmodule
"""
for n, t in c.items():
    plain = t.replace("CHAIN", CH).replace("CTORX", CTOR)
    h = t.replace("CHAIN", HH).replace("CTORX", CTORH)
    open(os.path.join(D, n + ".sv"), "w").write(plain)
    open(os.path.join(D, n + "_H.sv"), "w").write(h)
print(" ".join(sorted(c)))
