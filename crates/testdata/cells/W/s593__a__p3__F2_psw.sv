`timescale 1ns/1ns
module t;
  localparam logic [63:0] B2 = 64'h1_0000_000C;
  wire [7:0] s = 8'hA5;
  wire [7:0] ps = s[0 +: ($signed(B2) ==? 4'sb1?00)+3];
  initial #1 $display("ps=%b", ps);
  initial #5 $finish;
endmodule
