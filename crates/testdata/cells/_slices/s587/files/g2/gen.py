FN = {
 "if":   "  function automatic int f(input int a);\n    f = 7;\n    unique if (a == 1) f = 10;\n  endfunction\n",
 "case": "  function automatic int f(input int a);\n    f = 7;\n    unique case (a) 1: f = 10; 3: f = 30; endcase\n  endfunction\n",
}
END = "endmodule\n"
D = lambda fmt, e: f"  initial begin #5 $display(\"{fmt}\", {e}); $finish; end\n"
P = {
 # continuous / net
 "ca":   "  int w;\n  assign w = CALL;\n"+D("w=%0d","w"),
 "nda":  "  wire [31:0] w = CALL;\n"+D("w=%0d","w"),
 "vinit":"  int v = CALL;\n"+D("v=%0d","v"),
 "vinit2":"  logic [31:0] v = CALL;\n"+D("v=%0d","v"),
 "proc": "  int x;\n  initial begin #1 x = CALL; end\n"+D("x=%0d","x"),
 "acomb":"  int x;\n  always_comb x = CALL;\n"+D("x=%0d","x"),
 "port": "module sub(input int p);\n  initial begin #5 $display(\"p=%0d\", p); $finish; end\nendmodule\nTOP  sub u(.p(CALL));\n",
 "disp": "  initial begin #1 $display(\"d=%0d\", CALL); $finish; end\n",
 "ifc":  "  initial begin #1 if (CALL == 7) $display(\"if=7\"); else $display(\"if=other\"); $finish; end\n",
 "casee":"  initial begin #1 case (CALL) 7: $display(\"ce=7\"); default: $display(\"ce=other\"); endcase $finish; end\n",
 "casei":"  int s = 7;\n  initial begin #1 case (s) CALL: $display(\"ci=match\"); default: $display(\"ci=other\"); endcase $finish; end\n",
 "bsel": "  logic [15:0] v = 16'h0080;\n  initial begin #1 $display(\"b=%0d\", v[CALL]); $finish; end\n",
 "psel": "  logic [31:0] v = 32'h00000180;\n  initial begin #1 $display(\"p=%0d\", v[CALL +: 2]); $finish; end\n",
 "uidx": "  int arr [16];\n  initial begin for (int i = 0; i < 16; i++) arr[i] = i * 3; #1 $display(\"u=%0d\", arr[CALL]); $finish; end\n",
 "sidx": "  string sa [16];\n  initial begin for (int i = 0; i < 16; i++) sa[i] = $sformatf(\"s%0d\", i); #1 $display(\"s=%s\", sa[CALL]); $finish; end\n",
 "lidx": "  int arr [16];\n  initial begin #1 arr[CALL] = 5; $display(\"l7=%0d l10=%0d\", arr[7], arr[10]); $finish; end\n",
 "lbit": "  logic [15:0] v;\n  initial begin v = 0; #1 v[CALL] = 1'b1; $display(\"lb=%h\", v); $finish; end\n",
 "rep":  "  logic [31:0] r;\n  initial begin #1 r = {CALL{1'b1}}; $display(\"r=%h\", r); $finish; end\n",
 "rpt":  "  int n;\n  initial begin n = 0; #1 repeat (CALL) n++; $display(\"n=%0d\", n); $finish; end\n",
 "dly":  "  initial begin #(CALL) $display(\"t=%0t\", $time); $finish; end\n",
 "forb": "  int n;\n  initial begin n = 0; #1 for (int i = 0; i < CALL; i++) n++; $display(\"n=%0d\", n); $finish; end\n",
 "blk":  "  initial begin : b\n    int t = CALL;\n    #1 $display(\"t=%0d\", t); $finish;\n  end\n",
 "blk2": "  initial begin #1 begin : b\n    int t = CALL;\n    $display(\"t=%0d\", t); $finish;\n  end end\n",
 "clog": "  int x;\n  initial begin #1 x = $clog2(CALL); $display(\"c=%0d\", x); $finish; end\n",
 "sizd": "  int a2 [4][8];\n  initial begin #1 $display(\"sz=%0d\", $size(a2, CALL - 6)); $finish; end\n",
 "evix": "  logic [15:0] v = 0;\n  initial begin #1 v[7] = 1; v[10] = 1; end\n  initial begin @(posedge v[CALL]); $display(\"ev at %0t\", $time); #5 $finish; end\n",
 "insd": "  int s = 7;\n  initial begin #1 if (s inside {CALL}) $display(\"in=1\"); else $display(\"in=0\"); $finish; end\n",
 "asrt": "  initial begin #1 assert (CALL == 7) $display(\"a=pass\"); else $display(\"a=fail\"); $finish; end\n",
 "dflt": "  function automatic int g(input int k = CALL);\n    g = k;\n  endfunction\n  initial begin #1 $display(\"g=%0d\", g()); $finish; end\n",
 "finit":"  function automatic int g(input int k);\n    int t = CALL;\n    g = k + t;\n  endfunction\n  initial begin #1 $display(\"g=%0d\", g(0)); $finish; end\n",
 "sinit":"  function int g(input int k);\n    static int t = CALL;\n    g = k + t;\n  endfunction\n  initial begin #1 $display(\"g=%0d\", g(0)); $finish; end\n",
 "cast": "  logic [31:0] x = 32'hFFFF_FFFF;\n  initial begin #1 $display(\"cw=%0d\", $bits(CALL'(x[3:0]))); $finish; end\n",
 "strm": "  logic [15:0] x = 16'h1234;\n  logic [15:0] y;\n  initial begin #1 y = {<< CALL {x}}; $display(\"st=%h\", y); $finish; end\n",
 "sfmt": "  string s;\n  initial begin #1 s = $sformatf(\"%0d\", CALL); $display(\"sf=%s\", s); $finish; end\n",
 "elabt":"  if (1) begin : g\n    $info(\"el=%0d\", CALL);\n  end\n  initial begin #1 $finish; end\n",
 "genca":"  if (1) begin : g\n    int w;\n    assign w = CALL;\n  end\n"+D("w=%0d","g.w"),
 "tern": "  int x;\n  initial begin #1 x = 1 ? CALL : 3; $display(\"x=%0d\", x); $finish; end\n",
 "arith":"  int x;\n  initial begin #1 x = CALL + 1; $display(\"x=%0d\", x); $finish; end\n",
 "ffx":  "  logic clk = 0; int x;\n  always @(posedge clk) x <= CALL;\n  initial begin #1 clk = 1; end\n"+D("x=%0d","x"),
 "nb":   "  int x;\n  initial begin #1 x <= CALL; end\n"+D("x=%0d","x"),
 "wcond":"  int n;\n  initial begin n = 0; #1 while (n < CALL) n++; $display(\"n=%0d\", n); $finish; end\n",
 "waitc":"  int n = 0;\n  initial begin #1 n = 7; #1 n = 10; end\n  initial begin wait (n == CALL); $display(\"w at %0t\", $time); #5 $finish; end\n",
}
for pos, tpl in P.items():
  for fn in ("if","case"):
    for r,arg in (("M",2),("N",1)):
      src = tpl.replace("CALL", f"f({arg})")
      if "TOP" in src:
        src = src.replace("TOP", "module top;\n"+FN[fn]) + END
      else:
        src = "module top;\n"+FN[fn]+src+END
      open(f"c_{pos}_{fn}_{r}.sv","w").write(src)
