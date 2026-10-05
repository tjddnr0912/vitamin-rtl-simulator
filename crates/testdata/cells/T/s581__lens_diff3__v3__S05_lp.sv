module top;
  localparam integer unsigned PIU = 32'hFFFF_FFFC;
  localparam R = (PIU ==? 4'sb1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
