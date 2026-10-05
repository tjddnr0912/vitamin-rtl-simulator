#!/usr/bin/env python3
# Generates D1 cells: <name>.sv (plain chain) and <name>_H.sv (H spelling = armed tree on PRE).
import os
D = os.path.dirname(os.path.abspath(__file__))
CH = "unique if (x) r = 1; else if (z) r = 2;"
HH = "if (x) r = 1; else unique if (z) r = 2;"
TB = """module top;
  logic a, b; logic [1:0] y;
  dut u(.a(a), .b(b), .y(y));
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 b = 0;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"""
G_IN = """  function void g(input logic x, input logic z);
    logic [1:0] r; r = 0;
    CHAIN
  endfunction
"""
G_IN_A = G_IN.replace("function void g", "function automatic void g")
F_G = """  function logic [1:0] f(input logic x, input logic z);
    g(x, z);
    return {x, z};
  endfunction
"""
F_G_A = F_G.replace("function logic", "function automatic logic")
DUTH = "module dut(input logic a, input logic b, output logic [1:0] y);\n"
cells = {}
# module: non-void f -> void g (input-only, static / automatic / output formal)
cells["t1s_m_fg"] = DUTH + G_IN + F_G + "  assign y = f(a, b);\nendmodule\n" + TB
cells["t1a_m_fg_auto"] = DUTH + G_IN_A + F_G_A + "  assign y = f(a, b);\nendmodule\n" + TB
cells["t1o_m_fg_out"] = DUTH + """  function void g(input logic x, input logic z, output logic [1:0] r);
    r = 0;
    CHAIN
  endfunction
  function logic [1:0] f(input logic x, input logic z);
    logic [1:0] q; g(x, z, q);
    return q;
  endfunction
  assign y = f(a, b);
endmodule
""" + TB
# module: f -> h (non-void) -> g (void)
cells["t8s_m_fhg"] = DUTH + G_IN + """  function logic [1:0] h(input logic x, input logic z);
    g(x, z);
    return {x, z};
  endfunction
  function logic [1:0] f(input logic x, input logic z);
    return h(x, z);
  endfunction
  assign y = f(a, b);
endmodule
""" + TB
# class: non-void f -> void g via this. / bare
CLS_G = """class C;
  function void g(input logic x, input logic z);
    logic [1:0] r; r = 0;
    CHAIN
  endfunction
  function logic [1:0] f(input logic x, input logic z);
    CALL;
    return {x, z};
  endfunction
endclass
"""
DUT_OBJ = DUTH + "  C obj; initial obj = new;\n  assign y = obj.f(a, b);\nendmodule\n"
cells["t2t_c_fg_this"] = CLS_G.replace("CALL", "this.g(x, z)") + DUT_OBJ + TB
cells["t2b_c_fg_bare"] = CLS_G.replace("CALL", "g(x, z)") + DUT_OBJ + TB
# class: non-void f constructs D whose ctor holds the chain; module f the same
CLS_D_CTOR = """class D;
  logic [1:0] r;
  function new(input logic x, input logic z);
    r = 0;
    CHAIN
  endfunction
endclass
"""
cells["t3c_c_fnew"] = CLS_D_CTOR + """class C;
  function logic [1:0] f(input logic x, input logic z);
    D d; d = new(x, z);
    return d.r;
  endfunction
endclass
""" + DUT_OBJ + TB
cells["t3m_m_fnew"] = CLS_D_CTOR + DUTH + """  function automatic logic [1:0] f(input logic x, input logic z);
    D d; d = new(x, z);
    return d.r;
  endfunction
  assign y = f(a, b);
endmodule
""" + TB
# module f calls a class void method through a handle (formal / module variable); class f through a member handle
CLS_VG = """class C;
  function void g(input logic x, input logic z);
    logic [1:0] r; r = 0;
    CHAIN
  endfunction
endclass
"""
cells["t4h_m_fh_g"] = CLS_VG + DUTH + """  C obj; initial obj = new;
  function automatic logic [1:0] f(C h, input logic x, input logic z);
    h.g(x, z);
    return {x, z};
  endfunction
  assign y = f(obj, a, b);
endmodule
""" + TB
cells["t4o_m_fobj_g"] = CLS_VG + DUTH + """  C obj; initial obj = new;
  function logic [1:0] f(input logic x, input logic z);
    obj.g(x, z);
    return {x, z};
  endfunction
  assign y = f(a, b);
endmodule
""" + TB
cells["t4d_c_fdg"] = CLS_VG.replace("class C;", "class D;") + """class C;
  D d;
  function new; d = new; endfunction
  function logic [1:0] f(input logic x, input logic z);
    d.g(x, z);
    return {x, z};
  endfunction
endclass
""" + DUT_OBJ + TB
# package
PKG = "package pk;\n" + G_IN + F_G + "endpackage\n"
cells["t5s_pkg_fg_scoped"] = PKG + DUTH + "  assign y = pk::f(a, b);\nendmodule\n" + TB
cells["t5i_pkg_fg_import"] = PKG + DUTH + "  import pk::*;\n  assign y = f(a, b);\nendmodule\n" + TB
cells["t5a_pkg_fg_auto_import"] = "package pk;\n" + G_IN_A + F_G_A + "endpackage\n" + DUTH + "  import pk::*;\n  assign y = f(a, b);\nendmodule\n" + TB
# interface: CA inside the interface; CA in a module through the instance name
cells["t6l_ifc_fg_local"] = "interface I;\n  logic a, b; logic [1:0] y;\n" + G_IN + F_G + """  assign y = f(a, b);
endinterface
module top;
  I i();
  initial begin
    i.a = 0; i.b = 1;
    #1 $display("t=%0t y=%0d", $time, i.y);
    #1 i.b = 0;
    #1 $display("t=%0t y=%0d", $time, i.y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"""
