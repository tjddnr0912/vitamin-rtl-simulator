import os
D=os.path.join(os.path.dirname(os.path.abspath(__file__)),'e')
# each cell: (id, two_state_ok, module_items, initial_body_lines)
H="`timescale 1ns/1ns\nmodule t;\n"
T="  initial #100 $finish;\nendmodule\n"
cells=[
# unsized based
("E01",1,"logic [3:0] v;",["v=4'b1100; $display(\"E01 %b\", v inside {'b1?00});"]),
("E02",1,"logic [3:0] v;",["v=4'b0011; $display(\"E02 %b\", v inside {'bx1});"]),
("E03",1,"logic [3:0] v;",["v=4'b0010; $display(\"E03 %b\", v inside {'bx1});"]),
("E04",1,"logic [3:0] v;",["v=4'b1100; $display(\"E04 %b\", v inside {'b?});"]),
("E05",1,"logic [35:0] v;",["v=36'hF_0000_0001; $display(\"E05 %b\", v inside {'bx1});"]),
("E06",1,"logic [35:0] v;",["v=36'hF_0000_0001; $display(\"E06 %b\", v inside {'b1x1});"]),
("E07",1,"logic [3:0] v;",["v=4'b1100; $display(\"E07 %b\", v inside {'h?});"]),
# fills
("E10",1,"logic [3:0] v;",["v=4'b1010; $display(\"E10 %b\", v inside {'x});"]),
("E11",1,"logic [3:0] v;",["v=4'b1010; $display(\"E11 %b\", v inside {'z});"]),
("E12",1,"logic [3:0] v;",["v=4'b1111; $display(\"E12 %b\", v inside {'1});"]),
("E13",1,"logic [3:0] v;",["v=4'b0111; $display(\"E13 %b\", v inside {'1});"]),
("E14",1,"logic [3:0] v;",["v=4'b0000; $display(\"E14 %b\", v inside {'0});"]),
("E15",1,"logic [3:0] v;",["v=4'b0000; $display(\"E15 %b\", v inside {4'b0001, 'x});"]),
# parameters
("E20",1,"logic [3:0] v; localparam L = 4'b1?00;",["v=4'b1100; $display(\"E20 %b\", v inside {L});"]),
("E21",1,"logic [3:0] v; localparam logic [3:0] L = 4'b1?00;",["v=4'b1100; $display(\"E21 %b\", v inside {L});"]),
("E22",1,"logic [3:0] v; parameter P = 4'b1x00;",["v=4'b1100; $display(\"E22 %b\", v inside {P});"]),
("E23",1,"logic [3:0] v; localparam [3:0] L = 4'b1z00;",["v=4'b1100; $display(\"E23 %b\", v inside {L});"]),
("E24",1,"logic [3:0] v; localparam logic [3:0] L = 'x;",["v=4'b1100; $display(\"E24 %b\", v inside {L});"]),
("E25",1,"logic [3:0] v; localparam L = 4'b1100;",["v=4'b1100; $display(\"E25 %b\", v inside {L});"]),
("E26",1,"logic [3:0] v; localparam L = 4'b1?00;",["v=4'b1100; $display(\"E26 %b\", v ==? L);"]),
# enum label
("E30",1,"logic [3:0] v; typedef enum logic [3:0] {A = 4'b1x00, B = 4'b0001} e_t;",["v=4'b1100; $display(\"E30 %b\", v inside {A});"]),
("E31",1,"logic [3:0] v; typedef enum logic [3:0] {A = 4'b1100, B = 4'b0001} e_t;",["v=4'b1100; $display(\"E31 %b\", v inside {A, B});"]),
# compound const
("E40",1,"logic [3:0] v;",["v=4'b1100; $display(\"E40 %b\", v inside {{2'b1?, 2'b00}});"]),
("E41",1,"logic [3:0] v; localparam logic [3:0] P = 4'b1100;",["v=4'b1101; $display(\"E41 %b\", v inside {P | 4'b000x});"]),
("E42",1,"logic [3:0] v;",["v=4'b1100; $display(\"E42 %b\", v inside {(4'b1?00)});"]),
("E43",1,"logic [3:0] v;",["v=4'b1100; $display(\"E43 %b\", v inside {4'(4'b1?00)});"]),
("E44",1,"logic [3:0] v;",["v=4'b1100; $display(\"E44 %b\", v inside {1'b1 ? 4'b1?00 : 4'b0000});"]),
("E45",1,"logic [3:0] v;",["v=4'b1100; $display(\"E45 %b\", v inside {{2{2'b?0}}});"]),
("E46",1,"logic [3:0] v;",["v=4'b1100; $display(\"E46 %b\", v inside {~4'b0?11});"]),
("E47",1,"logic [3:0] v;",["v=4'b1100; $display(\"E47 %b\", v inside {$unsigned(4'b1?00)});"]),
# constant function
("E50",1,"logic [3:0] v; function automatic logic [3:0] cf(); return 4'b1?00; endfunction",["v=4'b1100; $display(\"E50 %b\", v inside {cf()});"]),
# strings / reals
("E60",1,"logic [15:0] s16;",["s16=\"ab\"; $display(\"E60 %b\", s16 inside {\"ab\"});"]),
("E61",1,"string s;",["s=\"ab\"; $display(\"E61 %b\", s inside {\"ab\", \"cd\"});"]),
("E62",1,"real r;",["r=1.5; $display(\"E62 %b\", r inside {1.5});"]),
("E63",1,"real r;",["r=1.5; $display(\"E63 %b\", r inside {[1.0:2.0]});"]),
("E64",1,"logic [3:0] v;",["v=4'b0010; $display(\"E64 %b\", v inside {2.0});"]),
("E65",1,"real r;",["r=12.0; $display(\"E65 %b\", r inside {4'b1?00});"]),
# array / struct
("E70",0,"logic [3:0] v; logic [3:0] arr [2];",["arr[0]=4'b0000; arr[1]=4'b1100; v=4'b1100; $display(\"E70 %b\", v inside {arr});"]),
("E71",1,"logic [3:0] v; typedef struct packed {logic [3:0] a; logic [3:0] b;} st; st s;",["s=8'hC1; v=4'b1100; $display(\"E71 %b\", v inside {s.a});"]),
("E72",1,"typedef struct packed {logic [3:0] a; logic [3:0] b;} st; st s;",["s=8'hC1; $display(\"E72 %b\", s.a inside {4'b1?00});"]),
("E73",1,"logic [3:0] arr [2];",["arr[1]=4'b1000; $display(\"E73 %b\", arr[1] inside {4'b1?00});"]),
("E74",1,"bit [3:0] bv;",["bv=4'b1100; $display(\"E74 %b\", bv inside {4'b1?00});"]),
("E75",1,"int iv;",["iv=12; $display(\"E75 %b\", iv inside {4'b1?00});"]),
("E76",1,"logic [3:0] v; bit b;",["v=4'b1100; b = v inside {4'b1?00}; $display(\"E76 %b\", b);"]),
("E77",1,"logic [3:0] v; logic [7:0] w8;",["v=4'b1100; w8 = v inside {4'b1?00}; $display(\"E77 %b\", w8);"]),
("E78",1,"logic [3:0] v;",["v=4'b1100; $display(\"E78 %b\", v[3:0] inside {4'b1?00});"]),
("E79",1,"logic [3:0] v;",["v=4'b1100; $display(\"E79 %b\", {v[3:2], v[1:0]} inside {4'b1?00});"]),
("E80",1,"logic [3:0] v;",["v=4'b1100; $display(\"E80 %b\", (v + 4'd0) inside {4'b1?00});"]),
("E81",1,"logic [3:0] v; logic [3:0] e;",["v=4'b1100; e=4'b1100; $display(\"E81 %b\", v inside {e, 4'b0?00});"]),
("E82",1,"logic [3:0] v; logic [1:0] p;",["v=4'b1100; p=2'b11; $display(\"E82 %b\", v inside {{p, 2'b?0}});"]),
("E83",1,"logic [3:0] v;",["v=4'b1100; $display(\"E83 %b\", v inside {4'sb1?00});"]),
("E84",1,"logic signed [3:0] v;",["v=4'sb1100; $display(\"E84 %b\", v inside {4'b1?00});"]),
("E85",1,"logic [3:0] v;",["v=4'b1100; $display(\"E85 %b\", $unsigned(v) inside {4'b1?00});"]),
("E86",1,"logic [7:0] v;",["v=8'hF4; $display(\"E86 %b\", $signed(v[3:0]) inside {8'sb1111?100});"]),
("E87",1,"logic [7:0] v;",["v=8'hF4; $display(\"E87 %b\", $signed(v[3:0]) inside {4'sb?100});"]),
]
for cid,ts,items,body in cells:
    s=H+"  "+items+"\n  initial begin\n"+"".join("    "+l+"\n" for l in body)+"    #1 $finish;\n  end\n"+T
    open(os.path.join(D,cid+".sv"),"w").write(s)
    if not ts: open(os.path.join(D,cid+".nov"),"w").write("")
print(len(cells))
