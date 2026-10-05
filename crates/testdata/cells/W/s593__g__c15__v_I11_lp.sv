module top;

  localparam R = (-4'sd4 ==? 8'sb1111_1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
