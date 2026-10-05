module top;
  localparam bit signed [63:0] PBS = -64'sd4;
  localparam R = (PBS ==? 4'sb1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
