# Test designs for crates/cli/tests/case_inside.rs — single source of truth.
# Each entry: name -> SV source. `write()` dumps them to <dir>/<name>.sv.
import os, sys

TS = "`timescale 1ns/1ns\n"

def loop(decl, vals, body, disp):
    """module-scope initial: a for loop over the values of `v` (the element type is the
    one `decl` gives `v`), each running `body` and `disp`."""
    ty = decl.split(" v;")[0]
    n = len(vals)
    arr = ", ".join(x.split("= ", 1)[1] for x in vals)
    body = "\n".join("  " + l for l in body.split("\n"))
    return (TS + "module t;\n"
            f"  {decl}\n"
            f"  {ty} vals [0:{n - 1}] = '{{{arr}}};\n"
            "  initial begin\n"
            f"    for (int i = 0; i < {n}; i++) begin\n"
            "      v = vals[i]; m = 9;\n"
            f"{body}\n"
            f"      {disp}\n"
            "    end\n"
            "    $finish;\n  end\nendmodule\n")

D = {}

D["v01"] = loop("logic [3:0] v; int m;",
    [f"v = 4'b{x}" for x in ["1000", "0010", "0110", "1100", "0000", "0011", "0100", "0001"]],
    "    case (v) inside\n      4'b1?00: m = 1;\n      [4'd1:4'd3]: m = 2;\n      default: m = 0;\n    endcase",
    '$display("v=%b m=%0d", v, m);')

D["v02"] = loop("logic [3:0] v; int m;",
    [f"v = 4'b{x}" for x in ["1000", "1001", "1100", "1101", "0001", "0111", "0011", "0000", "1010", "1011"]],
    "    case (v) inside 4'b1x0z: m = 1; 4'b0zz1: m = 2; 4'b??11: m = 3; default: m = 0; endcase",
    '$display("v=%b m=%0d", v, m);')

D["v03"] = loop("logic [3:0] v; int m;",
    [f"v = 4'b{x}" for x in ["1x00", "x100", "011x", "100z", "xxxx", "1z00", "1001", "zzzz", "10x1"]],
    "    case (v) inside 4'b1?00: m = 1; 4'b0110: m = 2; [4'd8:4'd9]: m = 3; default: m = 0; endcase",
    '$display("v=%b m=%0d", v, m);')

D["v04"] = loop("logic signed [3:0] v; int m;",
    [f"v = {x}" for x in ["-4'sd2", "-4'sd1", "4'sd0", "4'sd1", "4'sd2", "4'sd3", "-4'sd8", "4'sd7"]],
    "    case (v) inside [-4'sd2:4'sd1]: m = 1; [4'sd3:4'sd5]: m = 2; default: m = 0; endcase",
    '$display("v=%0d m=%0d", v, m);')

# c06a narrow E vs wide items; c06b wide E vs narrow items (two statements, two variables)
D["v05"] = (TS + """module t;
  logic [3:0] v; logic [7:0] w; int m;
  task automatic a(input logic [3:0] x);
    v = x; m = 9;
    case (v) inside 8'b0000_1?00: m = 1; 8'h1F: m = 2; [8'd5:8'd6]: m = 3; 8'b1???_??11: m = 4; default: m = 0; endcase
    $display("a v=%b m=%0d", v, m);
  endtask
  task automatic b(input logic [7:0] x);
    w = x; m = 9;
    case (w) inside 4'b1?00: m = 1; [4'd1:4'd2]: m = 2; default: m = 0; endcase
    $display("b w=%h m=%0d", w, m);
  endtask
  initial begin
    a(4'b1100); a(4'b1000); a(4'b1111); a(4'd5); a(4'd6); a(4'b0011);
    b(8'h0C); b(8'h1C); b(8'h08); b(8'h01); b(8'h11); b(8'hF8);
    $finish;
  end
endmodule
""")

