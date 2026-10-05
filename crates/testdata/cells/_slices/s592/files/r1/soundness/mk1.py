import os
D = os.path.dirname(os.path.abspath(__file__)) + '/cells'
T = 'initial #5 $finish;'
arm = lambda lbl, v, wd: f"begin : {lbl} wire [{wd-1}:0] w = {wd}'d{v}; initial #1 $display(\"@%m w=%0d bits=%0d\", w, $bits(w)); end"
cells = {}
cells['K1_mac2inv'] = f"""`define GIF(c, t, e) if (c) {arm('t',1,4).replace(': t ',': t ')} else {arm('e',200,8)}
module top;
  localparam P = 1;
  `GIF(P == 1, a1, a2)
  `GIF(P == 2, b1, b2)
  {T}
endmodule
"""
# macro arg labels: substitute t/e by arg names
cells['K1_mac2inv'] = cells['K1_mac2inv'].replace("begin : t ", "begin : t ").replace("begin : e ", "begin : e ")
cells['K2_mac1inv2c'] = f"""`define G2(c1, c2) if (c1) {arm('a1',1,4)} else {arm('a2',200,8)} if (c2) {arm('b1',2,4)} else {arm('b2',201,8)}
module top;
  localparam P = 1;
  `G2(P == 1, P == 2)
  {T}
endmodule
"""
cells['K3_macfor2c'] = f"""`define F2(n1, n2) for (genvar i = 0; i < n1; i++) begin : f1 initial #1 $display("@%m"); end for (genvar j = 0; j < n2; j++) begin : f2 initial #1 $display("@%m"); end
module top;
  `F2(2, 3)
  {T}
endmodule
"""
cells['K4_maccase2c'] = f"""`define C2(s1, s2) case (s1) 1: {arm('c1',1,4)} default: {arm('c1d',200,8)} endcase case (s2) 1: {arm('c2',2,4)} default: {arm('c2d',201,8)} endcase
module top;
  `C2(1, 2)
  {T}
endmodule
"""
cells['K5_inc2'] = f"""module top;
`define V 1
`include "inc_k5.svh"
`undef V
`define V 2
`include "inc_k5.svh"
  {T}
endmodule
"""
open(D + '/inc_k5.svh', 'w').write("if (`V == 1) begin wire [3:0] w = 4'd1; initial #1 $display(\"@%m then w=%0d\", w); end else begin wire [7:0] w = 8'd200; initial #1 $display(\"@%m else w=%0d\", w); end\n")
cells['K7_mac2inv_for'] = f"""`define GF(lbl, n) for (genvar i = 0; i < n; i++) begin : lbl initial #1 $display("@%m"); end
module top;
  `GF(f1, 2)
  `GF(f2, 3)
  {T}
endmodule
"""
def initA(inner):
    return f"""module top;
  localparam A = 1;
  if (1) begin : b
    for (genvar i = A; i < 3; i = i + 1) begin : g
      wire [3:0] w = i;
      initial #1 $display("@%m w=%0d", w);
    end
    {inner}
  end
  {T}
endmodule
"""
cells['U1_initA_ltype'] = initA('localparam type A = logic [7:0];')
cells['U2_initA_arr'] = initA("localparam int A [2] = '{5, 6};")
cells['U3_initA_str'] = initA('localparam string A = "ab";')
cells['U4_initA_net'] = initA("wire [3:0] A = 4'd9;")
cells['U6_initA_var'] = initA('int A = 9;')
cells['U7_initA_real'] = initA('localparam real A = 2.5;')
cells['U8_initA_wide'] = initA("localparam [99:0] A = 100'd2;")
cells['U13_if_ltype'] = f"""module top;
  localparam A = 1;
  if (1) begin : b
    if (A == 1) {arm('x',1,4)} else {arm('y',200,8)}
    localparam type A = logic [7:0];
  end
  {T}
endmodule
"""
cells['H1_ifcparam'] = f"""interface ifc #(parameter W = 8); logic [W-1:0] d; endinterface
module top;
  ifc #(.W(16)) bus();
  if (bus.W == 16) {arm('a',1,4)} else {arm('b',200,8)}
  {T}
endmodule
"""
cells['H2_childparam'] = f"""module sub #(parameter W = 8); endmodule
module top;
  sub #(.W(16)) u();
  if (u.W == 16) {arm('a',1,4)} else {arm('b',200,8)}
  {T}
endmodule
"""
cells['H3_childparam_gen'] = f"""module sub #(parameter W = 8); endmodule
module top;
  if (1) begin : gb
    sub #(.W(16)) u();
    if (u.W == 16) {arm('a',1,4)} else {arm('b',200,8)}
  end
  {T}
endmodule
"""
cells['H4_defparam'] = f"""module sub #(parameter W = 8); endmodule
module top;
  sub u();
  defparam u.W = 16;
  if (u.W == 16) {arm('a',1,4)} else {arm('b',200,8)}
  {T}
endmodule
"""
cells['H5_ifc_in_gen'] = f"""interface ifc #(parameter W = 8); logic [W-1:0] d; endinterface
module top;
  if (1) begin : gb
    ifc #(.W(16)) bus();
    if (bus.W == 16) {arm('a',1,4)} else {arm('b',200,8)}
  end
  {T}
endmodule
"""
cells['FN1_gfn_noouter'] = f"""module top;
  if (1) begin : gb
    case (8'd99)
      f(98): {arm('g',200,8)}
      default: {arm('g',9,4)}
    endcase
    function automatic integer f(input integer a); f = a + 1; endfunction
  end
  {T}
endmodule
"""
cells['FN2_gfn_before'] = f"""module top;
  if (1) begin : gb
    function automatic integer f(input integer a); f = a + 1; endfunction
    if (f(98) == 99) {arm('x',1,4)} else {arm('y',200,8)}
  end
  {T}
endmodule
"""
cells['FN3_mfn_after'] = f"""module top;
  if (1) begin : gb
    if (f(98) == 99) {arm('x',1,4)} else {arm('y',200,8)}
  end
  function automatic integer f(input integer a); f = a + 1; endfunction
  {T}
endmodule
"""
cells['R1_step_bodyparam'] = f"""module top;
  localparam S = 2;
  for (genvar i = 0; i < 4; i = i + S) begin : g
    localparam S = 1;
    initial #1 $display("@%m");
  end
  {T}
endmodule
"""
cells['EI1_elseif'] = f"""module top;
  localparam P = 2;
  if (P == 1) {arm('a',1,4)} else if (P == 2) {arm('b',2,4)} else if (P == 3) {arm('c',3,4)} else {arm('d',200,8)}
  {T}
endmodule
"""
cells['B1_bind_fwd'] = f"""module chk;
  localparam K = 4;
  if (1) begin : gb
    case (8'd4)
      K: {arm('g',1,4)}
      default: {arm('g',200,8)}
    endcase
    localparam K = 8;
  end
endmodule
module top;
  wire x;
  {T}
endmodule
bind top chk c();
"""
for k, v in cells.items():
    open(f'{D}/{k}.sv', 'w').write(v)
print(len(cells))
