FN = {
 "if":   "  function automatic int f(input int a);\n    f = 7;\n    unique if (a == 1) f = 10;\n  endfunction\n",
 "case": "  function automatic int f(input int a);\n    f = 7;\n    unique case (a) 1: f = 10; 3: f = 30; endcase\n  endfunction\n",
}
P = {
 "prd":  "  logic [31:0] v = 32'hFFFF_FFFF;\n  initial begin #1 $display(\"pr=%h\", v[CALL:0]); $finish; end\n",
 "pwr":  "  logic [31:0] v = 0;\n  initial begin #1 v[CALL:0] = '1; $display(\"pr=%h\", v); $finish; end\n",
 "plsb": "  logic [31:0] v = 32'h0000_0F00;\n  initial begin #1 $display(\"pl=%h\", v[11:CALL]); $finish; end\n",
 "mdpr": "  logic [15:0][3:0] m = 64'h0123456789ABCDEF;\n  initial begin #1 $display(\"m=%h\", m[CALL:0]); $finish; end\n",
 "tdly": "  `timescale 1ns/1ns\n",
 "cad":  "  logic a = 0; wire w;\n  assign #(CALL) w = a;\n  initial begin #1 a = 1; end\n  initial begin @(posedge w); $display(\"w at %0t\", $time); #1 $finish; end\n",
 "wd":   "  logic a = 0;\n  wire #(CALL) w = a;\n  initial begin #1 a = 1; end\n  initial begin @(posedge w); $display(\"w at %0t\", $time); #1 $finish; end\n",
 "gcl":  "  case (7)\n    CALL: begin : g7\n      initial begin #1 $display(\"gcl=label\"); $finish; end\n    end\n    default: begin : gd\n      initial begin #1 $display(\"gcl=default\"); $finish; end\n    end\n  endcase\n",
 "gcl10":"  case (10)\n    CALL: begin : g7\n      initial begin #1 $display(\"gcl=label\"); $finish; end\n    end\n    default: begin : gd\n      initial begin #1 $display(\"gcl=default\"); $finish; end\n    end\n  endcase\n",
 "dist": "  class C;\n    rand int x;\n    constraint c { x dist { [0:CALL] := 1 }; }\n  endclass\n  C c;\n  initial begin c = new; #1 void'(c.randomize()); $display(\"x_le7=%0d\", c.x <= f(1)); $finish; end\n",
}
for pos,tpl in P.items():
  for fn in FN:
    for r,arg in (("M",2),("N",1)):
      if pos=="tdly":
        src="`timescale 1ns/1ns\nmodule top;\n"+FN[fn]+f"  initial begin #(f({arg}) * 1ns) $display(\"t=%0t\", $time); $finish; end\nendmodule\n"
      else:
        src="module top;\n"+FN[fn]+tpl.replace("CALL",f"f({arg})")+"endmodule\n"
      open(f"h_{pos}_{fn}_{r}.sv","w").write(src)
