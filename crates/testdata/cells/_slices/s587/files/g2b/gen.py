FN = {
 "if":   "  function automatic int f(input int a);\n    f = 7;\n    unique if (a == 1) f = 10;\n  endfunction\n",
 "case": "  function automatic int f(input int a);\n    f = 7;\n    unique case (a) 1: f = 10; 3: f = 30; endcase\n  endfunction\n",
}
P = {
 "pswr": "  logic [31:0] v = 32'hFFFF_FFFF;\n  initial begin #1 $display(\"pw=%h\", v[0 +: CALL]); $finish; end\n",
 "psww": "  logic [31:0] v = 0;\n  initial begin #1 v[0 +: CALL] = '1; $display(\"pw=%h\", v); $finish; end\n",
 "irpt": "  logic clk = 0; int x, y = 5;\n  always #1 clk = ~clk;\n  initial begin x = repeat (CALL) @(posedge clk) y; $display(\"x=%0d t=%0t\", x, $time); $finish; end\n",
 "nrpt": "  logic clk = 0; int x, y = 5;\n  always #1 clk = ~clk;\n  initial begin x <= repeat (CALL) @(posedge clk) y; #40 $display(\"x=%0d\", x); $finish; end\n",
 "trpt": "  logic clk = 0;\n  always #1 clk = ~clk;\n  initial begin repeat (CALL) @(posedge clk); $display(\"t=%0t\", $time); $finish; end\n",
 "ktrpt":"  logic clk = 0;\n  always #1 clk = ~clk;\n  task automatic w;\n    repeat (CALL) @(posedge clk);\n  endtask\n  initial begin w(); $display(\"t=%0t\", $time); $finish; end\n",
 "ktrp2":"  int n;\n  task automatic w;\n    repeat (CALL) n++;\n  endtask\n  initial begin n = 0; #1 w(); $display(\"n=%0d\", n); $finish; end\n",
 "frpt": "  function automatic int g(input int k);\n    g = k;\n    repeat (CALL) g++;\n  endfunction\n  initial begin #1 $display(\"g=%0d\", g(0)); $finish; end\n",
 "qleft":"  int a2 [4][8];\n  initial begin #1 $display(\"l=%0d\", $left(a2, CALL - 6)); $finish; end\n",
 "qhigh":"  logic [3:0][7:0] p2;\n  initial begin #1 $display(\"h=%0d\", $high(p2, CALL - 6)); $finish; end\n",
 "qsizp":"  logic [3:0][7:0] p2;\n  initial begin #1 $display(\"s=%0d\", $size(p2, CALL - 6)); $finish; end\n",
 "td":   "  typedef logic [CALL:0] t_t;\n  t_t v;\n  initial begin #1 $display(\"b=%0d\", $bits(v)); $finish; end\n",
 "fret": "  function automatic logic [CALL:0] g();\n    g = '1;\n  endfunction\n  initial begin #1 $display(\"b=%0d\", $bits(g())); $finish; end\n",
 "fform":"  function automatic int g(input logic [CALL:0] p);\n    g = $bits(p);\n  endfunction\n  initial begin #1 $display(\"b=%0d\", g(0)); $finish; end\n",
 "barr": "  initial begin : b\n    int arr [CALL];\n    #1 $display(\"s=%0d\", $size(arr)); $finish;\n  end\n",
 "stm":  "  typedef struct packed { logic [CALL:0] a; logic b; } s_t;\n  s_t s;\n  initial begin #1 $display(\"b=%0d\", $bits(s)); $finish; end\n",
 "cg":   "  logic clk = 0; int v = 7;\n  covergroup cg_t @(posedge clk);\n    coverpoint v { bins b = {CALL}; }\n  endgroup\n  cg_t cg = new;\n  initial begin #1 clk = 1; #1 $display(\"cov=%0.1f\", cg.get_coverage()); $finish; end\n",
 "cons": "  class C;\n    rand int x;\n    constraint c { x == CALL; }\n  endclass\n  C c = new;\n  initial begin #1 void'(c.randomize()); $display(\"x=%0d\", c.x); $finish; end\n",
 "case2":"  int s = 7;\n  initial begin #1 unique case (s) CALL: $display(\"ci=match\"); 10: $display(\"ci=10\"); endcase $finish; end\n",
 "sel2": "  logic [15:0][3:0] m = 0;\n  initial begin #1 m[CALL] = 4'hA; $display(\"m=%h\", m); $finish; end\n",
 "sel3": "  logic [15:0][3:0] m = 64'h0123456789ABCDEF;\n  initial begin #1 $display(\"m=%h\", m[CALL +: 2]); $finish; end\n",
}
for pos,tpl in P.items():
  for fn in FN:
    for r,arg in (("M",2),("N",1)):
      open(f"e_{pos}_{fn}_{r}.sv","w").write("module top;\n"+FN[fn]+tpl.replace("CALL",f"f({arg})")+"endmodule\n")
