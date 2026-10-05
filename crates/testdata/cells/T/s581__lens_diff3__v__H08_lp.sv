module top;
  localparam logic signed [64:0] S65N = -65'sd4;
  localparam R = (S65N ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
