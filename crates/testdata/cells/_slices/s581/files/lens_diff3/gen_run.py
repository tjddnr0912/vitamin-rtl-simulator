#!/usr/bin/env python3
"""lens:differential round 3 — generate variants of 12 hand-written designs, run PRE / POST3b / iverilog / sv2v->iverilog."""
import os, re, subprocess, sys
from concurrent.futures import ThreadPoolExecutor

SCR = "/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s581"
L = SCR + "/lens_diff3"
OUT = L + "/v"
SV2V = SCR + "/../s580/sv2v/sv2v-macOS/sv2v"
BINS = [("pre", SCR + "/pre/vita"), ("post3b", SCR + "/post3b/vita")]
os.makedirs(OUT, exist_ok=True)

DECL = {
    "P40": "localparam logic [39:0] P40 = 40'h10_0000_000C;",
    "P40F": "localparam logic [39:0] P40F = 40'h0C;",
    "P68H": "localparam logic [67:0] P68H = 68'h1_0000_0000_0000_000C;",
    "P68F": "localparam logic [67:0] P68F = 68'hC;",
    "P64": "localparam logic [63:0] P64 = 64'hC;",
    "P64H": "localparam logic [63:0] P64H = 64'h8000_0000_0000_000C;",
    "P65": "localparam logic [64:0] P65 = 65'hC;",
    "P65H": "localparam logic [64:0] P65H = 65'h0_8000_0000_0000_000C;",
    "S64N": "localparam logic signed [63:0] S64N = -64'sd4;",
    "S64P": "localparam logic signed [63:0] S64P = 64'sd12;",
    "S65N": "localparam logic signed [64:0] S65N = -65'sd4;",
    "S65P": "localparam logic signed [64:0] S65P = 65'sd12;",
    "N2": "localparam int N2 = 2;",
    "E0": "typedef enum logic [39:0] {E0 = 40'h10_0000_000C, E1 = 40'h0C} et40;",
    "E1": "typedef enum logic [39:0] {E0 = 40'h10_0000_000C, E1 = 40'h0C} et40;",
    "ES": "typedef enum logic signed [3:0] {ES = -4'sd4, ET = 4'sd3} est4;",
    "t40": "typedef logic [39:0] t40;",
    "PT": "typedef logic [39:0] t40;\nlocalparam t40 PT = 40'h10_0000_000C;",
    "PTF": "typedef logic [39:0] t40;\nlocalparam t40 PTF = 40'h0C;",
    "SS": "typedef struct packed {logic [39:0] a; logic [3:0] b;} st;\nlocalparam st SS = '{a: 40'h10_0000_000C, b: 4'hC};",
    "PA": "localparam logic [39:0] PA [2] = '{40'h10_0000_000C, 40'h0};",
    "PP": "localparam logic [1:0][39:0] PP = {40'h10_0000_000C, 40'h1};",
    "f40": "function automatic logic [39:0] f40(input int a); return 40'h10_0000_0000 + a; endfunction",
    "ft40": "typedef logic [39:0] t40;\nfunction automatic t40 ft40(input int a); return 40'h10_0000_0000 + a; endfunction",
    "finc": "function automatic int finc(input int a); return a + 5; endfunction",
    "fs64": "function automatic logic signed [63:0] fs64(input int a); return -a; endfunction",
    "f65": "function automatic logic [64:0] f65(input int a); return a; endfunction",
    "PX": "localparam logic [3:0] PX = 4'b1x00;",
}


def decls_for(text, extra=""):
    names = set(re.findall(r"(?<!['\w])[A-Za-z_]\w*", text))
    lines, seen = [], set()
    for k, v in DECL.items():
        if k in names:
            for ln in v.split("\n"):
                if ln not in seen:
                    seen.add(ln)
                    lines.append(ln)
    out = ([extra] if extra else []) + lines
    return "\n".join("  " + x for x in out)


LP = 'module top;\n@D@\n  localparam R = (@E@);\n  initial $display("R=%0d", R);\n  initial #100 $finish;\nendmodule\n'
GI = ('module top;\n@D@\n  if (@E@) begin : g\n    initial $display("GI=then");\n  end else begin : g\n'
      '    initial $display("GI=else");\n  end\n  initial #100 $finish;\nendmodule\n')
