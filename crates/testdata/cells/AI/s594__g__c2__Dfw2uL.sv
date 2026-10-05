`timescale 1ns/1ns
module t;
  localparam U = '1 ^ {2{1'b0}};
  initial #1 $display("U=%h B=%0d", U, $bits(U));
  initial #20 $finish;
endmodule
