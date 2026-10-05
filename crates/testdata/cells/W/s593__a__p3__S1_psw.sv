`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] SN = -8'sd60;
  wire [7:0] s = 8'hA5;
  wire [7:0] ps = s[0 +: (SN ==? 4'b?100)+3];
  initial #1 $display("ps=%b", ps);
  initial #5 $finish;
endmodule