cells["t6h_ifc_fg_hier"] = "interface I;\n" + G_IN + F_G + "endinterface\n" + DUTH + "  I i();\n  assign y = i.f(a, b);\nendmodule\n" + TB
# $unit
cells["t7u_unit_fg"] = G_IN.replace("  ", "", 1) + F_G + DUTH + "  assign y = f(a, b);\nendmodule\n" + TB
# generate block
cells["t9g_gen_fg"] = DUTH + "  if (1) begin : gb\n" + G_IN + F_G + "    assign y = f(a, b);\n  end\nendmodule\n" + TB
# program with a continuous assign
cells["t10p_prog_fg"] = "program dut(input logic a, input logic b, output logic [1:0] y);\n" + G_IN + F_G + "  assign y = f(a, b);\nendprogram\n" + TB
# net declaration assignment and port expression spellings of a CA (module void g)
cells["t11w_m_netdecl"] = DUTH.replace("output logic [1:0] y", "output wire [1:0] y") + G_IN + F_G + "  wire [1:0] w = f(a, b);\n  assign y = w;\nendmodule\n" + TB
cells["t12p_m_portexpr"] = """module leaf(input logic [1:0] i, output logic [1:0] o);
  assign o = i;
endmodule
""" + DUTH + G_IN + F_G + "  leaf l(.i(f(a, b)), .o(y));\nendmodule\n" + TB
# N2's only reach: a function forks a task (task holds the chain)
cells["t13k_m_fork_task"] = DUTH + """  task automatic t(input logic x, input logic z);
    logic [1:0] r; r = 0;
    CHAIN
  endtask
  function automatic logic [1:0] f(input logic x, input logic z);
    fork t(x, z); join_none
    return {x, z};
  endfunction
  assign y = f(a, b);
endmodule
""" + TB
for n, t in cells.items():
    open(os.path.join(D, n + ".sv"), "w").write(t.replace("CHAIN", CH))
    open(os.path.join(D, n + "_H.sv"), "w").write(t.replace("CHAIN", HH))
print(" ".join(sorted(cells)))