# c07 reversed range, c10 multi-label, c11 overlap first match, c12 no default
D["v06"] = (TS + """module t;
  logic [3:0] v; int m;
  task automatic rev(input logic [3:0] x);
    v = x; m = 9;
    case (v) inside [4'd3:4'd1]: m = 1; default: m = 0; endcase
    $display("rev v=%0d m=%0d", v, m);
  endtask
  task automatic multi(input logic [3:0] x);
    v = x; m = 9;
    case (v) inside 4'd1, 4'd3, [4'd8:4'd9]: m = 1; 4'd2, 4'b11??: m = 2; default: m = 0; endcase
    $display("multi v=%0d m=%0d", v, m);
  endtask
  task automatic overlap(input logic [3:0] x);
    v = x; m = 9;
    case (v) inside [4'd0:4'd7]: m = 1; 4'd5: m = 2; 4'b01??: m = 3; 4'b1???: m = 4; default: m = 0; endcase
    $display("overlap v=%0d m=%0d", v, m);
  endtask
  task automatic nodef(input logic [3:0] x);
    v = x; m = 9;
    case (v) inside 4'd1: m = 1; [4'd2:4'd3]: m = 2; endcase
    $display("nodef v=%b m=%0d", v, m);
  endtask
  initial begin
    rev(4'd1); rev(4'd2); rev(4'd3);
    multi(4'd1); multi(4'd3); multi(4'd8); multi(4'd9); multi(4'd2); multi(4'd12); multi(4'd15); multi(4'd4);
    overlap(4'd5); overlap(4'd6); overlap(4'd8); overlap(4'd9);
    nodef(4'd1); nodef(4'd2); nodef(4'd4); nodef(4'bxx00);
    $finish;
  end
endmodule
""")

# c20a/c20b constant case expression; c32 72-bit; c37 all-? item; c38 empty arm; c40 'bx1 vs 36-bit
D["v07"] = (TS + """module t;
  localparam logic [3:0] P = 4'b1000;
  logic [71:0] w72; logic [3:0] v; logic [35:0] w36; int m;
  initial begin
    m = 9; case (4'b1100) inside 4'b1?00: m = 1; [4'd1:4'd2]: m = 2; default: m = 0; endcase
    $display("const m=%0d", m);
    m = 9; case (P) inside 4'b1?00: m = 1; [4'd1:4'd2]: m = 2; default: m = 0; endcase
    $display("param m=%0d", m);
    w72 = 72'h80_0000_0000_0000_0050; m = 9;
    case (w72) inside 72'h80_0000_0000_0000_00?0: m = 1; [72'h1:72'h3]: m = 2; default: m = 0; endcase
    $display("w72=%h m=%0d", w72, m);
    w72 = 72'h80_0000_0000_0000_0051; m = 9;
    case (w72) inside 72'h80_0000_0000_0000_00?0: m = 1; [72'h1:72'h3]: m = 2; default: m = 0; endcase
    $display("w72=%h m=%0d", w72, m);
    w72 = 72'h2; m = 9;
    case (w72) inside 72'h80_0000_0000_0000_00?0: m = 1; [72'h1:72'h3]: m = 2; default: m = 0; endcase
    $display("w72=%h m=%0d", w72, m);
    w72 = 72'h0; m = 9;
    case (w72) inside 72'h80_0000_0000_0000_00?0: m = 1; [72'h1:72'h3]: m = 2; default: m = 0; endcase
    $display("w72=%h m=%0d", w72, m);
    v = 4'bxxxx; m = 9; case (v) inside 4'b????: m = 1; default: m = 0; endcase $display("allq v=%b m=%0d", v, m);
    v = 4'bzzzz; m = 9; case (v) inside 4'b????: m = 1; default: m = 0; endcase $display("allq v=%b m=%0d", v, m);
    v = 4'd3;    m = 9; case (v) inside 4'b????: m = 1; default: m = 0; endcase $display("allq v=%b m=%0d", v, m);
    w36 = 36'h1; m = 9; case (w36) inside 'bx1: m = 1; default: m = 0; endcase $display("bx1 w36=%h m=%0d", w36, m);
    w36 = 36'h3; m = 9; case (w36) inside 'bx1: m = 1; default: m = 0; endcase $display("bx1 w36=%h m=%0d", w36, m);
    w36 = 36'h2; m = 9; case (w36) inside 'bx1: m = 1; default: m = 0; endcase $display("bx1 w36=%h m=%0d", w36, m);
    $finish;
  end
endmodule
""")

D["v07e"] = loop("logic [3:0] v; int m;", ["v = 4'd1", "v = 4'd2", "v = 4'd3"],
    "    case (v) inside 4'd1: ; 4'd2: m = 2; default: m = 0; endcase",
    '$display("empty v=%0d m=%0d", v, m);')

