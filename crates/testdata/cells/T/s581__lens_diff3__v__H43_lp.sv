module top;
  localparam logic [63:0] P64 = 64'hC;
  localparam R = (P64 inside {4'b1?00, 65'b0?11});
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
