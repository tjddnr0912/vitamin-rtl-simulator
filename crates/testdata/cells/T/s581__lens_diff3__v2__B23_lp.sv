module top;
  localparam logic [1:0][63:0] PQ = {64'hC, 64'h8000_0000_0000_000C};
  localparam R = (PQ[0] ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
