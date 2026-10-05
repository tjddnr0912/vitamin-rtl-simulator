import os
D = os.path.dirname(os.path.abspath(__file__)) + '/c'
H = "  localparam logic signed [7:0] X = -4;\n"
ARR = ("  localparam logic [7:0] A [2] = '{8'hFC, 8'h03};\n"
       "  localparam logic signed [7:0] AS [2] = '{-8'sd4, 8'sd3};\n")
WD = "  initial #10 $finish;\n"
T = "((({N{1'b0}} + 8'd255 + 8'd1) >> 1) == 128)"   # 1 iff the region is >= 9 bits (N >= 9)
cells = {}
def m(name, body, pre="", post=""):
    cells[name] = pre + "module t;\n" + body + WD + "endmodule\n" + post
# --- count resolution: shadow / channel axes (true N = 16 => 1; misread 4 => 0) ---
m("D01_fnlp_shadow", "  localparam N = 4;\n  function automatic int f(input int a);\n    localparam N = 16;\n    f = " + T + ";\n  endfunction\n  localparam L = f(0);\n  initial $display(\"R: L=%0d\", L);\n")
m("D02_fnlp_arr_shadow", "  localparam logic [3:0] A [2] = '{4'd1, 4'd2};\n  function automatic int f(input int a);\n    localparam logic [15:0] A [2] = '{16'd0, 16'd2};\n    f = ((((A[0] + 8'd255 + 8'd1) >> 1)) == 128);\n  endfunction\n  localparam L = f(0);\n  initial $display(\"R: L=%0d\", L);\n")
m("D03_gen_shadow", "  localparam N = 4;\n  if (1) begin : G\n    localparam N = 16;\n    localparam L = " + T + ";\n    initial $display(\"R: L=%0d\", L);\n  end\n")
m("D04_genvar_count", "  for (genvar g = 4; g <= 16; g += 12) begin : G\n    localparam L = ((({g{1'b0}} + 8'd255 + 8'd1) >> 1) == 128);\n    initial $display(\"R: g=%0d L=%0d\", g, L);\n  end\n")
cells["D05_defparam"] = ("module sub #(parameter N = 4) ();\n  localparam L = " + T + ";\n  initial $display(\"R: N=%0d L=%0d\", N, L);\nendmodule\n"
  "module t;\n  sub u();\n  defparam u.N = 16;\n" + WD + "endmodule\n")
cells["D06_lpchain_ovr"] = ("module sub #(parameter M = 2) ();\n  localparam N = M * 2;\n  localparam L = " + T + ";\n  initial $display(\"R: M=%0d L=%0d\", M, L);\nendmodule\n"
  "module t;\n  sub #(.M(8)) u1();\n" + WD + "endmodule\n")
cells["D07_ovr_arr_elem"] = ("module sub #(parameter logic signed [7:0] AS [2] = '{8'sd1, 8'sd2}) ();\n  localparam L = AS[0] + {4{1'b0}};\n  localparam K = (AS[0] < 8'sd0);\n  initial $display(\"R: L=%0d B=%0d K=%0d\", L, $bits(L), K);\nendmodule\n"
  "module t;\n  sub #(.AS('{-8'sd4, 8'sd3})) u();\n" + WD + "endmodule\n")
