module top;
  typedef logic signed [63:0] t64s;
  localparam R = (t64s'(64'hFFFF_FFFF_FFFF_FFFC) ==? 4'sb1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
