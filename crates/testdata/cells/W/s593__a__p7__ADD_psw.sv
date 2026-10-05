`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] SA = -8'sd4;
  wire [7:0] s = 8'hA5;
  wire [7:0] ps = s[0 +: ((SA + 8'sd0) ==? 4'sb1?00)+3];
  initial #1 $display("ps=%b", ps);
  initial #5 $finish;
endmodule
