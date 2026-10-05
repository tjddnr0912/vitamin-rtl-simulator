module top;

  localparam R = (65'sd12 ==? 4'sb1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
