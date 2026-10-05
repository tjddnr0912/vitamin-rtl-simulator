module top;
  localparam int unsigned PU = 32'hFFFF_FFFC;
  localparam R = (PU ==? 4'sb1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
