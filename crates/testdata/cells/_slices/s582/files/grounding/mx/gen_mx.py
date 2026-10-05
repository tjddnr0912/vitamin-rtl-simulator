import os, json, itertools
OUT = os.path.dirname(os.path.abspath(__file__))
# E types: (decl, expr, list of (assign, label))
def vals4(): return [f"4'b{i:04b}" for i in range(16)]
E = {
 "E1u4": ("logic [3:0] e;", "e", [(f"e = {v}", v) for v in vals4()]),
 "E2s4": ("logic signed [3:0] e;", "e", [(f"e = {v}", v) for v in vals4()]),
 "E3s8": ("logic signed [7:0] e;", "e", [(f"e = 8'h{x}", x) for x in ["00","0E","0F","FE","FF","80","7F","FA","F6","06","0A","1E","EE","E6","FC","0C"]]),
 "E4u8": ("logic [7:0] e;", "e", [(f"e = 8'h{x}", x) for x in ["00","0E","0F","FE","FF","80","7F","FA","F6","06","0A","1E","EE","E6","FC","0C"]]),
 "E5opu8": ("logic [7:0] a, b;", "a + b", [(f"a = 8'h{x}; b = 8'h{y}", x+"+"+y) for x,y in [("80","80"),("FF","FF"),("7F","7F"),("00","00"),("FF","01"),("FE","00"),("80","7F"),("00","FE")]]),
 "E6ops4": ("logic signed [3:0] a, b;", "a + b", [(f"a = 4'sh{x}; b = 4'sh{y}", x+"+"+y) for x,y in [("7","7"),("F","F"),("8","8"),("0","0"),("7","1"),("E","0"),("F","E"),("1","1")]]),
}
I = {
 "I01_m1": "-1", "I02_s4m2": "-4'sd2", "I03_u8FE": "8'hFE", "I04_rs32": "[-2:-1]", "I05_rs4": "[-4'sd3:-4'sd1]",
 "I06_ru4": "[4'd14:4'd15]", "I07_ws4": "4'sb1?10", "I08_wu4": "4'b1?10", "I09_fill1": "'1", "I10_u16": "16'h0100", "I11_s8": "8'sh80",
 "I12_u8FC": "8'hFC", "I13_ru8": "[8'hFC:8'hFE]",
}
SIB = {"S0": None, "S1u8": "8'h55", "S2s16": "16'sh7777", "S3u32": "32'd12345"}
def ci_case(sel, item, sib, k):
    arms = f"{item}: m{k} = 1; " + (f"{sib}: m{k} = 2; " if sib else "")
    return f"case ({sel}) inside {arms}default: m{k} = 0; endcase"
def if_case(sel, item, sib, k):
    s = f"if ({sel} inside {{{item}}}) m{k} = 1; "
    if sib: s += f"else if ({sel} inside {{{sib}}}) m{k} = 2; "
    return s + f"else m{k} = 0;"
cells = []
for en,(decl,sel,vals) in E.items():
    for iname,item in I.items():
        name = f"{en}__{iname}"
        for sp, fn in (("ci", ci_case), ("if", if_case)):
            lines = ["module top;", f"  {decl} int m0, m1, m2, m3;", "  initial begin"]
            for assign, lab in vals:
                lines.append(f"    {assign};")
                for k,(sn,sib) in enumerate(SIB.items()):
                    lines.append("    " + fn(sel, item, sib, k))
                lines.append(f'    $display("{lab} %0d %0d %0d %0d", m0, m1, m2, m3);')
            lines += ["    #10 $finish;", "  end", "endmodule", ""]
            open(os.path.join(OUT, f"{name}_{sp}.sv"), "w").write("\n".join(lines))
        cells.append(name)
json.dump({"E": {k: [v[1], [l for _,l in v[2]]] for k,v in E.items()}, "I": I, "SIB": SIB, "cells": cells}, open(os.path.join(OUT, "spec.json"), "w"))
print(len(cells))