# q1 A: 64-bit E with [0:3] and 32'sd7 (non-negative signed constants)
D["v08"] = (TS + """module t;
  logic [63:0] v; int m;
  task automatic t1(input logic [63:0] x);
    v = x;
    case (v) inside [0:3]: m = 1; 32'sd7: m = 2; default: m = 0; endcase
    $display("v=%h m=%0d", v, m);
  endtask
  initial begin
    t1(64'd2); t1(64'd7); t1(64'd5); t1(64'hFFFFFFFF_FFFFFFFF); t1(64'h1_00000002);
    $finish;
  end
endmodule
""")

# q2: a signed operator item at the case width, unsigned and signed E
D["v09"] = (TS + """module t;
  logic [7:0] v; logic signed [7:0] s; int m;
  task automatic tu(input logic [7:0] x);
    v = x;
    case (v) inside (-4'sd8 + 8'sd0): m = 1; default: m = 0; endcase
    $display("u v=%h m=%0d", v, m);
  endtask
  task automatic ts(input logic signed [7:0] x);
    s = x;
    case (s) inside (-4'sd8 + 8'sd0): m = 1; default: m = 0; endcase
    $display("s s=%h m=%0d", s, m);
  endtask
  initial begin
    tu(8'hF8); tu(8'h08); ts(8'shF8); ts(8'sh08);
    $finish;
  end
endmodule
""")

# m1 sub-cells 2-4: operator items evaluated at the pair width
D["v10"] = (TS + """module t;
  logic [7:0] v8; bit [3:0] a4, b4; int m;
  initial begin
    a4 = 4'h8; b4 = 4'h8; v8 = 8'h10;
    case (v8) inside a4 + b4: m = 1; default: m = 0; endcase $display("sum m=%0d", m);
    case (v8) inside [a4 + b4 : 8'hFF]: m = 1; default: m = 0; endcase $display("sumrange m=%0d", m);
    v8 = 8'hF7;
    case (v8) inside ~a4: m = 1; default: m = 0; endcase $display("notF7 m=%0d", m);
    v8 = 8'h07;
    case (v8) inside ~a4: m = 1; default: m = 0; endcase $display("not07 m=%0d", m);
    $finish;
  end
endmodule
""")

D["v10x"] = (TS + """module t;
  logic [7:0] v8; logic [3:0] a4, b4; int m;
  initial begin
    a4 = 4'h8; b4 = 4'h8; v8 = 8'h10;
    case (v8) inside [a4 + b4 : 8'hFF]: m = 1; default: m = 0; endcase $display("sumrange m=%0d", m);
    $finish;
  end
endmodule
""")
one_x = None

# m7: enum labels and a string case expression (verilator only)
D["v11"] = (TS + """module t;
  typedef enum logic [2:0] {R_F, I_F, S_F, B_F, U_F, J_F} fmt_t;
  string s; fmt_t f; int m;
  initial begin
    s = "ab";
    case (s) inside "aa", "ab": m = 1; default: m = 0; endcase $display("s=%s m=%0d", s, m);
    s = "zz";
    case (s) inside "aa", "ab": m = 1; default: m = 0; endcase $display("s=%s m=%0d", s, m);
    for (int i = 0; i < 6; i++) begin
      f = fmt_t'(i);
      case (f) inside R_F: m = 1; S_F, B_F: m = 2; [U_F:J_F]: m = 3; default: m = 0; endcase
      $display("f=%0d m=%0d", f, m);
    end
    $finish;
  end
endmodule
""")

# string-only cell (so sv2v can be asked)
D["v11s"] = (TS + """module t;
  string s; int m;
  initial begin
    s = "ab";
    case (s) inside "aa", "ab": m = 1; default: m = 0; endcase $display("s=%s m=%0d", s, m);
    s = "zz";
    case (s) inside "aa", "ab": m = 1; default: m = 0; endcase $display("s=%s m=%0d", s, m);
    $finish;
  end
endmodule
""")

# v2 split: 2-state variable items (bit, int), and a range with an operator bound at the case width
D["v12"] = (TS + """module t;
  logic [3:0] v; int m; bit [3:0] k2; int ki;
  initial begin
    k2 = 4'd3; ki = 5;
    for (int i = 0; i < 9; i++) begin
      v = i[3:0];
      case (v) inside k2: m = 1; ki: m = 2; default: m = 0; endcase
      $display("var v=%0d m=%0d", v, m);
    end
    for (int i = 6; i < 11; i++) begin
      v = i[3:0];
      case (v) inside [k2 + 4'd5 : 4'd9]: m = 3; default: m = 0; endcase
      $display("rng v=%0d m=%0d", v, m);
    end
    $finish;
  end
endmodule
""")

