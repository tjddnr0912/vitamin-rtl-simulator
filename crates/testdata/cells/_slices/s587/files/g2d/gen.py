FN = {
 "if":   "  function automatic int f(input int a);\n    f = 7;\n    unique if (a == 1) f = 10;\n  endfunction\n",
 "case": "  function automatic int f(input int a);\n    f = 7;\n    unique case (a) 1: f = 10; 3: f = 30; endcase\n  endfunction\n",
}
# M: f(2)=7, N: f(1)=10; expressions chosen so M and N both land on a legal value
P = {
 "evlsb":"  logic [15:0] v = 0;\n  initial begin #1 v[0] = 1; end\n  initial begin @(posedge v[CALL - SUB]); $display(\"ev at %0t\", $time); #1 $finish; end\n",
 "evlvl":"  logic [15:0] v = 0;\n  initial begin #1 v[3] = 1; end\n  initial begin @(v[CALL - SUB + 3]); $display(\"lv at %0t\", $time); #1 $finish; end\n",
 "inoff":"  logic [3:0][15:0][3:0] m = '0;\n  initial begin m[1] = 64'h0123456789ABCDEF; #1 $display(\"m=%h\", m[1][CALL - SUB +: 2]); $finish; end\n",
 "cons": "  class C;\n    rand int x;\n    constraint c { x == CALL; }\n  endclass\n  C c;\n  initial begin c = new; #1 void'(c.randomize()); $display(\"x=%0d\", c.x); $finish; end\n",
 "dlo":  "  class C;\n    rand int x;\n    constraint c { x dist { CALL := 1 }; }\n  endclass\n  C c;\n  initial begin c = new; #1 void'(c.randomize()); $display(\"x=%0d\", c.x); $finish; end\n",
 "dhi":  "  class C;\n    rand int x;\n    constraint c { x dist { [CALL:CALL] := 1 }; }\n  endclass\n  C c;\n  initial begin c = new; #1 void'(c.randomize()); $display(\"x=%0d\", c.x); $finish; end\n",
 "dwt":  "  class C;\n    rand int x;\n    constraint c { x dist { 5 := CALL, 6 := 0 }; }\n  endclass\n  C c;\n  initial begin c = new; #1 void'(c.randomize()); $display(\"x=%0d\", c.x); $finish; end\n",
}
for pos,tpl in P.items():
  for fn in FN:
    for r,arg,sub in (("M",2,7),("N",1,10)):
      open(f"k_{pos}_{fn}_{r}.sv","w").write("module top;\n"+FN[fn]+tpl.replace("CALL",f"f({arg})").replace("SUB",str(sub))+"endmodule\n")