rows = []


def emit(vid, design, text, exp):
    p = f"{OUT}/{vid}.sv"
    with open(p, "w") as f:
        f.write(text)
    rows.append((vid, design, p, exp))


def lhs(design, vid, expr, e, pre="", hdr=None, extra="", cons=("lp", "gi")):
    for c in cons:
        t = (LP if c == "lp" else GI).replace("@D@", decls_for(expr, extra)).replace("@E@", expr)
        if hdr:
            t = t.replace("module top;", hdr, 1)
        exp = f"R={e}" if c == "lp" else {"1": "GI=then", "0": "GI=else", "x": "GI=else"}.get(e, "?")
        emit(f"{vid}_{c}", design, (pre + "\n" + t) if pre else t, exp)


def body(design, vid, b, exp, pre=""):
    t = "module top;\n" + decls_for(b) + "\n  " + b + "\n  initial #100 $finish;\nendmodule\n"
    emit(vid, design, (pre + "\n" + t) if pre else t, exp)


# ---- D1 system-call left operands
for vid, ex, e in [
    ("A01", "$unsigned(P40) ==? 4'b1?00", "0"), ("A01c", "$unsigned(P40F) ==? 4'b1?00", "1"),
    ("A02", "$unsigned(4'd12) ==? 4'sb1?00", "1"), ("A03", "$signed(P40) ==? 4'b1?00", "0"),
    ("A04", "$signed(4'b1100) ==? 8'b1111_1?00", "0"), ("A05", "$signed(4'b1100) ==? 8'sb1111_1?00", "1"),
    ("A06", "$bits(P40) ==? 8'b0010_1?00", "1"), ("A07", "$clog2(P40) ==? 6'b10_0?01", "1"),
    ("A08", "$unsigned(-4'sd4) ==? 4'sb1?00", "1"), ("A09", "$signed(4'd12) ==? 4'b1?00", "1"),
    ("A10", "$signed(4'd12) ==? 4'sb1?00", "1"), ("A11", "$unsigned(P68H) ==? 4'b1?00", "0"),
    ("A12", "$unsigned(P68F) ==? 4'b1?00", "1"), ("A13", "$signed(4'd12) !=? 4'b1?00", "0"),
    ("A14", "$countones(P40) ==? 4'b0?11", "1"), ("A15", "$bits(P40) ==? 'b10_1?00", "1"),
    ("A16", "$clog2(40) ==? 'sb1?0", "1"),
]:
    lhs("D01", vid, ex, e)

# ---- D2 identifiers / enum / typed / function left operands
for vid, ex, e in [
    ("B01", "E0 ==? 4'b1?00", "0"), ("B01c", "E1 ==? 4'b1?00", "1"),
    ("B02", "ES ==? 8'sb1111_1?00", "1"), ("B02b", "ES ==? 8'b0000_1?00", "1"), ("B02c", "ES ==? 8'b1111_1?00", "0"),
    ("B09", "PT ==? 4'b1?00", "0"), ("B09c", "PTF ==? 4'b1?00", "1"),
    ("B11", "f40(12) ==? 4'b1?00", "0"), ("B11t", "ft40(12) ==? 4'b1?00", "0"), ("B11c", "finc(7) ==? 4'b1?00", "1"),
    ("B11s", "fs64(4) ==? 4'sb1?00", "1"), ("B11u", "f65(12) ==? 4'b1?00", "1"),
]:
    lhs("D02", vid, ex, e)
