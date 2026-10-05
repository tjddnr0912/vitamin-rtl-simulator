#!/usr/bin/env python3
# gen1.py <outdir>: row-X census cells. One expression x one consumer per file.
import os, sys
out = sys.argv[1]; os.makedirs(out, exist_ok=True)
cells = {}

PKG = "package p;\n  localparam int PN = 2;\n  localparam logic [7:0] PA [0:1] = '{8'hFC, 8'h02};\n  function automatic logic [7:0] pf(); return 8'd2; endfunction\nendpackage\n"

def consumers(E, K, gt, wK=None):
    """consumer name -> (decl lines, display stmt)"""
    return {
      'lp': ([f"localparam L = (({E}) == {K});"], 'initial #1 $display("L=%0d", L);'),
      'lg': ([f"localparam L = (({E}) > {gt});"], 'initial #1 $display("L=%0d", L);'),
      'lv': ([f"localparam L = {E};"], 'initial #1 $display("L=%0d B=%0d", L, $bits(L));'),
      'lt': ([f"localparam logic [{(wK or 8)*2-1}:0] L = {E};"], 'initial #1 $display("L=%0d", L);'),
      'gi': ([f"if (({E}) == {K}) begin : gt initial #1 $display(\"GI=then\"); end else begin : ge initial #1 $display(\"GI=else\"); end"], None),
      'gc': ([f"case ({E}) {K}: begin : gk initial #1 $display(\"GC=item\"); end default: begin : gd initial #1 $display(\"GC=def\"); end endcase"], None),
      'rb': ([f"logic [(({E}) == {K}) + 3:0] v;"], 'initial #1 $display("vb=%0d", $bits(v));'),
      'rt': ([], f'initial #1 $display("RT=%0d", (({E}) == {K}));'),
      'rv': ([], f'initial #1 $display("RV=%0d", {E});'),
    }

def emit(name, pre, decls, body_lines, hdr="", pkg=False, wrap=None):
    s = "`timescale 1ns/1ns\n" + (PKG if pkg else "") + f"module t{hdr};\n"
    for d in decls: s += "  " + d + "\n"
    inner = ""
    for l in body_lines: inner += "  " + l + "\n"
    if wrap:
        inner = wrap[0] + "\n" + inner + wrap[1] + "\n"
    s += inner + "  initial #5 $finish;\nendmodule\n"
    cells[name] = s

def add(name, decls, E, K, gt, wK=8, hdr="", pkg=False, cons=None, wrap=None):
    for c, (cd, disp) in consumers(E, K, gt, wK).items():
        if cons and c not in cons: continue
        body = cd + ([disp] if disp else [])
        emit(f"{name}_{c}", None, decls, body, hdr, pkg, wrap)

X8 = "localparam logic signed [7:0] X = -4;"
XU = "localparam logic [7:0] X = 8'hFC;"
N  = "localparam int N = 2;"
ARR = "localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};"
C1 = "localparam bit C = 1;"
# leaf table: name -> (extra decls, Z expr, form) ; form add => Z value 0, or => Z value 2
leaves = {
 'Rlp':  ([N], "{N{1'b0}}", 'add', False, ""),
 'Rhp':  ([], "{NH{1'b0}}", 'add', False, " #(parameter int NH = 2)"),
 'Run':  (["localparam NU = 2;"], "{NU{1'b0}}", 'add', False, ""),
 'Rfn':  (["function automatic int f2(input int a); return a; endfunction"], "{f2(2){1'b0}}", 'add', False, ""),
 'Rex':  ([N], "{(N-0){1'b0}}", 'add', False, ""),
 'Rnest':([N], "{2{{N{1'b0}}}}", 'add', False, ""),
 'Rcat': ([N], "{{N{1'b0}}, 1'b0}", 'add', False, ""),
 'Rpkg': ([], "{p::PN{1'b0}}", 'add', True, ""),
 'Rsys': ([], "{$clog2(4){1'b0}}", 'add', False, ""),
 'Rbits':([], "{$bits(2'b00){1'b0}}", 'add', False, ""),
 'Rlit': ([], "{2{1'b0}}", 'add', False, ""),
 'Eel':  ([ARR], "A[1]", 'or', False, ""),
 'Epel': ([], "p::PA[1]", 'or', True, ""),
 'Eesl': ([ARR], "A[1][7:0]", 'or', False, ""),
 'Eepa': ([ARR], "A[1][3:0]", 'or', False, ""),
 'Ocast':(["typedef logic [7:0] u8_t;"], "u8_t'(8'd2)", 'or', False, ""),
 'Ofret':(["typedef logic [7:0] u8_t;", "function automatic u8_t fz(); return 8'd2; endfunction"], "fz()", 'or', False, ""),
 'Opfn': ([], "p::pf()", 'or', True, ""),
 'Oslen':(["localparam string S = \"ab\";"], "S.len()", 'slen', False, ""),
 'Osel': (["localparam W = 5 + 3;"], "W[1:0]", 'or2', False, ""),
}
for ln, (ex, Z, form, pkg, hdr) in leaves.items():
    for nb, xd in (('s', X8), ('u', XU)):
        decls = [xd, C1] + ex
        if form == 'add':
            forms = {'a': (f"X + {Z}", "8'hFC"), 't': (f"C ? X : {Z}", "8'hFC")}
        elif form == 'or':
            forms = {'o': (f"X | {Z}", "8'hFE"), 't': (f"C ? X : {Z}", "8'hFC")}
        elif form == 'or2':
            forms = {'o': (f"X | {Z}", "8'hFC"), 't': (f"C ? X : {Z}", "8'hFC")}
        else:  # slen: (X | 8'd0) + S.len() -> 32-bit unsigned region
            forms = {'o': (f"(X | 8'd0) + {Z}", "32'd254")}
        for fn, (E, K) in forms.items():
            if nb == 'u' and fn == 't':
                continue
            wK = 32 if form == 'slen' else 8
            add(f"{ln}_{fn}_{nb}", decls, E, K, "8'd100", wK, hdr, pkg)

