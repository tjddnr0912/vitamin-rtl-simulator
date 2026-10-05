module top;
  typedef logic signed [39:0] t40s;
  localparam R = (t40s'(40'hFF_FFFF_FFFC) ==? 4'sb1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
