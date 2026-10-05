module top;

  localparam R = ((64'hFFFF_FFFF_FFFF_FFFF + 64'd13) ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
