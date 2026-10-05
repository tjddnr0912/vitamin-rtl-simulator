module top;
  typedef logic [63:0] t64u;
  localparam R = (t64u'(-64'sd4) ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