# E01: the case expression is evaluated exactly once (m2: f writes a module net)
D["e01"] = (TS + """module t;
  int cnt = 0; int m;
  function automatic logic [3:0] f(input int n); cnt++; $display("f(%0d) call %0d", n, cnt); return n[3:0]; endfunction
  initial begin
    case (f(3)) inside 4'd1: m = 1; [4'd5:4'd6]: m = 2; 4'b1?00: m = 3; 4'd3: m = 4; default: m = 0; endcase
    $display("A m=%0d cnt=%0d", m, cnt);
    cnt = 0;
    case (f(9)) inside 4'd1, 4'd2: m = 1; default: m = 0; endcase
    $display("B m=%0d cnt=%0d", m, cnt);
    cnt = 0;
    case (f(7)) inside default: m = 5; endcase
    $display("C m=%0d cnt=%0d", m, cnt);
    $finish;
  end
endmodule
""")

# E02: pure f (m2b) and a function / task body (f1)
D["e02"] = (TS + """module t;
  int m;
  function automatic logic [3:0] g(input int n); $display("g(%0d)", n); return n[3:0]; endfunction
  function automatic int fn(input int n);
    case (g(n)) inside 4'd1: fn = 1; [4'd5:4'd6]: fn = 2; 4'b1?00: fn = 3; 4'd3: fn = 4; default: fn = 0; endcase
  endfunction
  task automatic tk(input int n, output int o);
    case (g(n)) inside 4'd1: o = 1; [4'd5:4'd6]: o = 2; 4'b1?00: o = 3; 4'd3: o = 4; default: o = 0; endcase
  endtask
  initial begin
    case (g(3)) inside 4'd1: m = 1; [4'd5:4'd6]: m = 2; 4'b1?00: m = 3; 4'd3: m = 4; default: m = 0; endcase
    $display("mod m=%0d", m);
    case (g(7)) inside default: m = 5; endcase
    $display("defonly m=%0d", m);
    m = fn(3); $display("fn m=%0d", m);
    tk(3, m); $display("tk m=%0d", m);
    $finish;
  end
endmodule
""")

# C01 m4 contexts
D["c01"] = (TS + """module t;
  logic clk = 0; logic [3:0] v; int mc, mf, mt; logic [7:0] r;
  class C;
    function int meth(logic [3:0] x);
      case (x) inside 4'b1?00: return 1; [4'd1:4'd3]: return 2; default: return 0; endcase
    endfunction
  endclass
  function automatic int fn(input logic [3:0] x);
    case (x) inside 4'b1?00: fn = 1; [4'd1:4'd3]: fn = 2; default: fn = 0; endcase
  endfunction
  task automatic tk(input logic [3:0] x, output int o);
    case (x) inside 4'b1?00: o = 1; [4'd1:4'd3]: o = 2; default: o = 0; endcase
  endtask
  always_comb begin
    case (v) inside 4'b1?00: mc = 1; [4'd1:4'd3]: mc = 2; default: mc = 0; endcase
  end
  always_ff @(posedge clk) begin
    case (v) inside 4'b1?00: r <= 8'hAB; [4'd1:4'd3]: r <= 4'h5; default: r <= 16'hFFEE; endcase
  end
  logic [3:0] vals [0:4] = '{4'b1000, 4'b0010, 4'b0110, 4'b1100, 4'b0011};
  initial begin
    C c; c = new;
    for (int i = 0; i < 5; i++) begin
      v = vals[i]; #1 clk = 1; #1 clk = 0;
      tk(v, mt); mf = fn(v);
      case (v) inside
        [4'd0:4'd7]: case (v) inside 4'b0?10: $display("nest A"); default: $display("nest B"); endcase
        default: $display("nest C");
      endcase
      $display("v=%b comb=%0d ff=%h fn=%0d task=%0d meth=%0d", v, mc, r, mf, mt, c.meth(v));
    end
    $finish;
  end
endmodule
""")

