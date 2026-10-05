module top;
  typedef enum logic signed [3:0] {ES = -4'sd4, ET = 4'sd3} est4;
  localparam R = (ES ==? 8'b0000_1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
