`timescale 1ns/1ns
module t;
  localparam int I = 12;
  wire [7:0] s = 8'hA5;
  wire [7:0] ps = s[0 +: (I inside {4'b1?00, 4'b0011})+3];
  initial #1 $display("ps=%b", ps);
  initial #5 $finish;
endmodule
