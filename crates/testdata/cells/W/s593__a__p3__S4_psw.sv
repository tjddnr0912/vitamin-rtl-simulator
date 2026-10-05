`timescale 1ns/1ns
module t;
  localparam logic [7:0] U = 8'h0C;
  wire [7:0] s = 8'hA5;
  wire [7:0] ps = s[0 +: (U ==? (4'b1?00))+3];
  initial #1 $display("ps=%b", ps);
  initial #5 $finish;
endmodule