# width axis (R_lp leaf), signed neighbour of various widths, and wide replications
wax = {
 'w32': ("localparam int X = -4;", "X + {N{1'b0}}", "32'hFFFF_FFFC", "32'd100", 32),
 'w33': ("localparam logic signed [32:0] X = -4;", "X + {N{1'b0}}", "33'h1_FFFF_FFFC", "33'd100", 33),
 'w64': ("localparam longint X = -4;", "X + {N{1'b0}}", "64'hFFFF_FFFF_FFFF_FFFC", "64'd100", 64),
 'w65': ("localparam logic signed [64:0] X = -4;", "X + {N{1'b0}}", "65'h1_FFFF_FFFF_FFFF_FFFC", "65'd100", 65),
 'z32': (X8, "X + {N{16'h0}}", "32'hFC", "32'd300", 32),
 'z34': (X8, "X + {N{17'h0}}", "34'hFC", "34'd300", 34),
 'z64': (X8, "X + {N{32'h0}}", "64'hFC", "64'd300", 64),
 'z80': (X8, "X + {N{40'h0}}", "80'hFC", "80'd300", 80),
}
for wn, (xd, E, K, gt, w) in wax.items():
    add(f"W{wn}", [xd, N], E, K, gt, w)
# operator axis (R_lp and E_el), signed X
ops = {
 'ne':  ("({E}) != {K}",),
 'lt':  ("({E}) < 8'd100",),
 'ceq': ("({E}) === {K}",),
 'weq': ("({E}) ==? 8'b1111_1?00",),
 'ins': ("({E}) inside {{{K}, 8'h00}}",),
}
for ln, ex, E, K in (('Rlp', [N], "X + {N{1'b0}}", "8'hFC"), ('Eel', [ARR], "X | A[1]", "8'hFE")):
    for on, (tmpl,) in ops.items():
        cmp_ = tmpl.format(E=E, K=K)
        for c, (cd, disp) in {
            'lp': ([f"localparam L = ({cmp_});"], 'initial #1 $display("L=%0d", L);'),
            'gi': ([f"if ({cmp_}) begin : gt initial #1 $display(\"GI=then\"); end else begin : ge initial #1 $display(\"GI=else\"); end"], None),
            'rb': ([f"logic [({cmp_}) + 3:0] v;"], 'initial #1 $display("vb=%0d", $bits(v));'),
            'rt': ([], f'initial #1 $display("RT=%0d", ({cmp_}));'),
        }.items():
            emit(f"P{on}_{ln}_{c}", None, [X8] + ex, cd + ([disp] if disp else []))
# arithmetic value axis: value consumer lv/rv only
for an, E in (('div', "X / {N{1'b1}}"), ('mod', "X % {N{1'b1}}"), ('shr', "(X + {N{1'b0}}) >> 1"),
              ('sub', "X - {N{1'b0}}"), ('mul', "X * {N{1'b1}}"), ('ediv', "X / A[1]"), ('eshr', "(X | A[1]) >> 1")):
    ex = [N, ARR] if an.startswith('e') else [N]
    for c in ('lv', 'lt', 'rv'):
        cd, disp = consumers(E, "8'h0", "8'd0")[c]
        emit(f"A{an}_{c}", None, [X8] + ex, cd + [disp])
# signed array element as the signed operand
ASD = "localparam logic signed [7:0] AS [0:1] = '{-8'sd4, 8'sd2};"
add("Sel_u", [ASD], "AS[0] + 8'd0", "8'hFC", "8'd100")         # unsigned region: expect 252
for c in ('lp', 'gi', 'rb', 'rt', 'lv', 'rv'):
    E = "AS[0] + 8'sd0"
    d = consumers(E, "-8'sd4", "8'sd0")[c]
    emit(f"Sel_s_{c}", None, [ASD], d[0] + ([d[1]] if d[1] else []))
    d = consumers("AS[0]", "-8'sd4", "8'sd0")[c]
    emit(f"Sel_b_{c}", None, [ASD], d[0] + ([d[1]] if d[1] else []))
    d = consumers("(AS[0] < 0)", "1'b1", "1'b0")[c]
    emit(f"Sel_n_{c}", None, [ASD], d[0] + ([d[1]] if d[1] else []))
