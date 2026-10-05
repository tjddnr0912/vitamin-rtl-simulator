# Census generator: writes one .sv per group; cells = (id, setup, expr, two_state_ok)
import os
D=os.path.dirname(os.path.abspath(__file__))
def b(n,s): return f"{n}'b{s}"
groups={}
# ---- A: 4-bit unsigned LHS, literal elements
A=[
("A01","v=4'b1100;","v inside {4'b1100}",1),
("A02","v=4'b1100;","v inside {4'b1?00}",1),
("A03","v=4'b1000;","v inside {4'b1?00}",1),
("A04","v=4'b0100;","v inside {4'b1?00}",1),
("A05","v=4'b1100;","v inside {4'b0000, 4'b1?00}",1),
("A06","v=4'b0101;","v inside {[4'd1:4'd7]}",1),
("A07","v=4'b1100;","v inside {4'b1x00}",1),
("A08","v=4'b1100;","v inside {4'b1z00}",1),
("A09","v=4'b0100;","v inside {4'b?100}",1),
("A10","v=4'b1100;","v inside {4'bx100}",1),
("A11","v=4'b0100;","v inside {4'bz100}",1),
("A12","v=4'b1101;","v inside {4'b110?}",1),
("A13","v=4'b1010;","v inside {4'b????}",1),
("A14","v=4'b1010;","v inside {4'bxxxx}",1),
("A15","v=4'b0101;","v inside {4'b1?00, [4'd1:4'd7]}",1),
("A16","v=4'b1000;","v inside {[4'd1:4'd7], 4'b1?00}",1),
("A17","v=4'b0011;","v inside {[4'd1:4'd2], 4'b1?00}",1),
("A18","v=4'b1100;","!(v inside {4'b1?00})",1),
("A19","v=4'b0100;","!(v inside {4'b1?00})",1),
("A20","v=4'b1100;","v inside {3'b1?0}",1),
("A21","v=4'b0110;","v inside {3'b1?0}",1),
("A22","v=4'b1100;","v inside {6'b001?00}",1),
("A23","v=4'b1100;","v inside {6'b101?00}",1),
("A24","v=4'b1100;","v inside {6'b??1?00}",1),
("A25","v=4'b0110;","v inside {4'b0000, 4'b0001, 4'b01?0}",1),
("A26","v=4'b0110;","v inside {4'b01?0, 4'b0000}",1),
("A27","v=4'b0111;","v inside {4'b01?0, 4'b0000}",1),
("A28","v=4'b1100;","v inside {4'hC}",1),
("A29","v=4'b1100;","v inside {4'h?}",1),
("A30","v=4'b1100;","v inside {4'hx}",1),
("A31","v=4'b1100;","v inside {4'dx}",1),
("A32","v=4'b1100;","v inside {4'o1?}",1),
("A33","v=4'b0100;","v inside {4'b1?00} || v inside {4'b0?00}",1),
("A34","v=4'b0100;","(v inside {4'b1?00}) + 2'd1",1),
("A35","v=4'b1100;","{v inside {4'b1?00}, v inside {4'b0?00}}",1),
("A36","v=4'b1100;","4'b1100 inside {4'b1?00}",1),
("A37","v=4'b1100;","4'b0100 inside {4'b1?00}",1),
]
groups['A']=("logic [3:0] v;",A)
# ---- W: LHS width ladder; element same width, wild at MSB and LSB
W=[]
for w in [8,16,32,33,64,65,128]:
    pat='?'+'1'*(w-2)+'?'     # wild MSB and LSB, ones else
    val0='0'+'1'*(w-2)+'0'
    val1='1'+'1'*(w-2)+'1'
    valn='1'+'1'*(w-3)+'01'   # bit1 differs -> 0
    W+= [ (f"W{w}a", f"u{w}={w}'b{val0};", f"u{w} inside {{{w}'b{pat}}}",1),
          (f"W{w}b", f"u{w}={w}'b{val1};", f"u{w} inside {{{w}'b{pat}}}",1),
          (f"W{w}c", f"u{w}={w}'b{valn};", f"u{w} inside {{{w}'b{pat}}}",1),
          (f"W{w}d", f"u{w}={w}'b{val1};", f"u{w} inside {{4'b1?11}}",1),   # narrower element: zero-ext => high bits compared to 0 -> 0
          (f"W{w}e", f"u{w}={w}'b1011;", f"u{w} inside {{4'b1?11}}",1),       # 1
        ]
