`timescale 1ns/1ns
module t;
  localparam logic [7:0] X = 8'hFC;
  localparam bit C = 1;
  typedef logic [7:0] u8_t;
  initial #1 $display("RT=%0d", ((X | u8_t'(8'd2)) == 8'hFE));
  initial #5 $finish;
endmodule
