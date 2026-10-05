module top;
  localparam logic [39:0] P40 = 40'h10_0000_000C;
  localparam R = ($bits(P40) ==? 6'sb10_1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
