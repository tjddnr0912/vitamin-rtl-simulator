import os, itertools
exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)),'gen.py')).read().split("cells = []")[0])
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "c4")
def write4(name, pkg_decl, routine, caller, body, imp=""):
    src = f"package q;\n  {pkg_decl}\n  {routine}\nendpackage\nmodule top;\n{imp}"
    if caller: src += f"  {caller}\n"
    src += body + "\nendmodule\n"
    open(os.path.join(OUT, name + ".sv"), "w").write(src)
n=0
# inline lane: static, explicit import / wildcard, bare call
for lane, mode, kind, sp in itertools.product(["ret","fml","loc","cast","rep","psel","dflt"], ["rt"], ["fn","ctl","par","pctl"], ["imp","wc"]):
    pd, cd, X = KINDS[kind]; imp, call = spell(sp)
    write4(f"s{sp}_{lane}_{mode}_{kind}", pd, lane_text(lane, "", X), cd, use(lane, mode, call), imp); n+=1
# compound constant in body positions
RT2 = {
 "rep2": "function automatic logic [31:0] h(input int x); return {(W*2){1'b1}}; endfunction",
 "psel2":"function automatic logic [31:0] h(input logic [15:0] x); return x[(W*2):0]; endfunction",
 "cast2":"function automatic int h(input int x); return (W*2)'(x); endfunction",
 "bits2":"function automatic int h(input int x); logic [W:0] t; return $bits(t) + W*0; endfunction",
}
for lane, kind in itertools.product(RT2, ["par","pctl"]):
    pd, cd, X = KINDS[kind]
    for mode in ["rt","ce"]:
        l2 = {"rep2":"rep","psel2":"psel","cast2":"cast","bits2":"ret"}[lane]
        write4(f"b_{lane}_{mode}_{kind}", pd, RT2[lane], cd, use(l2, mode, "q::h"), ""); n+=1
print(n)