# C02: a function containing a case inside, called from a continuous assign
D["c02"] = (TS + """module t;
  logic [3:0] x; logic [31:0] y;
  function automatic int fn(input logic [3:0] a);
    case (a) inside 4'b1?00: fn = 1; [4'd1:4'd3]: fn = 2; default: fn = 0; endcase
  endfunction
  assign y = fn(x);
  logic [3:0] vals [0:4] = '{4'b1000, 4'b0010, 4'b0110, 4'b1100, 4'b0011};
  initial begin
    for (int i = 0; i < 5; i++) begin x = vals[i]; #1 $display("x=%b y=%0d", x, y); end
    $finish;
  end
endmodule
""")

# C03 a1: always_comb sensitivity to range-bound variables
D["c03"] = (TS + """module t;
  logic [3:0] v, lo, hi; int m;
  always_comb begin
    case (v) inside [lo:hi]: m = 1; 4'b1?00: m = 2; default: m = 0; endcase
  end
  initial begin
    v = 4'd5; lo = 4'd1; hi = 4'd3;
    #1 $display("t1 m=%0d", m);
    hi = 4'd6;
    #1 $display("t2 m=%0d", m);
    lo = 4'd6;
    #1 $display("t3 m=%0d", m);
    v = 4'd12;
    #1 $display("t4 m=%0d", m);
    $finish;
  end
endmodule
""")

def uq(name, qual, items, vals, default=None):
    d = f" default: m = {default};" if default is not None else ""
    body = "".join(f"    v = {x}; m = 9; {qual} case (v) inside {items}{d} endcase $display(\"v=%b m=%0d\", v, m);\n" for x in vals)
    D[name] = TS + "module t;\n  logic [3:0] v; int m;\n  initial begin\n" + body + "    $finish;\n  end\nendmodule\n"
uq("u_a", "unique", "4'd1: m = 1; [4'd2:4'd3]: m = 2;", ["4'd1", "4'd5"])
uq("u_b", "unique", "[4'd0:4'd7]: m = 1; 4'd5: m = 2;", ["4'd5", "4'd1"])
uq("u_c", "unique0", "4'd1: m = 1; [4'd2:4'd3]: m = 2;", ["4'd1", "4'd5"])
uq("u_d", "priority", "4'd1: m = 1; [4'd2:4'd3]: m = 2;", ["4'd1", "4'd5"])
uq("u_e", "priority", "[4'd0:4'd7]: m = 1; 4'd5: m = 2;", ["4'd5"])
uq("u_f", "unique", "4'd1: m = 1; 4'b1?00: m = 2;", ["4'd1", "4'd12", "4'd3"], default=0)
uq("u_g", "unique", "4'd1: m = 1; 4'b1?00: m = 2;", ["4'bx001", "4'b1x00"])

# F1 (round 2): a declared identifier named `inside` after `case (…)`.
D["f1a"] = TS + """module t;
  reg [3:0] x; reg [3:0] inside; reg [3:0] m1, m2, m3;
  initial begin
    inside = 4'b0110;
    x = 4'd3; case (x) inside[2:1]: m1 = 1; default: m1 = 0; endcase
    x = 4'd7; case (x) inside + 1: m2 = 1; default: m2 = 0; endcase
    x = 4'd6; case (x) inside & 4'hF: m3 = 1; default: m3 = 0; endcase
    $display("A m1=%0d B m2=%0d C m3=%0d", m1, m2, m3);
    $finish;
  end
endmodule
"""
D["f1b"] = TS + """module t;
  reg [3:0] x, inside; reg [3:0] m1, m2;
  initial begin
    inside = 4'd5; x = 4'd5;
    case (x) inside: m1 = 1; default: m1 = 0; endcase
    case (x) inside, 4'd9: m2 = 1; default: m2 = 0; endcase
    $display("D m1=%0d E m2=%0d", m1, m2);
    $finish;
  end
endmodule
"""
D["f1c"] = TS + """module t;
  reg [3:0] x, inside; reg [3:0] m1, m2;
  initial begin
    inside = 4'b0110; x = 4'd3;
    casez (x) inside[2:1]: m1 = 1; default: m1 = 0; endcase
    casex (x) inside[2:1]: m2 = 1; default: m2 = 0; endcase
    $display("F m1=%0d m2=%0d", m1, m2);
    $finish;
  end
endmodule
"""
D["f1d"] = TS + """module t;
  reg [3:0] x, m2;
  function [3:0] inside; input [3:0] v; inside = v + 4'd1; endfunction
  initial begin
    x = 4'd4;
    case (x) inside(3): m2 = 1; default: m2 = 0; endcase
    $display("G m2=%0d", m2);
    $finish;
  end
endmodule
"""
D["f1e"] = "`begin_keywords \"1364-2005\"\n" + TS + """module t;
  reg [3:0] x; reg [3:0] inside; reg [3:0] m1;
  initial begin
    inside = 4'b0110; x = 4'd3;
    case (x) inside[2:1]: m1 = 1; default: m1 = 0; endcase
    $display("A m1=%0d", m1);
    $finish;
  end
endmodule
`end_keywords
"""
# a binary-only operator after the word: the narrow parser rule takes `inside`
D["f1f"] = TS + """module t;
  reg [3:0] x, inside; reg [3:0] m1;
  initial begin
    inside = 4'd5; x = 4'd1;
    case (x) inside == 4'd5: m1 = 1; default: m1 = 0; endcase
    $display("H m1=%0d", m1);
    $finish;
  end
endmodule
"""

