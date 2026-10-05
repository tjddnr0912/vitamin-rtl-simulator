import os
D = os.path.dirname(os.path.abspath(__file__)) + '/cells'
T = 'initial #5 $finish;'
cells = {
'F6_samecount_vals': f"""module top;
  localparam A = 1;
  if (1) begin : b
    for (genvar i = A; i < A + 2; i = i + 1) begin : g
      initial #1 $display("@%m");
    end
    localparam A = 2;
  end
  {T}
endmodule
""",
'F15_zero_nets_iter': f"""module top;
  localparam N = 0;
  if (1) begin : b
    for (genvar i = 0; i < N; i = i + 1) begin : g
      initial #1 $display("@%m");
    end
    localparam N = 2;
  end
  {T}
endmodule
""",
'F3i_iarr_two_mismatch': f"""module sub;
  localparam K = 4;
  if (1) begin : gb
    if (K == 4) begin : x initial #1 $display("@%m x"); end else begin : y initial #1 $display("@%m y"); end
    localparam K = 8;
  end
endmodule
module top;
  sub u[1:0]();
  {T}
endmodule
""",
}
for k, v in cells.items():
    open(f'{D}/{k}.sv', 'w').write(v)
