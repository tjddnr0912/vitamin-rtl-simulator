#!/usr/bin/env python3
"""§4.5.591 grounding cells: one shadow binder pair x the lanes reading the key."""
import os, sys
OUT = sys.argv[1]
MODE = sys.argv[2] if len(sys.argv) > 2 else "main"
os.makedirs(OUT, exist_ok=True)
cells = {}

W65 = "65'h1_0000_0000_0000_0009"
SUB = """
module sub #(parameter P = 0) ();
  initial #3 $display("ovr %m P=%0d b=%0d", P, $bits(P));
endmodule
module sub2 #(parameter N = 0) (input logic [N:0] a);
  initial #3 $display("port %m b=%0d", $bits(a));
endmodule
"""

# ---------------- G: a genvar over a same-key constant ----------------
G_SHADOW = {
    "w":  ("  localparam [64:0] i = %s;\n" % W65, "%0d"),
    "s":  ('  localparam string i = "AB";\n', "%s"),
    "r":  ("  localparam real i = 2.5;\n", "%f"),
    "ri": ("  localparam real i = 4.0;\n", "%f"),
    "n":  ("  localparam i = 9;\n", "%0d"),
    "wf": ("  localparam [64:0] i = 65'd9;\n", "%0d"),
}
G_PKG = {
    # wildcard / explicit import of a wide constant named like the genvar
    "wi": ("package pk; localparam [64:0] i = %s; endpackage\n" % W65, "  import pk::*;\n", "%0d"),
    "we": ("package pk; localparam [64:0] i = %s; endpackage\n" % W65, "  import pk::i;\n", "%0d"),
    "si": ('package pk; localparam string i = "AB"; endpackage\n', "  import pk::*;\n", "%s"),
}

def g_cells(tag, pre, decl, fmt, post=True):
    body_val = (
        "    localparam int K = i;\n"
        "    localparam [64:0] KW = i;\n"
        "    wire [64:0] wv = i;\n"
        "    initial begin\n"
        '      #1 $display("val %m i=%0d K=%0d KW=%0d wv=%0d b=%0d", i, K, KW, wv, $bits(i));\n'
        '      case (i) 0: $display("pc %m zero"); 1: $display("pc %m one"); default: $display("pc %m def"); endcase\n'
        "    end\n")
    post_l = ('  initial #5 $display("post ' + fmt + '", i);\n') if post else ""
    def mk(name, body, extra_mod="", extra_top=""):
        cells[name] = (pre + "module top;\n" + decl + extra_top +
                       "  for (genvar i = 0; i < 2; i++) begin : g\n" + body + "  end\n" +
                       post_l + "  initial #100 $finish;\nendmodule\n" + extra_mod)
    mk(f"G{tag}_val", body_val)
    mk(f"G{tag}_sel", '    initial #1 $display("sel %m s0=%b s1=%b", i[0], i[1]);\n')
    mk(f"G{tag}_wid", '    logic [i+1:0] v;\n    initial #1 $display("wid %m b=%0d", $bits(v));\n')
    mk(f"G{tag}_gif", '    if (i == 1) begin : t initial #1 $display("gif %m hit"); end\n'
                      '    else begin : e initial #1 $display("gif %m miss"); end\n')
    mk(f"G{tag}_gcs", "    case (i)\n"
                      '      0: begin : z initial #1 $display("gcs %m zero"); end\n'
                      '      1: begin : o initial #1 $display("gcs %m one"); end\n'
                      '      default: begin : d initial #1 $display("gcs %m def"); end\n'
                      "    endcase\n")
    mk(f"G{tag}_ovr", "    sub #(.P(i)) u ();\n    sub2 #(.N(i)) u2 (.a('0));\n", SUB)
    mk(f"G{tag}_fn", "    function automatic int f(); return i + 0; endfunction\n"
                     '    initial #1 $display("fn %m f=%0d", f());\n')
    mk(f"G{tag}_cmp", '    initial #1 $display("cmp %m eq1=%0d lt=%0d", (i == 1), (i < 1));\n')

for tag, (decl, fmt) in G_SHADOW.items():
    g_cells(tag, "", decl, fmt)
