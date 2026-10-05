module top;
  localparam [99:0] Q = 100'h1_0000_0000_0000_0002;
  localparam P = Q;
  for (genvar i = P; i < 3; i = i + 1) begin : g
    initial #1 $display("@%m");
  end
  initial #1 $display("@P=%0d Q=%0h", P, Q);
  initial #5 $finish;
endmodule
