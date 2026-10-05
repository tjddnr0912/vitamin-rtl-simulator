module top;
  localparam logic signed [63:0] S64N = -64'sd4;
  localparam R = (S64N ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