lhs("D02", "B04", "P ==? 4'b1?00", "0", hdr="module top #(parameter type T = logic [39:0], parameter T P = 40'h10_0000_000C);")
lhs("D02", "B04c", "P ==? 4'b1?00", "1", hdr="module top #(parameter type T = logic [39:0], parameter T P = 40'hC);")
lhs("D02", "B04s", "P ==? 4'sb1?00", "1", hdr="module top #(parameter type T = logic signed [63:0], parameter T P = -64'sd4);")
body("D02", "B03", 'for (genvar i = 8; i < 9; i++) begin : g\n    localparam R = (i ==? 4\'sb1?00);\n    initial $display("R=%0d", R);\n  end', "R=?")
body("D02", "B03b", 'for (genvar i = 8; i < 9; i++) begin : g\n    localparam R = (i ==? 4\'b1?00);\n    initial $display("R=%0d", R);\n  end', "R=1")
body("D02", "B10", 'if (1) begin : gb\n    localparam logic [39:0] GP = 40\'h10_0000_000C;\n    localparam R = (GP ==? 4\'b1?00);\n    initial $display("R=%0d", R);\n  end', "R=0")
body("D02", "B10s", 'if (1) begin : gb\n    localparam logic signed [63:0] GS = -64\'sd4;\n    localparam R = (GS ==? 4\'sb1?00);\n    initial $display("R=%0d", R);\n  end', "R=1")

# ---- D3 packages: import, scoped, package-in-package
PKA = ("package pa;\n  localparam logic [39:0] PW = 40'h10_0000_000C;\n  localparam logic [39:0] PF = 40'h0C;\n"
       "  localparam logic signed [63:0] PS = -64'sd4;\nendpackage\n")
for vid, ex, e in [("B05", "PW ==? 4'b1?00", "0"), ("B05c", "PF ==? 4'b1?00", "1"), ("B05s", "PS ==? 4'sb1?00", "1"),
                   ("B06", "pa::PW ==? 4'b1?00", "0"), ("B06s", "pa::PS ==? 4'sb1?00", "1"),
                   ("B06i", "pa::PW inside {4'b1?00}", "0"), ("B06u", "pa::PF ==? 'b1?00", "1")]:
    lhs("D03", vid, ex, e, pre=PKA, extra="import pa::*;")
for vid, imp, ex, e in [
    ("B07", "import pa::*;", "PW ==? 4'b1?00", "0"), ("B07s", "import pa::*;", "pa::PW ==? 4'b1?00", "0"),
    ("B07f", "import pa::*;", "PF ==? 4'b1?00", "1"), ("B07g", "import pa::*;", "PS ==? 4'sb1?00", "1"),
    ("B07c", "import pa::*;", "PW == 40'h10_0000_000C", "1"), ("B07i", "import pa::*;", "PW inside {4'b1?00}", "0"),
    ("B07q", "import pa::*;", "(4'd15 + 4'd1) ==? 5'b1?000", "1"), ("B07e", "import pa::PW;", "PW ==? 4'b1?00", "0"),
    ("B07u", "import pa::*;", "PS ==? 'sbx100", "1"),
]:
    t = PKA + f"package pb;\n  {imp}\n  localparam RB = ({ex});\nendpackage\n" + \
        'module top;\n  localparam R = pb::RB;\n  initial $display("R=%0d", R);\n  initial #100 $finish;\nendmodule\n'
    emit(vid, "D03", t, f"R={e}")

# ---- D4 struct / array / select / cast left operands
for vid, ex, e in [
    ("B08", "SS.a ==? 4'b1?00", "0"), ("B08b", "SS.b ==? 4'b1?00", "1"),
    ("B12", "PA[0] ==? 4'b1?00", "0"), ("B12c", "PA[1] ==? 4'b0?00", "1"),
    ("B13", "PP[1] ==? 4'b1?00", "0"), ("B13c", "PP[0] ==? 4'b0?01", "1"),
    ("B14a", "P40[39:0] ==? 4'b1?00", "0"), ("B14b", "P40[3:0] ==? 4'b1?00", "1"),
    ("B14c", "P40[36 -: 37] ==? 4'b1?00", "0"), ("B14d", "P40[0 +: 36] ==? 4'b1?00", "1"),
    ("B14e", "P40[36] ==? 2'b?1", "1"),
    ("B15a", "t40'(P40) ==? 4'b1?00", "0"), ("B15b", "40'(P40) ==? 4'b1?00", "0"),
    ("B15c", "signed'(P40) ==? 4'b1?00", "0"), ("B15d", "36'(P40) ==? 4'b1?00", "1"),
    ("B15e", "longint'(P40) ==? 4'sb1?00", "0"), ("B15f", "int'(P40) ==? 4'b1?00", "1"),
]:
    lhs("D04", vid, ex, e)

