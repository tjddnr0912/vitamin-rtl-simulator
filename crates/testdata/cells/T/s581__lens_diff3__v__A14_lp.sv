module top;
  localparam logic [39:0] P40 = 40'h10_0000_000C;
  localparam R = ($countones(P40) ==? 4'b0?11);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
