`timescale 1ns/1ns
module t;
  localparam logic [31:0] R = {(2'b11 + 2'd2){4'hA}};
  initial #1 $display("R=%h", R);
  initial #20 $finish;
endmodule
