#!/usr/bin/env python3
# Batch D: a class non-void method (reached from a CA through a handle) calling a $unit / package void function.
import os
D = os.path.dirname(os.path.abspath(__file__))
TB = open(os.path.join(D, "t1s_m_fg.sv")).read().split("endmodule\n", 1)[1]
DUT_OBJ = "module dut(input logic a, input logic b, output logic [1:0] y);\n  C obj; initial obj = new;\n  assign y = obj.f(a, b);\nendmodule\n"
G = {"w": "function void g(input logic x, input logic z);\n  logic [1:0] r; r = 0;\n  CHAIN_W\nendfunction\n",
     "n": "function void g(input logic x, input logic z);\n  CHAIN_N\nendfunction\n"}
CLS = "class C;\n  function logic [1:0] f(input logic x, input logic z);\n    CALL;\n    return {x, z};\n  endfunction\nendclass\n"
c = {}
for v, g in G.items():
    c[f"v1{v}_unit_vg_from_cls"] = g + CLS.replace("CALL", "g(x, z)") + DUT_OBJ + TB
    c[f"v2{v}_pkg_vg_unit_import_from_cls"] = "package pk;\n" + g + "endpackage\nimport pk::*;\n" + CLS.replace("CALL", "g(x, z)") + DUT_OBJ + TB
    c[f"v3{v}_pkg_vg_scoped_from_cls"] = "package pk;\n" + g + "endpackage\n" + CLS.replace("CALL", "pk::g(x, z)") + DUT_OBJ + TB
    c[f"v4{v}_unit_task_from_cls"] = g.replace("function void g", "task g").replace("endfunction", "endtask") + CLS.replace("CALL", "g(x, z)") + DUT_OBJ + TB
CW = ("unique if (x) r = 1; else if (z) r = 2;", "if (x) r = 1; else unique if (z) r = 2;")
CN = ("unique if (x) begin end else if (z) begin end", "if (x) begin end else unique if (z) begin end")
for n, t in c.items():
    open(os.path.join(D, n + ".sv"), "w").write(t.replace("CHAIN_W", CW[0]).replace("CHAIN_N", CN[0]))
    open(os.path.join(D, n + "_H.sv"), "w").write(t.replace("CHAIN_W", CW[1]).replace("CHAIN_N", CN[1]))
    print(n)