for tag, (pkg, imp, fmt) in G_PKG.items():
    g_cells(tag, pkg, imp, fmt)

# nested: the constant and the genvar both in one generate block (same key top.b.i)
cells["Gwb_val"] = ("module top;\n  if (1) begin : b\n    localparam [64:0] i = %s;\n" % W65 +
    "    for (genvar i = 0; i < 2; i++) begin : g\n      localparam int K = i;\n"
    '      initial #1 $display("val %m i=%0d K=%0d b=%0d", i, K, $bits(i));\n    end\n'
    '    initial #5 $display("post %0d", i);\n  end\n  initial #100 $finish;\nendmodule\n')
cells["Gwb_gcs"] = ("module top;\n  if (1) begin : b\n    localparam [64:0] i = %s;\n" % W65 +
    "    for (genvar i = 0; i < 2; i++) begin : g\n      case (i)\n"
    '        0: begin : z initial #1 $display("gcs %m zero"); end\n'
    '        1: begin : o initial #1 $display("gcs %m one"); end\n'
    '        default: begin : d initial #1 $display("gcs %m def"); end\n      endcase\n    end\n'
    "  end\n  initial #100 $finish;\nendmodule\n")
# nested loops: inner genvar j under a wide j declared in the outer iteration
cells["Gwn_val"] = ("module top;\n  for (genvar i = 0; i < 2; i++) begin : g\n"
    "    localparam [64:0] j = %s;\n" % W65 +
    "    for (genvar j = 0; j < 2; j++) begin : h\n      localparam int K = j;\n"
    '      initial #1 $display("val %m j=%0d K=%0d", j, K);\n    end\n'
    '    initial #5 $display("post %m %0d", j);\n  end\n  initial #100 $finish;\nendmodule\n')
# control: the wide constant lives in a DIFFERENT (inner) scope, so no key collision
cells["Gwa_val"] = ("module top;\n  if (1) begin : h\n    localparam [64:0] i = %s;\n" % W65 +
    '    initial #5 $display("post %0d", i);\n  end\n'
    "  for (genvar i = 0; i < 2; i++) begin : g\n    localparam int K = i;\n"
    '    initial #1 $display("val %m i=%0d K=%0d", i, K);\n  end\n  initial #100 $finish;\nendmodule\n')

# ---------------- E: an enum label over a same-key constant ----------------
E_LANES_VAL = ('  localparam int K = E1;\n  localparam [64:0] KW = E1;\n  wire [64:0] wv = E1;\n'
    '  initial begin\n    #1 $display("val E1=%0d K=%0d KW=%0d wv=%0d b=%0d", E1, K, KW, wv, $bits(E1));\n'
    '    case (E1) 0: $display("pc zero"); 1: $display("pc one"); default: $display("pc def"); endcase\n  end\n')
E_LANES = {
    "val": E_LANES_VAL,
    "sel": '  initial #1 $display("sel s0=%b s1=%b", E1[0], E1[1]);\n',
    "wid": '  logic [E1+1:0] v;\n  initial #1 $display("wid b=%0d", $bits(v));\n',
    "gif": '  if (E1 == 1) begin : t initial #1 $display("gif hit"); end\n'
           '  else begin : e initial #1 $display("gif miss"); end\n',
    "gcs": '  case (E1)\n    0: begin : z initial #1 $display("gcs zero"); end\n'
           '    1: begin : o initial #1 $display("gcs one"); end\n'
           '    default: begin : d initial #1 $display("gcs def"); end\n  endcase\n',
    "ovr": "  sub #(.P(E1)) u ();\n  sub2 #(.N(E1)) u2 (.a('0));\n",
    "cmp": '  initial #1 $display("cmp eq1=%0d lt=%0d", (E1 == 1), (E1 < 1));\n',
}
E_CFG = {
    # (package text, module-scope text before the typedef)
    "wi": ("package pk; localparam [64:0] E1 = 65'h1_0000_0000_0000_0000; endpackage\n", "  import pk::*;\n"),
    "si": ('package pk; localparam string E1 = "AB"; endpackage\n', "  import pk::*;\n"),
    "ri": ("package pk; localparam real E1 = 2.5; endpackage\n", "  import pk::*;\n"),
    "ni": ("package pk; localparam E1 = 7; endpackage\n", "  import pk::*;\n"),
}
for tag, (pkg, pre) in E_CFG.items():
    for ln, body in E_LANES.items():
        extra = SUB if ln == "ovr" else ""
        cells[f"E{tag}_{ln}"] = (pkg + "module top;\n" + pre + "  typedef enum {E0, E1} e_t;\n" + body +
                                 "  initial #100 $finish;\nendmodule\n" + extra)
