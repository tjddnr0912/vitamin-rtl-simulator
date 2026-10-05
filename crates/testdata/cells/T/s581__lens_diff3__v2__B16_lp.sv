module top;
  localparam logic [39:0] P40 = 40'h10_0000_000C;
  localparam int W = 40;
  localparam R = (P40[W-1:0] ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