# ---- D5 replication / string / hierarchical left operands
for vid, ex, e in [
    ("C01", "{N2{4'b1100}} ==? 8'b1?00_1100", "1"), ("C01b", "{N2{4'b1100}} ==? 4'b1?00", "0"),
    ("C02", "{2{4'b1100}} ==? 4'b1?00", "0"), ("C03", "{N2{4'b1100}} ==? 'b1?00", "0"),
    ("C04", "{N2{P40}} ==? 4'b1?00", "0"),
    ("D01", "\"A\" ==? 8'b0100_0?01", "1"), ("D01b", "\"AB\" ==? 16'h41_4?", "1"),
]:
    lhs("D05", vid, ex, e)
body("D05", "E01", 'if (1) begin : gb\n    localparam logic [39:0] GP = 40\'hC;\n  end\n  localparam R = (gb.GP ==? 4\'b1?00);\n  initial $display("R=%0d", R);', "R=1")

# ---- D6 the 64/65 boundary
for vid, ex, e in [
    ("H01", "P64 ==? 4'b1?00", "1"), ("H02", "P65 ==? 4'b1?00", "1"), ("H03", "P64H ==? 4'b1?00", "0"),
    ("H04", "P65H ==? 4'b1?00", "0"), ("H05", "S64N ==? 4'sb1?00", "1"), ("H06", "S64N ==? 4'b1?00", "0"),
    ("H07", "S65N ==? 4'sb1?00", "1"), ("H08", "S65N ==? 4'b1?00", "0"), ("H09", "S64P ==? 4'sb1?00", "0"),
    ("H10", "S65P ==? 4'sb1?00", "0"), ("H11", "P64 ==? 'bx100", "1"), ("H12", "P65 ==? 'bx100", "1"),
    ("H13", "P64H ==? 'bx100", "1"), ("H14", "P64H ==? 'b1?00", "0"), ("H15", "P65 ==? 'b1?00", "1"),
    ("H16", "4'd12 ==? 64'b1?00", "1"), ("H17", "4'd12 ==? 65'b1?00", "1"), ("H18", "S64N !=? 4'sb1?00", "0"),
    ("H19", "S65N !=? 4'sb1?00", "0"), ("H20", "(64'hFFFF_FFFF_FFFF_FFFF + 64'd13) ==? 4'b1?00", "1"),
    ("H21", "{1'b0, P64} ==? 4'b1?00", "1"), ("H22", "{1'b0, P64H} ==? 4'b1?00", "0"),
    ("H23", "S64N ==? 'sbx100", "1"), ("H24", "S64N ==? 'sb1?00", "0"),
    ("H25", "S64N ==? 64'shFFFF_FFFF_FFFF_FFF?", "1"), ("H26", "P64H ==? 64'h8000_0000_0000_000?", "1"),
    ("H27", "P65H ==? 65'h0_8000_0000_0000_000?", "1"), ("H29", "(P64 << 1) ==? 5'b1?000", "1"),
    ("H30", "(P65 >> 1) ==? 4'b0?10", "1"), ("H33", "S64N[63:0] ==? 4'sb1?00", "0"),
    ("H35", "64'sd12 ==? 4'sb1?00", "0"), ("H36", "65'sd12 ==? 4'sb1?00", "0"),
    ("H37", "S64P ==? 'sb1?00", "1"), ("H38", "S65P ==? 4'b1?00", "1"), ("H39", "P65H !=? 4'b1?00", "1"),
]:
    lhs("D06", vid, ex, e)