# illegal pairs (a duplicate declaration / an explicit import of a local name): oracles should refuse
cells["Elw_val"] = ("module top;\n  localparam [64:0] E1 = 65'h1_0000_0000_0000_0000;\n  typedef enum {E0, E1} e_t;\n" +
                    E_LANES_VAL + "  initial #100 $finish;\nendmodule\n")
cells["Ewe_val"] = ("package pk; localparam [64:0] E1 = 65'h1_0000_0000_0000_0000; endpackage\n"
                    "module top;\n  import pk::E1;\n  typedef enum {E0, E1} e_t;\n" + E_LANES_VAL +
                    "  initial #100 $finish;\nendmodule\n")
# a package-level label over a wildcard wide import, read from a module
cells["Epk_val"] = ("package pa; localparam [64:0] E1 = 65'h1_0000_0000_0000_0000; endpackage\n"
                    "package pb;\n  import pa::*;\n  typedef enum {E0, E1} e_t;\n  localparam int K = E1;\n"
                    "  localparam [64:0] KW = E1;\nendpackage\n"
                    "module top;\n  initial #1 $display(\"pk E1=%0d K=%0d KW=%0d\", pb::E1, pb::K, pb::KW);\n"
                    "  initial #100 $finish;\nendmodule\n")
# a routine-body enum label under a module constant of the same name (inline lanes register at the caller key)
for tag, decl, fmt in (("w", "  localparam [64:0] E1 = 65'h1_0000_0000_0000_0000;\n", "%0d"),
                       ("s", '  localparam string E1 = "AB";\n', "%s"),
                       ("r", "  localparam real E1 = 2.5;\n", "%f")):
    cells[f"Ef{tag}_fn"] = ("module top;\n" + decl +
        "  function automatic int f();\n    typedef enum {E0, E1} e_t;\n    return E1 + 0;\n  endfunction\n"
        "  localparam int KF = f();\n"
        '  initial begin\n    #1 $display("fn f=%0d KF=%0d", f(), KF);\n'
        '    #1 $display("post ' + fmt + '", E1);\n  end\n  initial #100 $finish;\nendmodule\n')
    cells[f"Ef{tag}_tk"] = ("module top;\n" + decl +
        "  int r;\n  task automatic t();\n    typedef enum {E0, E1} e_t;\n    r = E1 + 0;\n  endtask\n"
        '  initial begin\n    #1 t();\n    $display("tk r=%0d", r);\n'
        '    #1 $display("post ' + fmt + '", E1);\n  end\n  initial #100 $finish;\nendmodule\n')
    cells[f"Ef{tag}_ff"] = ("module top;\n" + decl +
        "  int r;\n  function automatic int f(input int a);\n    typedef enum {E0, E1} e_t;\n"
        "    int t;\n    t = a;\n    for (int k = 0; k < 1; k++) t = t + E1;\n    return t;\n  endfunction\n"
        '  initial begin\n    #1 r = f(10);\n    $display("ff r=%0d", r);\n'
        '    #1 $display("post ' + fmt + '", E1);\n  end\n  initial #100 $finish;\nendmodule\n')

