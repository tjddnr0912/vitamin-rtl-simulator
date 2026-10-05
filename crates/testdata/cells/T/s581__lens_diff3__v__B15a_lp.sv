module top;
  localparam logic [39:0] P40 = 40'h10_0000_000C;
  typedef logic [39:0] t40;
  localparam R = (t40'(P40) ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