# ---- D7 inside with several elements, one crossing 64 bits
for vid, ex, e in [
    ("H40", "4'd12 inside {4'b0?11, 65'b1?00}", "1"), ("H41", "4'd3 inside {65'b1?00, 4'b0?11}", "1"),
    ("H42", "4'd5 inside {65'b1?00, 4'b0?11}", "0"), ("H43", "P64 inside {4'b1?00, 65'b0?11}", "1"),
    ("H44", "P65 inside {4'b1?00, 4'b0?11}", "1"), ("H45", "4'd12 inside {4'b1?00, 4'd5}", "1"),
    ("H46", "4'd12 inside {4'd5, 65'b1?00}", "1"), ("H47", "4'd5 inside {4'b1?00, 65'd5}", "1"),
    ("H48", "4'd12 inside {64'b1?00, 64'b0?11}", "1"), ("H49", "P64H inside {64'h8000_0000_0000_000?, 4'd3}", "1"),
    ("H50", "4'd12 inside {4'b0?11, 'bx100}", "1"), ("H51", "4'd12 inside {[4'd1:4'd3], 4'b1?00}", "1"),
    ("H52", "S64N inside {4'sb1?00}", "1"), ("H53", "P64H inside {4'b1?00}", "0"),
]:
    lhs("D07", vid, ex, e)

# ---- consumer drivers
EDRV = [("e1", "(4'd15 + 4'd1) ==? 5'b1?000"), ("e4", "4'd12 inside {4'b1?00}"),
        ("e6", "S64N ==? 4'sb1?00"), ("e7", "P65 ==? 4'b1?00"), ("ctl", "4'd12 == 4'd12")]


def cons(design, cid, tmpl, exp, pre=""):
    for k, ex in EDRV:
        b = tmpl.replace("@E@", ex)
        body(design, f"{cid}_{k}", b, exp, pre)


# ---- D8 generate-for init / bound / step
cons("D08", "GFB", 'for (genvar i = 0; i < 1 + (@E@); i++) begin : g\n    initial $display("G=%0d", i);\n  end', "G=0;G=1")
cons("D08", "GFS", 'for (genvar i = 0; i < 4; i = i + 1 + (@E@)) begin : g\n    initial $display("G=%0d", i);\n  end', "G=0;G=2")
cons("D08", "GFI", 'for (genvar i = (@E@); i < 2; i++) begin : g\n    initial $display("G=%0d", i);\n  end', "G=1")
# ---- D9 replication count, part-select bounds, indexed part-select width / base
cons("D09", "RC", 'localparam logic [7:0] R = {(@E@) + 1 {4\'b1010}};\n  initial $display("R=%b", R);', "R=10101010")
cons("D09", "PS", 'localparam logic [7:0] V = 8\'b1011_0110;\n  localparam logic [3:0] R = V[(@E@) + 2 : 0];\n  initial $display("R=%b", R);', "R=0110")
cons("D09", "IP", 'localparam logic [7:0] V = 8\'b1011_0110;\n  localparam logic [1:0] R = V[0 +: (@E@) + 1];\n  initial $display("R=%b", R);', "R=10")
cons("D09", "IB", 'localparam logic [7:0] V = 8\'b1011_0110;\n  localparam logic [3:0] R = V[(@E@) * 4 +: 4];\n  initial $display("R=%b", R);', "R=1011")
# ---- D10 const-function argument, ternary, $bits/$clog2/$signed, enum init
cons("D10", "FA", 'localparam int R = finc(@E@);\n  initial $display("R=%0d", R);', "R=6")
cons("D10", "TC", 'localparam int R = (@E@) ? 10 : 20;\n  initial $display("R=%0d", R);', "R=10")
cons("D10", "SB", 'localparam int R = $bits(@E@);\n  initial $display("R=%0d", R);', "R=1")
cons("D10", "SC", 'localparam int R = $clog2((@E@) + 3);\n  initial $display("R=%0d", R);', "R=2")
cons("D10", "SS", 'localparam int R = $signed(@E@);\n  initial $display("R=%0d", R);', "R=-1")
cons("D10", "EN", 'typedef enum logic [3:0] {A = (@E@) ? 4\'d5 : 4\'d6, B} etc;\n  localparam int R = A;\n  initial $display("R=%0d", R);', "R=5")
body("D10", "FR1", 'function automatic int fwc(input int a); return (a ==? 4\'b1?00); endfunction\n  localparam int R = fwc(12);\n  initial $display("R=%0d", R);', "R=1")
body("D10", "FR2", 'function automatic int fwc2(); return (4\'d12 ==? 4\'b1?00); endfunction\n  localparam int R = fwc2();\n  initial $display("R=%0d", R);', "R=1")
body("D10", "FR3", 'function automatic int fwc3(input int a); return a + (4\'d12 ==? 4\'b1?00); endfunction\n  localparam int R = fwc3(4);\n  initial $display("R=%0d", R);', "R=5")

