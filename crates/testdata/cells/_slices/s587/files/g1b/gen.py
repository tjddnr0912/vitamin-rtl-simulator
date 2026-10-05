FN = {
 "if":   "  function automatic int f(input int a);\n    f = 7;\n    unique if (a == 1) f = 10;\n  endfunction\n",
 "pif":  "  function automatic int f(input int a);\n    f = 7;\n    priority if (a == 1) f = 10;\n  endfunction\n",
 "case": "  function automatic int f(input int a);\n    f = 7;\n    unique case (a) 1: f = 10; 3: f = 30; endcase\n  endfunction\n",
}
WD = "  initial begin #1 $display(\"%s\", %s); $finish; end\n"
def ctx(c, fn, arg):
  F = FN[fn]
  if c=="pp":  return "module top;\n"+F+f"  parameter int P = f({arg});\n"+WD%("P=%0d","P")+"endmodule\n"
  if c=="pr":  return "module top;\n"+F+f"  logic [f({arg}):0] v;\n"+WD%("bits=%0d","$bits(v)")+"endmodule\n"
  if c=="ur":  return "module top;\n"+F+f"  logic u [f({arg})];\n"+WD%("size=%0d","$size(u)")+"endmodule\n"
  if c=="gi":  return "module top;\n"+F+f"  if (f({arg}) == 7) begin : g7\n    initial begin #1 $display(\"gi=7\"); $finish; end\n  end else begin : gx\n    initial begin #1 $display(\"gi=other\"); $finish; end\n  end\nendmodule\n"
  if c=="gf":  return "module top;\n"+F+f"  for (genvar i = 0; i < f({arg}); i++) begin : g\n    localparam int K = i;\n  end\n  initial begin #1 $display(\"last=%0d\", g[f({arg})-1].K); $finish; end\nendmodule\n"
  if c=="gc":  return "module top;\n"+F+f"  case (f({arg}))\n    7: begin : g7\n      initial begin #1 $display(\"gc=7\"); $finish; end\n    end\n    default: begin : gd\n      initial begin #1 $display(\"gc=other\"); $finish; end\n    end\n  endcase\nendmodule\n"
  if c=="sf":  return "module top;\n"+F+f"  localparam int P = $clog2(f({arg}));\n"+WD%("P=%0d","P")+"endmodule\n"
  if c=="pw":  return "module sub(input logic [f(%d):0] p);\n"%arg+F+"  initial begin #1 $display(\"pw=%0d\", $bits(p)); $finish; end\nendmodule\nmodule top;\n  logic [15:0] w;\n  sub u(.p(w));\nendmodule\n"
  if c=="ov":  return "module sub #(parameter int W = 1);\n  initial begin #1 $display(\"W=%0d\", W); $finish; end\nendmodule\nmodule top;\n"+F+f"  sub #(.W(f({arg}))) u();\nendmodule\n"
  if c=="dp":  return "module sub;\n  parameter int W = 1;\n  initial begin #1 $display(\"W=%0d\", W); $finish; end\nendmodule\nmodule top;\n"+F+f"  sub u();\n  defparam u.W = f({arg});\nendmodule\n"
  if c=="pk":  return "package pk;\n"+F+f"  localparam int P = f({arg});\nendpackage\nmodule top;\n"+WD%("P=%0d","pk::P")+"endmodule\n"
  if c=="pkx": return "package pk;\n"+F+"endpackage\nmodule top;\n"+f"  localparam int P = pk::f({arg});\n"+WD%("P=%0d","P")+"endmodule\n"
  if c=="pki": return "package pk;\n"+F+"endpackage\nmodule top;\n  import pk::*;\n"+f"  localparam int P = f({arg});\n"+WD%("P=%0d","P")+"endmodule\n"
  if c=="cl":  return "module top;\n"+F+f"  class C #(parameter int P = f({arg}));\n    static function int get(); return P; endfunction\n  endclass\n"+WD%("P=%0d","C#()::get()")+"endmodule\n"
  if c=="en":  return "module top;\n"+F+f"  typedef enum int {{A = f({arg}), B}} e_t;\n"+WD%("A=%0d","A")+"endmodule\n"
  if c=="gl":  return "module top;\n"+F+f"  if (1) begin : gb\n    localparam int P = f({arg});\n  end\n"+WD%("P=%0d","gb.P")+"endmodule\n"
  if c=="pa":  return "module top;\n"+F+f"  localparam int A [2] = '{{f({arg}), 1}};\n"+WD%("A0=%0d","A[0]")+"endmodule\n"
  if c=="nest": return "module top;\n"+F.replace("function automatic int f","function automatic int g").replace(" f "," g ").replace("    f =","    g =").replace("f = 10","g = 10").replace("f = 30","g = 30")+"  function automatic int f(input int a);\n    f = g(a) + 1;\n  endfunction\n"+f"  localparam int P = f({arg});\n"+WD%("P=%0d","P")+"endmodule\n"
  if c=="loop": return "module top;\n"+F.replace("    f = 7;\n","    f = 7;\n    for (int i = 0; i < 3; i++)\n").replace("(a == 1)","(a == i)")+f"  localparam int P = f({arg});\n"+WD%("P=%0d","P")+"endmodule\n"
ctxs=["pp","pr","ur","gi","gf","gc","sf","pw","ov","dp","pk","pkx","pki","cl","en","gl","pa","nest","loop"]
for c in ctxs:
  for fn in ("if","pif","case"):
    reaches = (("M",2),("N",1)) if fn=="if" else (("M",2),)
    if c=="loop": reaches = (("M",5),("N",1)) if fn=="if" else (("M",5),)
    for r,arg in reaches:
      open(f"b_{c}_{fn}_{r}.sv","w").write(ctx(c,fn,arg))