# ---------------- I: two imports (or an import and a local) at one key ----------------
I_LANES = {
    "val": '  localparam int K = P;\n  localparam [64:0] KW = P + 65\'d1;\n  wire [64:0] wv = P;\n'
           '  initial #1 $display("val P=%0d K=%0d KW=%0d wv=%0d b=%0d sh=%0d", P, K, KW, wv, $bits(P), P >> 60);\n',
    "sel": '  initial #1 $display("sel s0=%b s64=%b lo=%0d", P[0], P[64], P[3:0]);\n',
    "gif": "  if (P > 65'd100) begin : t initial #1 $display(\"gif big\"); end\n"
           '  else begin : e initial #1 $display("gif small"); end\n',
    "gcs": "  case (P)\n    65'h1_0000_0000_0000_0009: begin : hw initial #1 $display(\"gcs wide\"); end\n"
           '    3: begin : h3 initial #1 $display("gcs three"); end\n'
           '    default: begin : d initial #1 $display("gcs def"); end\n  endcase\n',
    "pcs": "  initial #1 case (P) 65'h1_0000_0000_0000_0009: $display(\"pcs wide\"); 3: $display(\"pcs three\"); default: $display(\"pcs def\"); endcase\n",
    "wid": '  logic [P[3:0]:0] v;\n  initial #1 $display("wid b=%0d", $bits(v));\n',
    "ovr": "  sub #(.P(P)) u ();\n",
    "cmp": "  initial #1 $display(\"cmp gt=%0d eq3=%0d\", (P > 65'd100), (P == 3));\n",
}
PA_N = "package pa; localparam P = 3; endpackage\n"
PB_W = "package pb; localparam [64:0] P = %s; endpackage\n" % W65
PA_W = "package pa; localparam [64:0] P = %s; endpackage\n" % W65
PB_N = "package pb; localparam P = 3; endpackage\n"
I_CFG = {
    "nw": (PA_N + PB_W, "  import pa::*;\n  import pb::P;\n"),     # wildcard narrow, explicit wide (q1g)
    "wn": (PA_W + PB_N, "  import pa::*;\n  import pb::P;\n"),     # wildcard wide, explicit narrow (mirror)
    "ew": (PA_N + PB_W, "  import pb::P;\n  import pa::*;\n"),     # explicit wide first (control)
    "en": (PA_W + PB_N, "  import pb::P;\n  import pa::*;\n"),     # explicit narrow first (control)
    "aa": (PA_N + PB_W, "  import pa::*;\n  import pb::*;\n"),     # two wildcards, referenced (ambiguous)
    "ab": (PA_N + PB_W, "  import pb::*;\n  import pa::*;\n"),     # two wildcards, other order
}
for tag, (pkgs, imps) in I_CFG.items():
    for ln, body in I_LANES.items():
        extra = SUB if ln == "ovr" else ""
        cells[f"I{tag}_{ln}"] = (pkgs + "module top;\n" + imps + body +
                                 "  initial #100 $finish;\nendmodule\n" + extra)
# $unit wildcard narrow + module explicit wide
cells["Icu_val"] = (PA_N + PB_W + "import pa::*;\nmodule top;\n  import pb::P;\n" + I_LANES["val"] +
                    I_LANES["gif"] + "  initial #100 $finish;\nendmodule\n")
# header import (before the parameter list) + body explicit import
cells["Ihd_val"] = (PA_N + PB_W + "module top import pa::*; #(parameter Q = 1) ();\n  import pb::P;\n" +
                    I_LANES["val"] + I_LANES["gif"] + "  initial #100 $finish;\nendmodule\n")
# package-level: pc imports both, folds Z/Y from P, module reads pc::Z
for tag, pk in (("nw", PA_N + PB_W + "package pc;\n  import pa::*;\n  import pb::P;\n"),
                ("wn", PA_W + PB_N + "package pc;\n  import pa::*;\n  import pb::P;\n")):
    cells[f"Ipk{tag}_val"] = (pk + "  localparam [64:0] Z = P;\n  localparam [64:0] Y = P + 65'd1;\n"
        "  localparam int K = P;\n  function automatic logic [64:0] fz(); return P; endfunction\nendpackage\n"
        "module top;\n  initial #1 $display(\"pk Z=%0d Y=%0d K=%0d fz=%0d\", pc::Z, pc::Y, pc::K, pc::fz());\n"
        "  initial #100 $finish;\nendmodule\n")
# a local declaration vs a wildcard import, both orders (local wins; controls)
for tag, local, pk in (("lw", "  localparam [64:0] P = %s;\n" % W65, PA_N),
                       ("ln", "  localparam P = 3;\n", PA_W)):
    cells[f"I{tag}a_val"] = (pk + "module top;\n  import pa::*;\n" + local + I_LANES["val"] + I_LANES["gif"] +
                             "  initial #100 $finish;\nendmodule\n")
    cells[f"I{tag}b_val"] = (pk + "module top;\n" + local + "  import pa::*;\n" + I_LANES["val"] + I_LANES["gif"] +
                             "  initial #100 $finish;\nendmodule\n")
