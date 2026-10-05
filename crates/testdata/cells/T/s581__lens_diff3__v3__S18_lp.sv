module top;
  localparam longint unsigned PLU = 64'hFFFF_FFFF_FFFF_FFFC;
  localparam R = (PLU ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
