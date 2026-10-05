module top;
  localparam longint PLS = -64'sd4;
  localparam R = (PLS ==? 4'sb1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
