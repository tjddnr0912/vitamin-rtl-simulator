#!/usr/bin/env python3
# Stage-2 consumer x form matrix and binder cells. Each cell prints "<ID> <value>".
import os, json
D = os.path.dirname(os.path.abspath(__file__)) + '/cells'
cells = {}
FORMS = {  # name: (expression giving 1 per both oracles, x/z-free control twin)
 'Q': "(4'b1100 ==? 4'b1?00)",
 'I': "(4'b1100 inside {4'b1?00})",
 'C': "(4'b1100 == 4'b1100)",          # control: no wildcard
 'W': "((4'd15 + 4'd1) ==? 5'b1?000)",  # common-width left operand (1 per oracles)
 'X': "(4'bx100 ==? 4'b1?00)",          # x-valued
 'N': "((4'd15 + 4'd1) inside {5'b0?000})",  # L4: 0 per oracles
}
def cell(cid, body, pre=''):
    cells[cid] = f"`timescale 1ns/1ns\n{pre}module t;\n{body}\nendmodule\n"
for f, e in FORMS.items():
    cell(f'LP_{f}', f"  localparam L = {e};\n  initial #1 $display(\"LP_{f} %b %0d\", L, $bits(L));")
    cell(f'LT_{f}', f"  localparam logic [3:0] L = {e};\n  initial #1 $display(\"LT_{f} %b\", L);")
    cell(f'LW_{f}', f"  localparam [64:0] L = {{64'd0, {e}}};\n  initial #1 $display(\"LW_{f} %h\", L);")
    cell(f'RB_{f}', f"  logic [{e} : 0] v;\n  initial #1 $display(\"RB_{f} %0d\", $bits(v));")
    cell(f'AD_{f}', f"  logic a [{e} : 0];\n  initial #1 $display(\"AD_{f} %0d\", $size(a));")
    cell(f'RP_{f}', f"  logic [7:0] r;\n  initial begin r = {{({e} + 1){{1'b1}}}}; #1 $display(\"RP_{f} %b\", r); end")
    cell(f'PS_{f}', f"  logic [7:0] v8 = 8'hA5;\n  initial #1 $display(\"PS_{f} %b\", v8[{e}*3+1:0]);")
    cell(f'GI_{f}', f"  if ({e}) begin : g initial #1 $display(\"GI_{f} then\"); end else begin : h initial #1 $display(\"GI_{f} else\"); end")
    cell(f'GC_{f}', f"  case (1) {e}: begin : g initial #1 $display(\"GC_{f} item\"); end default: begin : h initial #1 $display(\"GC_{f} default\"); end endcase")
    cell(f'GS_{f}', f"  case ({e}) 1'b1: begin : g initial #1 $display(\"GS_{f} one\"); end default: begin : h initial #1 $display(\"GS_{f} default\"); end endcase")
    cell(f'GF_{f}', f"  for (genvar i = 0; i <= {e}; i++) begin : g initial #1 $display(\"GF_{f} %0d\", i); end")
    cell(f'PC_{f}', f"  initial begin #1 case (1'b1) {e}: $display(\"PC_{f} item\"); default: $display(\"PC_{f} default\"); endcase end")
    cell(f'BI_{f}', f"  initial #1 $display(\"BI_{f} %0d\", $bits(logic [{e}:0]));")
    cell(f'EN_{f}', f"  typedef enum logic [3:0] {{EA = {e}, EB}} e_t;\n  initial #1 $display(\"EN_{f} %0d %0d\", EA, EB);")
    cell(f'OV_{f}', f"  m #(.P({e})) u();", pre="module m #(parameter P = 0) (); initial #1 $display(\"OV_P %b %0d\", P, $bits(P)); endmodule\n")
    cell(f'OT_{f}', f"  m #(.P({e})) u();", pre="module m #(parameter logic [64:0] P = 0) (); initial #1 $display(\"OT_P %h\", P); endmodule\n")
    cell(f'CF_{f}', f"  function automatic int f(int a); return a + {e}; endfunction\n  localparam L = f(2);\n  initial #1 $display(\"CF_{f} %0d\", L);")
    cell(f'PK_{f}', f"  localparam L = pk::K;\n  initial #1 $display(\"PK_{f} %b\", L);", pre=f"package pk; localparam K = {e}; endpackage\n")
    cell(f'SC_{f}', f"  localparam logic [7:0] L = 8'({e});\n  initial #1 $display(\"SC_{f} %b\", L);")
    cell(f'IU_{f}', f"  initial #1 $display(\"IU_{f} %b\", $isunknown({e}));\n  localparam L = $isunknown({e});\n  initial #1 $display(\"IUc_{f} %b\", L);")
    cell(f'CL_{f}', f"  localparam L = $clog2({e} + 4);\n  initial #1 $display(\"CL_{f} %0d\", L);")
    # binder cells: a newly bound constant read by the generate-case label lane and scrutinee
    cell(f'BL_{f}', f"  localparam L = {e};\n  case (1) L: begin : g initial #1 $display(\"BL_{f} item\"); end default: begin : h initial #1 $display(\"BL_{f} default\"); end endcase")
    cell(f'BW_{f}', f"  localparam [64:0] L = {{64'd0, {e}}};\n  case (1) L: begin : g initial #1 $display(\"BW_{f} item\"); end default: begin : h initial #1 $display(\"BW_{f} default\"); end endcase")
    cell(f'BS_{f}', f"  localparam L = {e};\n  case (L) 1'b1: begin : g initial #1 $display(\"BS_{f} one\"); end default: begin : h initial #1 $display(\"BS_{f} default\"); end endcase")
    cell(f'BP_{f}', f"  case (1) pk::K: begin : g initial #1 $display(\"BP_{f} item\"); end default: begin : h initial #1 $display(\"BP_{f} default\"); end endcase", pre=f"package pk; localparam K = {e}; endpackage\n")
    cell(f'BG_{f}', f"  if (1) begin : b\n    localparam [64:0] L = {{64'd0, {e}}};\n    case (1) L: begin : g initial #1 $display(\"BG_{f} item\"); end default: begin : h initial #1 $display(\"BG_{f} default\"); end endcase\n  end")
    cell(f'BO_{f}', f"  m #(.P({e})) u();", pre=f"module m #(parameter P = 0) (); case (1) P: begin : g initial #1 $display(\"BO_P item\"); end default: begin : h initial #1 $display(\"BO_P default\"); end endcase endmodule\n")
    cell(f'BOW_{f}', f"  m #(.P({{64'd0, {e}}})) u();", pre=f"module m #(parameter logic [64:0] P = 0) (); case (1) P: begin : g initial #1 $display(\"BOW_P item\"); end default: begin : h initial #1 $display(\"BOW_P default\"); end endcase endmodule\n")