# ---- D11 defparam / ordered override / parameter type
SUB = 'module sub;\n  parameter @T@ P = 0;\n  localparam R = (P ==? @PAT@);\n  initial $display("R=%0d", R);\nendmodule\n'
for vid, T, pat, ov, e, how in [
    ("DP1", "logic [39:0]", "4'b1?00", "40'h10_0000_000C", "0", "def"), ("DP2", "logic [39:0]", "4'b1?00", "40'hC", "1", "def"),
    ("DP3", "logic signed [63:0]", "4'sb1?00", "-4", "1", "def"), ("DP4", "", "4'b1?00", "40'h10_0000_000C", "0", "def"),
    ("DP5", "", "4'b1?00", "40'h10_0000_000C", "0", "ovr"), ("DP6", "", "4'sb1?00", "-64'sd4", "1", "def"),
    ("DP7", "logic [64:0]", "4'b1?00", "65'hC", "1", "def"), ("DP8", "", "4'b1?00", "65'hC", "1", "def"),
]:
    sub = SUB.replace("@T@", T).replace("@PAT@", pat).replace("parameter  P", "parameter P")
    top = ("module top;\n  sub u();\n  defparam u.P = " + ov + ";\n  initial #100 $finish;\nendmodule\n") if how == "def" else \
          ("module top;\n  sub #(" + ov + ") u();\n  initial #100 $finish;\nendmodule\n")
    emit(vid, "D11", sub + top, f"R={e}")
PTS = 'module sub #(parameter type T = logic [3:0], parameter T PV = \'0);\n  localparam T LV = 40\'h10_0000_000C;\n  localparam R = (@E@);\n  initial $display("R=%0d", R);\nendmodule\n'
for vid, ex, inst, e in [
    ("PT1", "PV ==? 4'b1?00", "sub #(.T(logic [39:0]), .PV(40'h10_0000_000C)) u();", "0"),
    ("PT2", "$bits(T) ==? 8'b0010_1?00", "sub #(.T(logic [39:0])) u();", "1"),
    ("PT3", "LV ==? 4'b1?00", "sub #(.T(logic [39:0])) u();", "0"),
    ("PT4", "LV ==? 4'b1?00", "sub u();", "1"),
    ("PT5", "PV ==? 4'sb1?00", "sub #(.T(logic signed [63:0]), .PV(-64'sd4)) u();", "1"),
]:
    emit(vid, "D11", PTS.replace("@E@", ex) + "module top;\n  " + inst + "\n  initial #100 $finish;\nendmodule\n", f"R={e}")

# ---- D12 x left operands, !=?, nested inside
for vid, ex, e in [
    ("I01", "(4'd12 inside {4'b1?00}) inside {1'b1}", "1"), ("I02", "4'd12 inside {4'b0?11, 4'b1?00}", "1"),
    ("I02b", "4'd12 inside {4'b0?11, 4'd5}", "0"), ("I03", "4'b1x00 ==? 4'b1?00", "1"), ("I04", "4'bx100 ==? 4'b1?00", "x"),
    ("I05", "4'b1x00 !=? 4'b1?00", "0"), ("I06", "4'bx100 inside {4'b1?00, 4'bx100}", "1"), ("I07", "!(4'd12 ==? 4'b1?00)", "0"),
    ("I08", "(4'd12 ==? 4'b1?00) ==? 1'b?", "1"), ("I09", "PX ==? 4'b1?00", "1"), ("I10", "4'd12 ==? 4'bz?00", "1"),
    ("I11", "-4'sd4 ==? 8'sb1111_1?00", "1"), ("I12", "-4'sd4 ==? 8'b1111_1?00", "1"), ("I13", "~4'd3 ==? 8'b1111_1?00", "1"),
    ("I14", "(4'd12 !=? 4'b1?00) !=? 1'b1", "1"), ("I15", "4'd12 ==? 4'b1?00 && 4'd3 ==? 4'b0?11", "1"),
    ("I16", "(4'bx100 ==? 4'b1?00) || 1'b1", "1"), ("I17", "4'd12 inside {4'bx100}", "1"), ("I18", "{4'b1x00} ==? 4'b1?00", "1"),
    ("I19", "(4'd12 inside {4'b1?00}) !=? 1'b0", "1"), ("I20", "4'd12 !=? 'bx100", "0"),
]:
    lhs("D12", vid, ex, e)