# overrides: a narrow default overridden wide, a wide default overridden narrow (controls)
cells["Ov_nw"] = ("module c #(parameter [64:0] P = 3) ();\n  initial #1 $display(\"%m P=%0d b=%0d\", P, $bits(P));\n"
                  "  if (P > 65'd100) begin : t initial #1 $display(\"%m big\"); end\nendmodule\n"
                  "module top;\n  c #(.P(W65X)) u ();\n  initial #100 $finish;\nendmodule\n").replace("W65X", W65)
cells["Ov_wn"] = ("module c #(parameter [64:0] P = %s) ();\n" % W65 +
                  "  initial #1 $display(\"%m P=%0d b=%0d\", P, $bits(P));\n"
                  "  if (P > 65'd100) begin : t initial #1 $display(\"%m big\"); end\nendmodule\n"
                  "module top;\n  c #(.P(3)) u ();\n  initial #100 $finish;\nendmodule\n")


if MODE == "ctl":
    main = dict(cells)
    cells.clear()
    # plain twins (one binding per key by construction)
    g_cells("x", "", "", "%0d", post=False)
    for ln, body in E_LANES.items():
        extra = SUB if ln == "ovr" else ""
        cells[f"Ex_{ln}"] = ("module top;\n  typedef enum {E0, E1} e_t;\n" + body +
                             "  initial #100 $finish;\nendmodule\n" + extra)
    for tag, pkgs in (("w", PB_W), ("n", PB_N)):
        for ln, body in I_LANES.items():
            extra = SUB if ln == "ovr" else ""
            cells[f"Ip{tag}_{ln}"] = (pkgs + "module top;\n  import pb::P;\n" + body +
                                      "  initial #100 $finish;\nendmodule\n" + extra)
    for tag, pk in (("w", PB_W + "package pc;\n  import pb::P;\n"), ("n", PB_N + "package pc;\n  import pb::P;\n")):
        cells[f"Ippk{tag}_val"] = (pk + "  localparam [64:0] Z = P;\n  localparam [64:0] Y = P + 65'd1;\n"
            "  localparam int K = P;\n  function automatic logic [64:0] fz(); return P; endfunction\nendpackage\n"
            "module top;\n  initial #1 $display(\"pk Z=%0d Y=%0d K=%0d fz=%0d\", pc::Z, pc::Y, pc::K, pc::fz());\n"
            "  initial #100 $finish;\nendmodule\n")
    # two wildcard wides of one name (ROADMAP "bind the first")
    PB_W2 = "package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0007; endpackage\n"
    for ln in ("val", "gcs"):
        cells[f"Iww_{ln}"] = (PA_W + PB_W2 + "module top;\n  import pa::*;\n  import pb::*;\n" + I_LANES[ln] +
                              "  initial #100 $finish;\nendmodule\n")
    # an inner wide over an outer narrow (different keys): the i64 walk over params alone
    for ln in ("val", "gif", "gcs", "cmp"):
        body = I_LANES[ln].replace("\n  ", "\n    ")
        cells[f"Nw_{ln}"] = ("module top;\n  localparam P = 3;\n  if (1) begin : b\n    localparam [64:0] P = %s;\n" % W65 +
                             "  " + body + "  end\n  initial #100 $finish;\nendmodule\n")
    # genvar declared at module scope (`genvar i;`) under a wildcard wide import: the local declaration shadows
    cells["Gmd_val"] = ("package pk; localparam [64:0] i = %s; endpackage\n" % W65 +
        "module top;\n  import pk::*;\n  genvar i;\n  for (i = 0; i < 2; i++) begin : g\n    localparam int K = i;\n"
        '    initial #1 $display("val %m i=%0d K=%0d b=%0d", i, K, $bits(i));\n  end\n  initial #100 $finish;\nendmodule\n')

for n, src in cells.items():
    open(os.path.join(OUT, n + ".sv"), "w").write(src)
print(len(cells))