m("D08a_pkg_count", "  import p::*;\n  localparam L1 = ((({p::N{1'b0}} + 8'd255 + 8'd1) >> 1) == 128);\n  localparam L2 = " + T + ";\n  initial $display(\"R: L1=%0d L2=%0d\", L1, L2);\n", pre="package p;\n  localparam N = 16;\nendpackage\n")
m("D08b_pkg_elem", "  localparam L3 = p::AS[0] + {4{1'b0}};\n  localparam K3 = (p::AS[0] < 8'sd0);\n  initial $display(\"R: L3=%0d B3=%0d K3=%0d\", L3, $bits(L3), K3);\n", pre="package p;\n  localparam logic signed [7:0] AS [2] = '{-8'sd4, 8'sd3};\nendpackage\n")
m("D09_count_spellings", H + "  localparam N = 16;\n  localparam L1 = ((({$clog2(65536){1'b0}} + 8'd255 + 8'd1) >> 1) == 128);\n  localparam L2 = {$bits(X){1'b1}} + 8'd1;\n  localparam L3 = ((({N-12{1'b0}} + 8'd255 + 8'd1) >> 1) == 128);\n  localparam L4 = {N/4{1'b1}} + 4'd1;\n  initial $display(\"R: L1=%0d L2=%0d B2=%0d L3=%0d L4=%0d B4=%0d\", L1, L2, $bits(L2), L3, L4, $bits(L4));\n")
# --- newly sized leaves inside other operators ---
m("D10a_signed_rep", "  localparam N = 4;\n  localparam L1 = $signed({N{1'b1}}) + 8'sd0;\n  localparam L2 = ($signed({N{1'b1}}) < 8'sd0);\n  localparam L3 = $unsigned($signed({N{1'b1}})) + 8'd0;\n  initial $display(\"R: L1=%0d B1=%0d L2=%0d L3=%0d\", L1, $bits(L1), L2, L3);\n")
m("D10b_signed_elem", ARR + "  localparam L2 = $unsigned(AS[0]) + 16'sd0;\n  localparam L3 = $signed(A[0]) + 16'sd0;\n  localparam L4 = (AS[0] + 16'sd0);\n  initial $display(\"R: L2=%0d L3=%0d L4=%0d B4=%0d\", L2, L3, L4, $bits(L4));\n")
m("D11_cast", H + "  localparam N = 4;\n  localparam L1 = 16'(X + {N{1'b0}});\n  localparam L2 = signed'({N{1'b1}}) + 8'sd0;\n  localparam L3 = 3'({N{1'b1}}) + 8'd0;\n  initial $display(\"R: L1=%0d B1=%0d L2=%0d L3=%0d\", L1, $bits(L1), L2, L3);\n")
m("D12_concat", H + "  localparam N = 4;\n  localparam L1 = {X + {N{1'b0}}};\n  localparam L2 = {X, {N{1'b0}}};\n  localparam L3 = {{N{1'b1}} + 4'd1, X};\n  initial $display(\"R: L1=%0d B1=%0d L2=%0d B2=%0d L3=%0d B3=%0d\", L1, $bits(L1), L2, $bits(L2), L3, $bits(L3));\n")
m("D13_nested_rep", "  localparam N = 4;\n  localparam L1 = {2{{N{1'b1}}}} + 8'd1;\n  localparam L2 = {N{{2{1'b1}}}} + 8'd1;\n  localparam L3 = {N{{N{1'b1}}}} + 16'd1;\n  initial $display(\"R: L1=%0d B1=%0d L2=%0d L3=%0d B3=%0d\", L1, $bits(L1), L2, L3, $bits(L3));\n")
m("D14_shift_pow", H + "  localparam N = 4;\n  localparam L1 = {N{1'b1}} << 4;\n  localparam L2 = X >>> {N{1'b0}};\n  localparam L3 = {N{1'b1}} ** 2;\n  localparam L4 = (X + {N{1'b0}}) >>> 2;\n  localparam L5 = 2 ** {N{1'b1}};\n  initial $display(\"R: L1=%0d B1=%0d L2=%0d L3=%0d L4=%0d L5=%0d\", L1, $bits(L1), L2, L3, L4, L5);\n")
m("D15_ternary_sign", ARR + "  localparam N = 4;\n  localparam L1 = 1'b1 ? AS[0] : {N{1'b0}};\n  localparam L2 = ((1'b1 ? AS[0] : AS[1]) < 0);\n  localparam L3 = ((1'b1 ? AS[0] : A[1]) < 0);\n  initial $display(\"R: L1=%0d B1=%0d L2=%0d L3=%0d\", L1, $bits(L1), L2, L3);\n")
m("D16_2d_elem", "  localparam N = 4;\n  localparam logic signed [7:0] A2 [2][2] = '{'{-8'sd4, 8'sd1}, '{8'sd2, 8'sd3}};\n  localparam L1 = A2[0][0] + {N{1'b0}};\n  localparam L2 = (A2[0][0] < 8'sd0);\n  localparam L3 = A2[1][1] + 8'd0;\n  initial $display(\"R: L1=%0d B1=%0d L2=%0d L3=%0d\", L1, $bits(L1), L2, L3);\n")
m("D17_packed_arr", "  localparam N = 4;\n  localparam logic [1:0][7:0] PA = {8'hFC, 8'h03};\n  localparam L1 = PA[1] + {N{1'b0}};\n  localparam L2 = ($signed(PA[1]) < 0);\n  localparam L3 = PA[1] + 8'sd0;\n  initial $display(\"R: L1=%0d B1=%0d L2=%0d L3=%0d\", L1, $bits(L1), L2, L3);\n")
m("D18_string_arr", "  localparam string SA [2] = '{\"ab\", \"cd\"};\n  localparam L = (SA[1] == \"cd\");\n  initial $display(\"R: L=%0d s=%s\", L, SA[0]);\n")
m("D19_real_arr", "  localparam N = 4;\n  localparam real RA [2] = '{1.5, -2.5};\n  localparam real L1 = RA[1] + 1;\n  localparam L2 = RA[0] + {N{1'b0}};\n  localparam L3 = (RA[1] < 0);\n  initial $display(\"R: L1=%0.2f L2=%0.2f L3=%0d\", L1, L2, L3);\n")
m("D20_oob_elem", "  localparam N = 4;\n  localparam logic [7:0] A [2] = '{8'hFC, 8'h03};\n  localparam int IA [2] = '{-4, 3};\n  localparam L1 = A[3] + {N{1'b0}};\n  localparam L2 = IA[2] + {N{1'b0}};\n  initial $display(\"R: L1=%b L2=%0d B2=%0d\", L1, L2, $bits(L2));\n")
# --- consumers ---
m("D21_gen_if_case", H + "  localparam N = 4;\n  if ((X + {N{1'b0}}) == 8'hFC) begin : GA initial $display(\"R: GI=then\"); end\n  else begin : GB initial $display(\"R: GI=else\"); end\n  case (X + {N{1'b0}})\n    8'hFC: begin : CA initial $display(\"R: GC=fc\"); end\n    default: begin : CD initial $display(\"R: GC=def\"); end\n  endcase\n")
m("D22_gen_for", H + "  localparam N = 4;\n  for (genvar i = 0; i < (X + {N{1'b0}}) / 63; i++) begin : G\n    initial $display(\"R: i=%0d\", i);\n  end\n")
m("D23a_bits_range", H + "  localparam N = 4;\n  localparam B1 = $bits({N{1'b1}});\n  logic [(X + {N{1'b0}}) - 250 : 0] v;\n  initial $display(\"R: B1=%0d vb=%0d\", B1, $bits(v));\n")
m("D23b_elem_range", ARR + "  localparam B1 = $bits(AS[0]);\n  logic [AS[1] + {4{1'b0}} : 0] w;\n  logic [(AS[0] + 8'd0) - 250 : 0] u;\n  initial $display(\"R: B1=%0d wb=%0d ub=%0d\", B1, $bits(w), $bits(u));\n")
m("D24_case_rt", H + "  localparam N = 4;\n  logic [7:0] r;\n  initial begin\n    r = 8'hFC;\n    case (r)\n      (X + {N{1'b0}}): $display(\"R: C=hit\");\n      default: $display(\"R: C=miss\");\n    endcase\n  end\n")
m("D25_inside_wild", H + "  localparam N = 4;\n  localparam L1 = (X + {N{1'b0}}) inside {8'hFC};\n  localparam L2 = ((X + {N{1'b0}}) ==? 8'b1111_11xx);\n  localparam L3 = ((X + {N{1'b0}}) inside {[8'd250:8'd255]});\n  initial $display(\"R: L1=%0d L2=%0d L3=%0d\", L1, L2, L3);\n")
m("D26_wide", H + "  localparam N = 70;\n  localparam L1 = X + {N{1'b0}};\n  localparam L2 = {N{1'b1}} + 1'b1;\n  localparam logic [7:0] L3 = (X + {N{1'b0}}) >> 2;\n  initial $display(\"R: L1=%0d B1=%0d L2=%0d B2=%0d L3=%0d\", L1, $bits(L1), L2, $bits(L2), L3);\n")
m("D27_xz", H + "  localparam N = 4;\n  localparam L1 = 8'bx + {N{1'b0}};\n  localparam L2 = X + {N{1'bz}};\n  localparam L3 = ({N{1'bx}} == 4'd0);\n  initial $display(\"R: L1=%b L2=%b L3=%b\", L1, L2, L3);\n")
m("D28a_runtime", H + "  localparam N = 4;\n  logic [31:0] r32;\n  initial begin\n    r32 = X + {N{1'b0}};\n    $display(\"R: r32=%0d d=%0d\", r32, X + {N{1'b0}});\n  end\n")
m("D28b_runtime_elem", ARR + "  localparam N = 4;\n  logic [31:0] r32;\n  initial begin\n    r32 = AS[0] + {N{1'b0}};\n    $display(\"R: r32=%0d d=%0d\", r32, AS[0] + {N{1'b0}});\n  end\n")
m("D29_call_in_arm", H + "  localparam N = 4;\n  function automatic logic [7:0] f(input int a); f = a; endfunction\n  localparam L1 = ((1'b1 ? f(252) : 8'd0) == (X + {N{1'b0}}));\n  localparam L2 = ((1'b0 ? f(0) : (X + {N{1'b0}})) >> 2);\n  initial $display(\"R: L1=%0d L2=%0d\", L1, L2);\n")
m("D30_fnvar_arm", "  localparam N = 4;\n  function automatic int g(input int a);\n    logic signed [7:0] v;\n    v = -4;\n    g = (((a != 0) ? v : 8'sd0) + {N{1'b0}}) == 8'hFC;\n  endfunction\n  localparam L = g(1);\n  initial $display(\"R: L=%0d\", L);\n")
m("D31_rep_partsel", "  localparam N = 4;\n  localparam logic [15:0] P = 16'hABCD;\n  localparam L1 = P[{N{1'b1}} : 0];\n  localparam L2 = P[$bits({N{1'b1}}) + 3 -: 4];\n  initial $display(\"R: L1=%h B1=%0d L2=%h\", L1, $bits(L1), L2);\n")
m("D33_zero_count", H + "  localparam Z = 0;\n  localparam L1 = {X, {Z{1'b1}}};\n  localparam L2 = {X, {0{1'b1}}};\n  initial $display(\"R: L1=%0d B1=%0d L2=%0d B2=%0d\", L1, $bits(L1), L2, $bits(L2));\n")
m("D34_genidx_elem", "  localparam logic signed [7:0] AS [2] = '{-8'sd4, 8'sd3};\n  for (genvar g = 0; g < 2; g++) begin : G\n    localparam L = AS[g] + {4{1'b0}};\n    initial $display(\"R: g=%0d L=%0d B=%0d\", g, L, $bits(L));\n  end\n")
cells["D35_iface_count"] = ("interface ifc #(parameter N = 4);\n  localparam L = " + T + ";\nendinterface\n"
  "module t;\n  ifc #(.N(16)) i();\n  initial $display(\"R: L=%0d\", i.L);\n" + WD + "endmodule\n")
os.makedirs(D, exist_ok=True)
for k, v in cells.items():
    open(f'{D}/{k}.sv', 'w').write(v)
print(len(cells))