TAG = re.compile(r"^(R|GI|G)=")


def norm(out):
    keep = []
    for ln in out.splitlines():
        if TAG.match(ln):
            keep.append(ln.strip())
        m = re.search(r"(?:error|fatal)\[(VITA-E\d+)\]", ln)
        if m:
            keep.append(m.group(1))
    return ";".join(sorted(keep)) or "<none>"


def run_vita(b, p):
    try:
        r = subprocess.run([b, os.path.basename(p)], cwd=os.path.dirname(p), capture_output=True, text=True, timeout=60)
    except subprocess.TimeoutExpired:
        return "TIMEOUT"
    s = norm(r.stdout + r.stderr)
    return s if r.returncode in (0, 1) else f"{s}(rc{r.returncode})"


def run_iv(p, tag):
    vvp = p.rsplit(".", 1)[0] + f".{tag}.vvp"
    r = subprocess.run(["iverilog", "-g2012", "-o", vvp, p], capture_output=True, text=True, timeout=60)
    if r.returncode != 0:
        with open(p + f".{tag}.err", "w") as f:
            f.write(r.stdout + r.stderr)
        return "IVERR"
    r = subprocess.run(["vvp", "-n", vvp], capture_output=True, text=True, timeout=60)
    return ";".join(sorted(ln.strip() for ln in r.stdout.splitlines() if TAG.match(ln))) or "<none>"


def run_sv(p):
    v = p.rsplit(".", 1)[0] + ".s2v.v"
    r = subprocess.run([SV2V, p], capture_output=True, text=True, timeout=60)
    if r.returncode != 0:
        with open(p + ".sv2v.err", "w") as f:
            f.write(r.stderr)
        return "SV2VERR"
    with open(v, "w") as f:
        f.write(r.stdout)
    return run_iv(v, "s")


def one(row):
    vid, d, p, exp = row
    res = [run_vita(b, p) for _, b in BINS]
    return (vid, d, exp, res[0], res[1], run_iv(p, "i"), run_sv(p))


with ThreadPoolExecutor(6) as ex:
    res = list(ex.map(one, rows))
with open(L + "/r3.tsv", "w") as f:
    f.write("id\tdesign\texp\tpre\tpost3b\tiverilog\tsv2v\n")
    for r in res:
        f.write("\t".join(r) + "\n")
designs = sorted({r[1] for r in res})
moved = [r for r in res if r[3] != r[4]]
print(f"designs={len(designs)} variants={len(res)} moved(PRE!=POST3b)={len(moved)}")
for d in designs:
    rr = [r for r in res if r[1] == d]
    print(f"  {d}: n={len(rr)} moved={sum(1 for r in rr if r[3] != r[4])}")
print("== MOVED rows: id | exp | PRE | POST3b | iverilog | sv2v")
for r in moved:
    print(" | ".join([r[0], r[2], r[3], r[4], r[5], r[6]]))
print("== NOT moved, POST3b differs from an agreeing oracle pair (pre-existing candidates)")
for r in res:
    if r[3] == r[4] and r[5] == r[6] and r[5] not in ("IVERR", "<none>") and r[4] != r[5]:
        print(" | ".join([r[0], r[2], r[3], r[4], r[5], r[6]]))
