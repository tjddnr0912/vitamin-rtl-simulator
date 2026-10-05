#!/usr/bin/env python3
"""A1: the 5 loud->value cells' opened value sent into other consumers (+ control twins Gx/Ex)."""
import os, sys
out = sys.argv[1]
PK_W = "package pk; localparam [64:0] i = 65'h1_0000_0000_0000_0009; endpackage\n"
PK_E = "package pk; localparam [64:0] E1 = 65'h1_0000_0000_0000_0000; endpackage\n"
BASES = {
  # name: (pre-module text, module-head text, loop?, NAME)
  "Gw":  ("", "  localparam [64:0] i = 65'h1_0000_0000_0000_0009;\n", True, "i"),
  "Gwi": (PK_W, "  import pk::*;\n", True, "i"),
  "Gwe": (PK_W, "  import pk::i;\n", True, "i"),
  "Gs":  ("", "  localparam string i = \"AB\";\n", True, "i"),
  "Gx":  ("", "", True, "i"),
  "Ewi": (PK_E, "  import pk::*;\n  typedef enum {E0, E1} e_t;\n", False, "E1"),
  "Ex":  ("", "  typedef enum {E0, E1} e_t;\n", False, "E1"),
}
SUBS = {
 "sub": "module sub #(parameter P = 0) ();\n  initial #3 $display(\"ovr %m P=%0d b=%0d\", P, $bits(P));\nendmodule\n",
 "subh": "module subh #(parameter P = 0) ();\n  initial #3 $display(\"ovrh %m P=%h b=%0d\", P, $bits(P));\nendmodule\n",
 "sgen": ("module sgen #(parameter N = 0) ();\n"
          "  if (N == 1) begin : one initial #3 $display(\"gif %m one\"); end else begin : oth initial #3 $display(\"gif %m other\"); end\n"
          "  case (N) 0: begin : c0 initial #3 $display(\"gcs %m zero\"); end 1: begin : c1 initial #3 $display(\"gcs %m one\"); end default: begin : cd initial #3 $display(\"gcs %m def\"); end endcase\n"
          "  for (genvar k = 0; k <= N; k++) begin : lp initial #3 $display(\"gfor %m k=%0d\", k); end\n"
          "endmodule\n"),
 "srep": ("module srep #(parameter N = 0) ();\n"
          "  wire [7:0] r = {(N+1){1'b1}};\n  localparam [7:0] RR = {(N+1){1'b1}};\n"
          "  initial #3 $display(\"rep %m r=%b RR=%b\", r, RR);\nendmodule\n"),
 "siar": ("module siar #(parameter N = 0) ();\n  leaf lf[N:0] ();\nendmodule\n"
          "module leaf ();\n  initial #3 $display(\"leaf %m\");\nendmodule\n"),
 "sae": ("module sae #(parameter N = 0) ();\n"
         "  function automatic integer fae(input integer x); logic [7:0] t; fae = x + t; endfunction\n"
         "  localparam integer W = fae(N);\n  initial #3 $display(\"ae %m W=%0d\", W);\nendmodule\n"),
 "sac": ("module sac #(parameter N = 0) ();\n"
         "  function automatic integer fc(input integer x); case (x) 0: fc = 3; 1: fc = 5; default: fc = 7; endcase endfunction\n"
         "  wire [7:0] r = {fc(N){1'b1}};\n  initial #3 $display(\"ac %m r=%b\", r);\nendmodule\n"),
 "sacr": ("module sacr #(parameter N = 0) (input logic [N:0] a);\n"
         "  function automatic integer fc(input integer x); case (x) 0: fc = 3; 1: fc = 5; default: fc = 7; endcase endfunction\n"
         "  wire [7:0] r = {fc(N){1'b1}};\n  initial #3 $display(\"acr %m r=%b b=%0d\", r, $bits(a));\nendmodule\n"),
 "saer": ("module saer #(parameter N = 0) (input logic [N:0] a);\n"
         "  function automatic integer fae(input integer x); logic [7:0] t; fae = x + t; endfunction\n"
         "  localparam integer W = fae(N);\n  initial #3 $display(\"aer %m W=%0d b=%0d\", W, $bits(a));\nendmodule\n"),
 "leaf": "module leaf ();\n  initial #3 $display(\"leaf %m\");\nendmodule\n",
}
def body(cons, N):
    """consumer text placed in the genvar scope (loop body) or module scope (label)."""
    if cons == "ovrx1":   return f"    sub #(.P({N} + 1)) ua ();\n    sub #(.P({N} - 2)) ue ();\n", ["sub"]
    if cons == "ovrx2":   return f"    subh #(.P({N}[3:0])) ub ();\n    subh #(.P({{{N}, 1'b0}})) uc ();\n    subh #(.P(8'({N}))) ud ();\n", ["subh"]
    if cons == "cgen":    return f"    sgen #(.N({N})) u3 ();\n", ["sgen"]
    if cons == "crep":    return f"    srep #(.N({N})) u4 ();\n", ["srep"]
    if cons == "ciar":    return f"    siar #(.N({N})) u5 ();\n", ["siar"]
    if cons == "cae":     return f"    sae #(.N({N})) u6 ();\n", ["sae"]
    if cons == "cac":     return f"    sac #(.N({N})) u7 ();\n", ["sac"]
    if cons == "cacr":    return f"    sacr #(.N({N})) u8 (.a('0));\n", ["sacr"]
    if cons == "caer":    return f"    saer #(.N({N})) u9 (.a('0));\n", ["saer"]
    if cons == "prep":    return (f"    wire [7:0] r = {{({N}+1){{1'b1}}}};\n    localparam [7:0] RR = {{({N}+1){{1'b1}}}};\n"
                                  f"    initial #3 $display(\"prep %m r=%b RR=%b b=%0d\", r, RR, $bits({N}));\n"), []
    if cons == "piar":    return f"    leaf la[{N}:0] ();\n", ["leaf"]
    if cons == "pae":     return (f"    function automatic integer fae(input integer x); logic [7:0] t; fae = x + t; endfunction\n"
                                  f"    localparam integer W = fae({N});\n    initial #3 $display(\"pae %m W=%0d\", W);\n"), []
    if cons == "pac":     return (f"    function automatic integer fc(input integer x); case (x) 0: fc = 3; 1: fc = 5; default: fc = 7; endcase endfunction\n"
                                  f"    wire [7:0] r = {{fc({N}){{1'b1}}}};\n    initial #3 $display(\"pac %m r=%b\", r);\n"), []
    if cons == "pcl":     return (f"    logic [31:0] k = 1;\n    initial #3 case (k) {N}: $display(\"pcl %m hit\"); default: $display(\"pcl %m miss\"); endcase\n"
                                  f"    case (1) {N}: begin : gm initial #3 $display(\"gcl %m hit\"); end default: begin : gd initial #3 $display(\"gcl %m miss\"); end endcase\n"), []
    raise SystemExit(cons)
CONS = ["ovrx1", "ovrx2", "cgen", "crep", "ciar", "cae", "cac", "cacr", "caer", "prep", "piar", "pae", "pac", "pcl"]
os.makedirs(out, exist_ok=True)
n = 0
for b, (pre, head, loop, N) in BASES.items():
    for c in CONS:
        txt, subs = body(c, N)
        s = pre + "module top;\n" + head
        if loop:
            s += "  for (genvar i = 0; i < 2; i++) begin : g\n" + txt + "  end\n"
        else:
            s += txt.replace("\n    ", "\n  ").replace("    ", "  ", 1)
        s += "  initial #100 $finish;\nendmodule\n"
        for m in subs:
            s += SUBS[m]
        open(os.path.join(out, f"{b}_{c}.sv"), "w").write(s); n += 1
print(n, "cells")
