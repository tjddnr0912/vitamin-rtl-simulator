module top;
  localparam logic [39:0] P40 = 40'h10_0000_000C;
  localparam int I36 = 36;
  localparam R = (P40[I36] ==? 2'b?1);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
