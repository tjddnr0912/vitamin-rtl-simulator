module top;
  localparam logic signed [64:0] S65P = 65'sd12;
  localparam R = (S65P ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
