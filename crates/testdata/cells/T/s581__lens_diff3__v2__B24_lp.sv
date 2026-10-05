module top;
  localparam int IA [2] = '{12, 3};
  localparam R = (IA[0] ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