# Round 3: design-wide decline when `inside` is a NAME anywhere in the design.
D["f2a"] = TS + """module child(output reg [3:0] m);
  reg [3:0] x;
  initial begin x = 4'd4; case (x) inside(3): m = 1; default: m = 0; endcase end
endmodule
module t;
  function [3:0] inside; input [3:0] v; inside = v + 4'd1; endfunction
  wire [3:0] r;
  child u(r);
  initial begin #1 $display("up m=%0d", r); $finish; end
endmodule
"""
D["f2b"] = TS + """module t;
  logic [3:0] x; int m;
  initial begin
    x = 7;
    for (int inside = 6; inside < 7; inside++) begin case (x) inside + 1: m = 1; default: m = 0; endcase end
    $display("forvar m=%0d", m);
    $finish;
  end
endmodule
"""
D["esc1"] = TS + """module t;
  reg [3:0] x; reg [3:0] \\inside ; int m1;
  initial begin
    \\inside = 4'b0110; x = 4'd3;
    case (x) \\inside [2:1]: m1 = 1; default: m1 = 0; endcase
    $display("esc m1=%0d", m1);
    $finish;
  end
endmodule
"""
D["esc2"] = TS + """module t;
  reg [3:0] x, y; reg [3:0] \\inside ; int m1, m2;
  initial begin
    \\inside = 4'b0110; x = 4'd3; y = 4'd2;
    case (x) \\inside [2:1]: m1 = 1; default: m1 = 0; endcase
    case (y) inside [4'd1:4'd3]: m2 = 1; default: m2 = 0; endcase
    $display("esc m1=%0d m2=%0d", m1, m2);
    $finish;
  end
endmodule
"""
D["ctl"] = TS + """module t;
  logic [3:0] v; int m, k;
  logic [3:0] vals [0:3] = '{4'b1000, 4'b0010, 4'b0110, 4'b1100};
  initial begin
    for (int i = 0; i < 4; i++) begin
      v = vals[i];
      case (v) inside 4'b1?00: m = 1; [4'd1:4'd3]: m = 2; default: m = 0; endcase
      k = v inside {4'b1?00, [4'd1:4'd3]};
      $display("v=%b m=%0d k=%0d", v, m, k);
    end
    $finish;
  end
endmodule
"""
# cross-file: `inside` declared in file A (a package parameter), the case inside in file B
XA = """package p;
  parameter int inside = 5;
endpackage
"""
XB = TS + """module t;
  import p::*;
  logic [3:0] x; int m;
  initial begin
    x = 4'd5;
    case (x) inside [4'd4:4'd6]: m = 1; default: m = 0; endcase
    $display("x m=%0d", m);
    $finish;
  end
endmodule
"""
D["xa"] = XA
D["xb"] = XB

def one(name, decl, stmt_and_disp, pre=""):
    D[name] = TS + f"module t;\n  {decl}\n{pre}  initial begin\n{stmt_and_disp}\n    $finish;\n  end\nendmodule\n"

