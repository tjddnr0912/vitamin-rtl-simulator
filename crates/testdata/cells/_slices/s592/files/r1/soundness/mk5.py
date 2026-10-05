import os
D = os.path.dirname(os.path.abspath(__file__)) + '/cells'
T = 'initial #5 $finish;'
cells = {
'W65_back_chain': f"""module top;
  localparam [99:0] Q = 100'h1_0000_0000_0000_0002;
  localparam P = Q;
  for (genvar i = P; i < 3; i = i + 1) begin : g
    initial #1 $display("@%m");
  end
  initial #1 $display("@P=%0d Q=%0h", P, Q);
  {T}
endmodule
""",
'MSG1_blame_innocent': f"""module top;
  localparam A = 1;
  localparam B = 2;
  if (1) begin : b
    if (A == 1 && B == 2) begin : x initial #1 $display("@%m x"); end else begin : y initial #1 $display("@%m y"); end
    localparam A = 1;
    localparam B = 3;
  end
  {T}
endmodule
""",
}
for k, v in cells.items():
    open(f'{D}/{k}.sv', 'w').write(v)
