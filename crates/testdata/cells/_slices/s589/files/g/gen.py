import os, itertools
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "c")
os.makedirs(OUT, exist_ok=True)
CALLER_F = "function automatic int f(input int a);\n    f = 7;\n    if (a == 1) f = 10;\n  endfunction"
KINDS = {
  # kind: (pkg decl, caller decl, X token, typedef?)
  "fn":   ("function automatic int f(input int a); return 3; endfunction", CALLER_F, "f(2)"),
  "ctl":  ("function automatic int f(input int a); return 3; endfunction", "", "f(2)"),
  "par":  ("localparam int W = 3;", "localparam int W = 7;", "W"),
  "pctl": ("localparam int W = 3;", "", "W"),
  "fnpar":("function automatic int f(input int a); return 3; endfunction", "localparam int f = 7;", "f(2)"),
}
TD = {
  "td":   ("typedef logic [3:0] T;", "typedef logic [7:0] T;"),
  "tctl": ("typedef logic [3:0] T;", ""),
}
def lane_text(lane, A, X):
    a = (A + " ") if A else ""
    return {
      "ret":  f"function {a}logic [{X}:0] h(input int x); return x; endfunction",
      "fml":  f"function {a}int h(input logic [{X}:0] x); return x; endfunction",
      "loc":  f"function {a}int h(input int x); logic [{X}:0] t; t = x; return t; endfunction",
      "cast": f"function {a}int h(input int x); return {X}'(x); endfunction",
      "rep":  f"function {a}logic [31:0] h(input int x); return {{{X}{{1'b1}}}}; endfunction",
      "psel": f"function {a}logic [31:0] h(input logic [15:0] x); return x[{X}:0]; endfunction",
      "dflt": f"function {a}int h(input int x, input int k = {X}); return x + k; endfunction",
      "frloc":f"function {a}int h(input int x);\n    logic [{X}:0] t;\n    t = 0;\n    for (int i = 0; i < x; i++) t = t + 1;\n    return t;\n  endfunction",
    }[lane]
def td_lane_text(lane, A):
    a = (A + " ") if A else ""
    return {
      "ret":  f"function {a}T h(input int x); return x; endfunction",
      "fml":  f"function {a}int h(input T x); return x; endfunction",
      "loc":  f"function {a}int h(input int x); T t; t = x; return t; endfunction",
      "cast": f"function {a}int h(input int x); return T'(x); endfunction",
    }[lane]
ARG = {"ret":"1000","fml":"1000","loc":"1000","cast":"1000","rep":"0","psel":"16'hABCD","dflt":"1000","frloc":"n"}
HEX = {"rep","psel"}
def use(lane, mode, call):
    arg = ARG[lane]
    if mode == "rt":
        if lane in HEX:
            return f"  logic [31:0] v;\n  initial begin v = {call}({arg}); $display(\"v=%h\", v); #1 $finish; end"
        if lane == "frloc":
            return f"  int v, n = 1000;\n  initial begin v = {call}(n); $display(\"v=%0d\", v); #1 $finish; end"
        return f"  int v;\n  initial begin v = {call}({arg}); $display(\"v=%0d\", v); #1 $finish; end"
    else:
        if lane == "frloc": arg = "1000"
        if lane in HEX:
            return f"  localparam logic [31:0] P = {call}({arg});\n  initial begin #1 $display(\"P=%h\", P); $finish; end"
        return f"  localparam int P = {call}({arg});\n  initial begin #1 $display(\"P=%0d\", P); $finish; end"
def spell(sp):
    return {"q": ("", "q::h"), "imp": ("  import q::h;\n", "h"), "wc": ("  import q::*;\n", "h")}[sp]
def write(name, pkg_decl, routine, caller, body, imp=""):
    src = f"package q;\n  {pkg_decl}\n  {routine}\nendpackage\nmodule top;\n{imp}"
    if caller: src += f"  {caller}\n"
    src += body + "\nendmodule\n"
    open(os.path.join(OUT, name + ".sv"), "w").write(src)
cells = []
LANES = ["ret","fml","loc","cast","rep","psel","dflt","frloc"]
# core: automatic, scoped, kinds fn/ctl/par/pctl/fnpar
for lane, mode, kind in itertools.product(LANES, ["rt","ce"], ["fn","ctl","par","pctl","fnpar"]):
    pd, cd, X = KINDS[kind]
    imp, call = spell("q")
    n = f"a_{lane}_{mode}_{kind}"
    write(n, pd, lane_text(lane, "automatic", X), cd, use(lane, mode, call), imp); cells.append(n)
# static
for lane, mode, kind in itertools.product(LANES, ["rt","ce"], ["fn","ctl","par"]):
    pd, cd, X = KINDS[kind]
    imp, call = spell("q")
    n = f"s_{lane}_{mode}_{kind}"
    write(n, pd, lane_text(lane, "", X), cd, use(lane, mode, call), imp); cells.append(n)
# import spellings
for lane, mode, kind, sp in itertools.product(["ret","loc","dflt","cast"], ["rt","ce"], ["fn","ctl","par"], ["imp","wc"]):
    pd, cd, X = KINDS[kind]
    imp, call = spell(sp)
    n = f"i{sp}_{lane}_{mode}_{kind}"
    write(n, pd, lane_text(lane, "automatic", X), cd, use(lane, mode, call), imp); cells.append(n)
# typedef
for lane, mode, kind in itertools.product(["ret","fml","loc","cast"], ["rt","ce"], ["td","tctl"]):
    pd, cd = TD[kind]
    imp, call = spell("q")
    n = f"t_{lane}_{mode}_{kind}"
    write(n, pd, td_lane_text(lane, "automatic"), cd, use(lane, mode, call), imp); cells.append(n)
# $bits(q::h(0))
for kind, sp in itertools.product(["fn","ctl","par","pctl"], ["q","imp"]):
    pd, cd, X = KINDS[kind]
    imp, call = spell(sp)
    n = f"w_{kind}_{sp}"
    body = f"  initial begin #1 $display(\"B=%0d\", $bits({call}(0))); $finish; end"
    write(n, pd, lane_text("ret", "automatic", X), cd, body, imp); cells.append(n)
for kind in ["td","tctl"]:
    pd, cd = TD[kind]
    n = f"w_{kind}_q"
    body = f"  initial begin #1 $display(\"B=%0d\", $bits(q::h(0))); $finish; end"
    write(n, pd, td_lane_text("ret", "automatic"), cd, body, ""); cells.append(n)
print(len(cells))