groups['W']=(" ".join(f"logic [{w-1}:0] u{w};" for w in [8,16,32,33,64,65,128]),W)
# ---- S: sign axis
S=[
("S01","s8=-8'sd4;","s8 inside {4'sb1?00}",1),
("S02","s8=-8'sd4;","s8 inside {4'sb?100}",1),
("S03","s8=8'sb01010100;","s8 inside {4'sb?100}",1),
("S04","s8=-8'sd4;","s8 inside {4'b1?00}",1),
("S05","s8=-8'sd4;","s8 inside {8'sb11111?00}",1),
("S06","u8=8'b11111100;","u8 inside {4'sb1?00}",1),
("S07","s8=-8'sd4;","s8 inside {-8'sd4}",1),
("S08","s8=-8'sd4;","s8 inside {4'sb1100}",1),
("S09","s4=-4'sd4;","s4 inside {8'sb11111?00}",1),
("S10","s4=-4'sd4;","s4 inside {8'b11111?00}",1),
("S11","s4=-4'sd4;","s4 inside {8'sb00001?00}",1),
("S12","s8=8'sb00001100;","s8 inside {4'sb1?00}",1),
("S13","s8=8'sb00001100;","s8 inside {4'b1?00}",1),
("S14","i32=-4;","i32 inside {4'sb1?00}",1),
("S15","i32=-4;","i32 inside {32'sb1111111111111111111111111111?100}",1),
("S16","s8=-8'sd4;","s8 inside {4'sbx100}",1),
("S17","s8=8'sb01010100;","s8 inside {4'sbz100}",1),
("S18","s8=-8'sd4;","s8 inside {4'sb1100, 4'sb1?00}",1),
("S19","s4=-4'sd4;","s4 inside {8'sb?1111100}",1),
("S20","s4=4'sb0100;","s4 inside {8'sb?0000100}",1),
("S21","s8=-8'sd4;","s8 inside {[-8'sd5:-8'sd3]}",1),
("S22","s8=-8'sd4;","s8 inside {[-8'sd5:-8'sd3], 4'sb0?00}",1),
]
groups['S']=("logic signed [7:0] s8; logic [7:0] u8; logic signed [3:0] s4; integer i32;",S)
# ---- X: LHS x/z (no verilator)
X=[
("X01","v=4'b1x00;","v inside {4'b1?00}",0),
("X02","v=4'bx100;","v inside {4'b1?00}",0),
("X03","v=4'bz100;","v inside {4'b1?00}",0),
("X04","v=4'b1z00;","v inside {4'b1?00}",0),
("X05","v=4'bx100;","v inside {4'b0000, 4'b1?00}",0),
("X06","v=4'bx100;","v inside {4'b?100, 4'b1?00}",0),
("X07","v=4'bx100;","v inside {4'b?100}",0),
("X08","v=4'bxxxx;","v inside {4'b????}",0),
("X09","v=4'bx100;","v inside {4'b0100}",0),
("X10","v=4'bx100;","v inside {[4'd1:4'd7]}",0),
("X11","v=4'b1x00;","v inside {4'b1x00}",0),
("X12","v=4'bx100;","v inside {4'b0?00}",0),
("X13","v=4'bx110;","v inside {4'b0?00}",0),
("X14","v=4'bx100;","v inside {4'b1?00, 4'b0110}",0),
("X15","v=4'bx100;","!(v inside {4'b1?00})",0),
("X16","v=4'bzzzz;","v inside {4'bzzzz}",0),
("X17","v=4'bzzzz;","v inside {4'b0000}",0),
("X18","v=4'bx100;","v inside {6'b??1?00}",0),
("X19","v=4'bx100;","v inside {6'b011?00}",0),
("X20","v=4'b1100;","v inside {4'b1?00, 4'bx}",0),
]
groups['X']=("logic [3:0] v;",X)
# ---- V: variable elements (runtime x/z)
V=[
("V01","v=4'b1100; e=4'b1x00;","v inside {e}",0),
("V02","v=4'b1100; e=4'b1100;","v inside {e}",1),
("V03","v=4'b1100; be=4'b1100;","v inside {be}",1),
("V04","v=4'b1000; e=4'b1z00;","v inside {e}",0),
("V05","v=4'b1x00; e=4'b1100;","v inside {e}",0),
("V06","v=4'b1100; e=4'b1x00;","v ==? e",0),
("V07","v=4'b0100; e=4'b1x00;","v inside {e}",0),
("V08","v=4'b1100; e=4'b1x00;","v inside {4'b0000, e}",0),
("V09","v=4'b1100; e=4'b0000;","v inside {e, 4'b1?00}",1),
("V10","v=4'b1100; e=4'bxxxx;","v inside {e}",0),
]
groups['V']=("logic [3:0] v, e; bit [3:0] be;",V)

def write(name, decls, cells, two_state_only):
    lines=["`timescale 1ns/1ns","module t;","  "+decls,"  initial begin"]
    for (cid,setup,expr,ts) in cells:
        if two_state_only and not ts: continue
        lines.append(f"    {setup} $display(\"{cid} %b\", {expr});")
    lines += ["    #1 $finish;","  end","  initial #100 $finish;","endmodule",""]
    open(os.path.join(D,name),"w").write("\n".join(lines))
for g,(decls,cells) in groups.items():
    write(f"{g}.sv",decls,cells,False)
    if any(c[3] for c in cells):
        write(f"{g}_2s.sv",decls,cells,True)
print("ok")