one("l01", "logic signed [3:0] v; int m;", "    v = -4'sd1; case (v) inside -1: m = 1; 8'h00: m = 2; default: m = 0; endcase $display(\"m=%0d\", m);")
one("l02", "logic [7:0] a, b; int m;", "    a = 8'h80; b = 8'h80; case (a + b) inside 8'h00: m = 1; 16'h0100: m = 2; default: m = 0; endcase $display(\"m=%0d\", m);")
one("l03", "logic signed [3:0] v; int m;", "    v = -4'sd1; case (v) inside [-2:-1]: m = 1; [4'd0:4'd1]: m = 2; default: m = 0; endcase $display(\"m=%0d\", m);")
one("l04", "logic [3:0] v; int m;", "    v = 4'd15; case (v) inside 1, 3: m = 1; [4:7]: m = 2; '1: m = 3; default: m = 0; endcase $display(\"m=%0d\", m);")
one("l05", "logic [63:0] v; int m;", "    v = 64'h00000000_FFFFFFFF; case (v) inside 32'shFFFFFFFF: m = 3; default: m = 0; endcase $display(\"m=%0d\", m);")
one("l06", "logic [3:0] v; int m; logic [3:0] w = 4'b1x00;", "    v = 4'b1000; case (v) inside w: m = 1; default: m = 0; endcase $display(\"m=%0d\", m);")
one("l07", "logic [3:0] v; int m;", "    v = 4'b1000; case (v) inside {2'b1?, 2'b00}: m = 1; default: m = 0; endcase $display(\"m=%0d\", m);")
one("l07b", "logic [7:0] v8; logic [3:0] a4, b4; int m;", "    a4 = 4'h8; b4 = 4'h8; v8 = 8'h10; case (v8) inside a4 + b4: m = 1; default: m = 0; endcase $display(\"m=%0d\", m);")
one("l08", "real r; int m;", "    r = 2.5; case (r) inside 1.0: m = 1; [2.0:3.0]: m = 2; default: m = 0; endcase $display(\"m=%0d\", m);")
one("l09", "string s; int m;", "    s = \"am\"; case (s) inside [\"aa\":\"az\"]: m = 1; \"b\": m = 2; default: m = 0; endcase $display(\"m=%0d\", m);")
one("l10", "int m;", "    case (sf(1)) inside \"b\": m = 3; default: m = 0; endcase $display(\"m=%0d\", m);",
    pre="  function automatic string sf(input int n); $display(\"sf(%0d)\", n); return \"b\"; endfunction\n")
one("l11", "logic [3:0] v; int m;", "    v = 4'd5; case (v) inside g(1), g(2): m = 1; [g(3):g(6)]: m = 2; g(5): m = 3; default: m = 0; endcase $display(\"m=%0d\", m);",
    pre="  function automatic logic [3:0] g(input int n); $display(\"g(%0d)\", n); return n[3:0]; endfunction\n")
one("l11r", "logic [3:0] v; int m;", "    v = 4'd5; case (v) inside [g(3):g(6)]: m = 2; default: m = 0; endcase $display(\"m=%0d\", m);",
    pre="  function automatic logic [3:0] g(input int n); $display(\"g(%0d)\", n); return n[3:0]; endfunction\n")
one("l12", "logic [3:0] v; int m;", "    v = 4'd15; case (v) inside [4'd12:$]: m = 1; default: m = 0; endcase $display(\"m=%0d\", m);")
one("l13", "int m;", "    case (f(-1)) inside [-4:4]: m = 1; default: m = 0; endcase $display(\"m=%0d\", m);",
    pre="  function int f(input int x); return x; endfunction\n")
