module top;
  localparam logic signed [63:0] S64P = 64'sd12;
  localparam R = (S64P ==? 4'sb1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
