`timescale 1ns/1ns
module t;
  localparam logic [63:0] B2 = 64'h1_0000_000C;
  wire [7:0] r = {(($signed(B2) ==? 4'sb1?00)+1){4'b1010}};
  initial #1 $display("r=%b", r);
  initial #5 $finish;
endmodule
