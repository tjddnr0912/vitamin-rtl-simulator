module top;
  localparam byte unsigned PBU = 8'hFC;
  localparam R = (PBU ==? 4'sb1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
