module top;
  localparam logic [39:0] P40 = 40'h10_0000_000C;
  localparam R = (P40[36] ==? 2'b?1);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
