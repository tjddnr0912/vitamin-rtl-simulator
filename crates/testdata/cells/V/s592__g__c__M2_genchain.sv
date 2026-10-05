module top;
  if (1) begin : gb
    localparam A = B;
    localparam B = 3;
    initial #1 $display("@A=%0d B=%0d", A, B);
  end
  initial #5 $finish;
endmodule