# signed-region constant twin (round-2 blocking) and no-width left operand
cell('SR1', "  localparam logic signed [3:0] S4 = -8;\n  localparam logic signed [7:0] S8 = 8'sd0;\n  localparam L = (S4 + S8) ==? 4'sb1?00;\n  initial #1 $display(\"SR1 %b\", L);")
cell('SR2', "  localparam signed [7:0] S = 8'sd84;\n  localparam L = S ==? 4'sb?100;\n  initial #1 $display(\"SR2 %b\", L);")
cell('NW1', "  localparam L = t.u.v ==? 4'b1?00;\n  initial #1 $display(\"NW1 %b\", L);")
cell('NW2', "  string s = \"ab\";\n  initial #1 $display(\"NW2 %b\", s ==? 'x);")
cell('XD1', "  logic a [4'bx : 0];\n  initial #1 $display(\"XD1 %0d\", $size(a));")
cell('FW1', "  sub u();\n  initial #1 $display(\"FW1 %b\", u.v36 ==? 'x);", pre="module sub; logic [35:0] v36 = '0; endmodule\n")
os.makedirs(D, exist_ok=True)
for k, v in cells.items():
    open(f'{D}/{k}.sv', 'w').write(v.replace('module t;', 'module t;') )
json.dump(sorted(cells), open(D + '/list.json', 'w'))
print(len(cells))