# wrap (unsigned only, no sign): overflow at the region width
add("Kwr", [N], "8'hFF + 8'd1 + {N{1'b0}}", "8'h00", "8'd100")
add("Kwe", [ARR], "8'hFF + A[1]", "8'h01", "8'd100")
# fill beside a parameter-count replication
add("Fil", [N], "'1 ^ {N{1'b0}}", "2'b11", "2'd2", 2)
# binders (R_lp, lp/gi/rb)
for c in ('lp', 'gi', 'rb', 'lv'):
    cd, disp = consumers("X + {N{1'b0}}", "8'hFC", "8'd100")[c]
    emit(f"Bgen_{c}", None, [X8, N], cd + ([disp] if disp else []), wrap=("  if (1) begin : gb", "  end"))
    emit(f"Bgv_{c}", None, [X8], [l.replace("{N{", "{g{") for l in cd] + ([disp] if disp else []),
         wrap=("  for (genvar g = 2; g < 3; g = g + 1) begin : gl", "  end"))
# package binder
cells["Bpkg_lp"] = ("`timescale 1ns/1ns\npackage q;\n  localparam logic signed [7:0] X = -4;\n  localparam int N = 2;\n"
  "  localparam L = ((X + {N{1'b0}}) == 8'hFC);\n  localparam V = X + {N{1'b0}};\nendpackage\nmodule t;\n"
  "  initial #1 $display(\"L=%0d V=%0d B=%0d\", q::L, q::V, $bits(q::V));\n  initial #5 $finish;\nendmodule\n")
# instance / override binder
for bn, inst in (('Binst', "m u();"), ('Bovr', "m #(.X(-4), .N(2)) u();")):
    dflt = "-4" if bn == 'Binst' else "0"; dn = "2" if bn == 'Binst' else "1"
    cells[f"{bn}_lp"] = ("`timescale 1ns/1ns\n"
      f"module m #(parameter logic signed [7:0] X = {dflt}, parameter int N = {dn});\n"
      "  localparam L = ((X + {N{1'b0}}) == 8'hFC);\n  localparam V = X + {N{1'b0}};\n"
      "  logic [((X + {N{1'b0}}) == 8'hFC) + 3:0] v;\n"
      "  initial #1 $display(\"L=%0d V=%0d B=%0d vb=%0d\", L, V, $bits(V), $bits(v));\nendmodule\n"
      f"module t;\n  {inst}\n  initial #5 $finish;\nendmodule\n")
# constant-function bodies: module params inside a function; and a local of unknown width (envw 0)
cells["Bfn_lp"] = ("`timescale 1ns/1ns\nmodule t;\n  localparam logic signed [7:0] X = -4;\n  localparam int N = 2;\n"
  "  function automatic int g(input int a); return ((X + {N{1'b0}}) == 8'hFC); endfunction\n"
  "  localparam L = g(0);\n  initial #1 $display(\"L=%0d\", L);\n  initial #5 $finish;\nendmodule\n")
cells["Bfl_lp"] = ("`timescale 1ns/1ns\nmodule t;\n  function automatic int h(); return 2; endfunction\n"
  "  function automatic int g(input int a); logic signed [7:0] x; logic [h()-1:0] z; x = -4; z = 0;\n"
  "    return ((x + z) == 8'hFC); endfunction\n"
  "  localparam L = g(0);\n  initial #1 $display(\"L=%0d\", L);\n  initial #5 $finish;\nendmodule\n")
cells["Bfr_lp"] = ("`timescale 1ns/1ns\nmodule t;\n  localparam int N = 2;\n"
  "  function automatic int g(input int a); logic signed [7:0] x; x = -4;\n"
  "    return ((x + {N{1'b0}}) == 8'hFC); endfunction\n"
  "  localparam L = g(0);\n  initial #1 $display(\"L=%0d\", L);\n  initial #5 $finish;\nendmodule\n")
cells["Bfa_lp"] = ("`timescale 1ns/1ns\nmodule t;\n  localparam int N = 2;\n"
  "  function automatic logic [15:0] g(input int a); logic signed [7:0] x; logic [15:0] r; x = -4;\n"
  "    r = x + {N{1'b0}}; return r; endfunction\n"
  "  localparam L = g(0);\n  initial #1 $display(\"L=%0d\", L);\n  initial #5 $finish;\nendmodule\n")
for n, s in cells.items():
    open(os.path.join(out, n + ".sv"), "w").write(s)
print(len(cells))
