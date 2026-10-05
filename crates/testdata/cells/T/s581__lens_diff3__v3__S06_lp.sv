module top;
  typedef int unsigned u32_t;
  localparam u32_t PUT = 32'hFFFF_FFFC;
  localparam R = (PUT ==? 4'sb1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
