module top;
  typedef enum logic [39:0] {E0 = 40'h10_0000_000C, E1 = 40'h0C} et40;
  localparam R = (E1 ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