D["l14"] = TS + """module t;
  class C;
    function logic [3:0] g(int n); $display("g(%0d)", n); return n[3:0]; endfunction
    function int meth(int n);
      case (g(n)) inside 4'd1: return 1; [4'd5:4'd6]: return 2; 4'd3: return 4; default: return 0; endcase
    endfunction
  endclass
  initial begin
    C c; c = new;
    $display("meth m=%0d", c.meth(3));
    $finish;
  end
endmodule
"""
D["l15"] = TS + """module t;
  class C; int x; endclass
  C h; int m;
  initial begin
    h = null;
    case (h) inside null: m = 1; default: m = 0; endcase
    $display("m=%0d", m);
    $finish;
  end
endmodule
"""
one("l16a", "logic signed [3:0] v; int m;", "    v = -4'sd1; case (v) inside [4'd1:4'd3]: m = 1; [-1:1]: m = 2; default: m = 0; endcase $display(\"m=%0d\", m);")
one("l16b", "logic signed [7:0] v; int m;", "    v = 8'shFC; case (v) inside 4'sb1?00: m = 1; 4'b1?11: m = 2; default: m = 0; endcase $display(\"m=%0d\", m);")
one("p01z", "logic [3:0] v; int m;", "    v = 4'd1; casez (v) inside 4'd1: m = 1; default: m = 0; endcase $display(\"m=%0d\", m);")
one("p01x", "logic [3:0] v; int m;", "    v = 4'd1; casex (v) inside 4'd1: m = 1; default: m = 0; endcase $display(\"m=%0d\", m);")
D["k01g"] = TS + """module t;
  localparam P = 2;
  generate case (P) inside 1: begin : g1 initial $display("one"); end [2:3]: begin : g2 initial $display("two"); end endcase endgenerate
  initial #1 $finish;
endmodule
"""
D["k01c"] = TS + """module t;
  function automatic int f(input logic [3:0] x);
    case (x) inside [4'd1:4'd3]: f = 1; 4'b1?00: f = 2; default: f = 0; endcase
  endfunction
  localparam int P1 = f(4'd2);
  initial begin $display("P1=%0d", P1); $finish; end
endmodule
"""
one("k01a", "logic [3:0] v; int m; logic [3:0] arr [0:1] = '{4'd5, 4'd7};", "    v = 4'd5; case (v) inside arr: m = 1; 4'd2: m = 2; default: m = 0; endcase $display(\"m=%0d\", m);")
D["l17p"] = TS + """package p;
  function automatic int f(input logic [3:0] x);
    case (x) inside 4'b1?00: f = 1; [4'd1:4'd3]: f = 2; default: f = 0; endcase
  endfunction
endpackage
module t;
  initial begin
    $display("f=%0d", p::f(4'b1000));
    $finish;
  end
endmodule
"""
D["l17i"] = TS + """package p;
  function automatic int f(input logic [3:0] x);
    case (x) inside 4'b1?00: f = 1; [4'd1:4'd3]: f = 2; default: f = 0; endcase
  endfunction
endpackage
module t;
  import p::*;
  logic [3:0] vals [0:4] = '{4'b1000, 4'b0010, 4'b0110, 4'b1100, 4'b0011};
  initial begin
    for (int i = 0; i < 5; i++) $display("x=%b f=%0d", vals[i], f(vals[i]));
    $finish;
  end
endmodule
"""
one("d01", "logic [3:0] v; int m;", "    v = 4'd5; case (v) inside [4'd1:$]: m = 1; default: m = 0; endcase $display(\"m=%0d\", m);")
one("d02", "int q[$]; int x; int m;", "    q = '{1, 5, 9};\n    x = 9; case (x) inside [q[$]:q[$]]: m = 1; default: m = 0; endcase $display(\"x=%0d m=%0d\", x, m);\n    x = 5; case (x) inside [q[$]:q[$]]: m = 1; default: m = 0; endcase $display(\"x=%0d m=%0d\", x, m);")
one("d02v", "int q[$]; int x; int m;", "    q = '{1, 5, 9};\n    x = 9; case (x) inside q[$]: m = 1; default: m = 0; endcase $display(\"x=%0d m=%0d\", x, m);")
# string literal item under a packed case expression (unmeasured lane in the plan)
one("x01", "logic [15:0] v; int m;", "    v = 16'h6162; case (v) inside \"ab\": m = 1; default: m = 0; endcase $display(\"m=%0d\", m);\n    v = 16'h6163; case (v) inside \"ab\": m = 1; default: m = 0; endcase $display(\"m=%0d\", m);")
# MinTypMax operator case expression narrower than an item (guard sees through it)
one("x02", "logic [7:0] a, b; int m;", "    a = 8'h80; b = 8'h80; case ((1:a + b:2)) inside 9'h100: m = 1; default: m = 0; endcase $display(\"m=%0d\", m);")

WATCHDOG = "  initial #1000 $finish; // watchdog\n"

def src(k):
    return D[k].replace("module t;\n", "module t;\n" + WATCHDOG, 1)

def write(d):
    os.makedirs(d, exist_ok=True)
    for k in D:
        open(os.path.join(d, k + ".sv"), "w").write(src(k))

if __name__ == "__main__":
    write(sys.argv[1])
    print(len(D))
